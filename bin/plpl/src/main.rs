#[macro_use]
extern crate lazy_static;

use ephemeris::*;
use log::*;
use server_lib::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;
use time_series::{Candle, Day, Month, Time};
use tokio::io::Result;

// Binance US API endpoint
// Data returned in ascending order, oldest first
// Timestamps are in milliseconds
#[allow(dead_code)]
const BINANCE_API: &str = "https://api.binance.us";

// Binance Spot Test Network API credentials
const BINANCE_TEST_API: &str = "https://testnet.binance.vision";
const BINANCE_TEST_API_KEY: &str =
    "hrCcYjjRCW6jCCOVGiOOXve1UVLK8jbYd08WyKQjuUI63VNmcuR0EDBtDsrW9KBJ";
const BINANCE_TEST_API_SECRET: &str =
    "XGKu8AelLejzC6R5ZBWvbNzy4NC7d78ckU0sOJk3VeFRsWnJTajCfcFsArnPFEjP";

lazy_static! {
    static ref ACCOUNT: Mutex<Account> = Mutex::new(Account {
        client: Client::new(
            Some(BINANCE_TEST_API_KEY.to_string()),
            Some(BINANCE_TEST_API_SECRET.to_string()),
            BINANCE_TEST_API.to_string()
        ),
        recv_window: 5000,
        base_asset: "BTC".to_string(),
        quote_asset: "BUSD".to_string(),
        ticker: "BTCBUSD".to_string(),
        active_order: None,
        quote_asset_free: None,
        quote_asset_locked: None,
        base_asset_free: None,
        base_asset_locked: None,
    });
    static ref USER_STREAM: Mutex<UserStream> = Mutex::new(UserStream {
        client: Client::new(
            Some(BINANCE_TEST_API_KEY.to_string()),
            Some(BINANCE_TEST_API_SECRET.to_string()),
            BINANCE_TEST_API.to_string()
        ),
        recv_window: 10000,
    });
}

pub fn init_logger(log_file: &PathBuf) {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            SimpleLogConfig::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            ConfigBuilder::new()
                .set_time_format_custom(simplelog::format_description!(
                    "[hour]:[minute]:[second].[subsecond]"
                ))
                .build(),
            File::create(log_file).expect("Failed to create star_stream log file"),
        ),
    ])
    .expect("Failed to initialize logger");
}

fn kline_to_candle(kline_event: &KlineEvent) -> Candle {
    let date = Time::from_unix_msec(kline_event.event_time as i64);
    Candle {
        date,
        open: kline_event
            .kline
            .open
            .parse::<f64>()
            .expect("Failed to parse Kline open to f64"),
        high: kline_event
            .kline
            .high
            .parse::<f64>()
            .expect("Failed to parse Kline high to f64"),
        low: kline_event
            .kline
            .low
            .parse::<f64>()
            .expect("Failed to parse Kline low to f64"),
        close: kline_event
            .kline
            .close
            .parse::<f64>()
            .expect("Failed to parse Kline close to f64"),
        volume: None,
    }
}

fn free_asset(account_info: &AccountInfoResponse, asset: &str) -> f64 {
    account_info
        .balances
        .iter()
        .find(|&x| x.asset == asset)
        .unwrap()
        .free
        .parse::<f64>()
        .unwrap()
}

fn locked_asset(account_info: &AccountInfoResponse, asset: &str) -> f64 {
    account_info
        .balances
        .iter()
        .find(|&x| x.asset == asset)
        .unwrap()
        .locked
        .parse::<f64>()
        .unwrap()
}

fn total_balance(account_info: &AccountInfoResponse, quote_asset: &str, base_asset: &str) -> f64 {
    let free_quote = free_asset(account_info, quote_asset);
    let locked_quote = locked_asset(account_info, quote_asset);
    let free_base = free_asset(account_info, base_asset);
    let locked_base = locked_asset(account_info, base_asset);
    let total_balance = ((free_quote + locked_quote) / 20_000.0) + free_base + locked_base;
    total_balance
}

#[tokio::main]
async fn main() -> Result<()> {
    let log_file = std::env::var("LOG_FILE").unwrap_or("plpl.log".to_string());
    init_logger(&PathBuf::from(log_file));

    info!("Starting Binance PLPL!");
    let config = Config::testnet();
    let keep_running = AtomicBool::new(true);

    let prev_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let curr_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let mut account = ACCOUNT.lock().expect("Failed to lock account");

    // PLPL parameters; tuned for 5 minute candles
    let trailing_stop = 0.95;
    let stop_loss_pct = 0.001;
    let planet = Planet::from("Jupiter");
    let plpl_scale = 0.5;
    let plpl_price = 20000.0;
    let num_plpls = 2000;
    let cross_margin_pct = 55.0;

    // initialize PLPL
    let plpl_system = PLPLSystem::new(PLPLSystemConfig {
        planet,
        origin: Origin::Heliocentric,
        first_date: Time::new(2023, &Month::from_num(6), &Day::from_num(1), None, None),
        last_date: Time::new(2050, &Month::from_num(6), &Day::from_num(1), None, None),
        plpl_scale,
        plpl_price,
        num_plpls,
        cross_margin_pct,
    });
    let plpl_system = match plpl_system {
        Err(e) => {
            error!("Failed to initialize PLPL system: {}", e);
            return Ok(());
        }
        Ok(plpl_system) => plpl_system,
    };

    // atomic counter to attempt trade every 2 candles
    let update_counter = AtomicUsize::new(0);

    // Kline Websocket
    let mut ws = WebSockets::new(|event: WebSocketEvent| {
        if let WebSocketEvent::Kline(kline_event) = event {
            let start = SystemTime::now();

            let count = update_counter.fetch_add(1, Ordering::SeqCst);
            debug!("Atomic counter: {}", count);
            if !kline_event.kline.is_final_bar {
                return Ok(());
            }

            let date = Time::from_unix_msec(kline_event.event_time as i64);
            // cache previous and current candle to assess PLPL trade conditions
            let mut prev = prev_candle.lock().expect("Failed to lock previous candle");
            let mut curr = curr_candle.lock().expect("Failed to lock current candle");
            // cast Kline to Candle
            let candle = kline_to_candle(&kline_event);
            info!("Current price: {}", candle.close);
            // compute closest PLPL to current Candle
            let plpl = plpl_system
                .closest_plpl(&candle)
                .expect("Failed to get closest plpl");

            match (&*prev, &*curr) {
                (None, None) => *prev = Some(candle),
                (Some(prev_candle), None) => {
                    *curr = Some(candle.clone());
                    if plpl_system.long_signal(prev_candle, &candle, plpl) {
                        // if position is Long, ignore
                        // if position is Short, close short and open Long
                        // if position is None, enter Long
                        match account.get_active_order() {
                            None => {
                                info!("No active order, enter Long");
                                match account.cancel_all_active_orders() {
                                    Err(_) => {
                                        info!("No active orders to cancel")
                                    }
                                    Ok(_) => {
                                        info!("All active orders canceled");
                                    }
                                }

                                let account_info = match account.account_info() {
                                    Err(e) => {
                                        error!("Failed to get account info: {}", e);
                                        return Ok(());
                                    }
                                    Ok(account_info) => account_info,
                                };
                                let busd_balance = free_asset(&account_info, &account.quote_asset);
                                let busd_balance_locked =
                                    locked_asset(&account_info, &account.quote_asset);
                                let btc_balance = free_asset(&account_info, &account.base_asset);
                                let btc_balance_locked =
                                    locked_asset(&account_info, &account.base_asset);
                                info!(
                                    "BUSD: locked: {}, free: {}\tBTC: locked: {}, free: {}",
                                    busd_balance_locked,
                                    busd_balance,
                                    btc_balance_locked,
                                    btc_balance
                                );
                                let total_balances = total_balance(
                                    &account_info,
                                    &account.quote_asset,
                                    &account.base_asset,
                                );
                                info!("Total account balance: {}", total_balances);
                                // calculate quantity of base asset to trade
                                // Trade with $1000 or as close as the account can get
                                let long_qty: f64 = if btc_balance * candle.close < 1000.0 {
                                    btc_balance
                                } else {
                                    BinanceTrade::round_quantity(1000.0 / candle.close)
                                };

                                let trade = plpl_long(
                                    account.ticker.clone(),
                                    &candle,
                                    trailing_stop,
                                    stop_loss_pct,
                                    long_qty,
                                );
                                let res = account.trade::<LimitOrderResponse>(trade);
                                match res {
                                    Err(e) => {
                                        error!("Failed to enter Long: {}", e);
                                        return Ok(());
                                    }
                                    Ok(res) => {
                                        debug!("{:?}", res);
                                        info!(
                                            "Long {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                                            kline_event.kline.symbol,
                                            date.to_string(),
                                            prev_candle.close,
                                            candle.close,
                                            plpl
                                        );
                                    }
                                };
                            }
                            Some(active_order) => match active_order.side() {
                                Side::Long => {
                                    info!("Already Long, ignoring");
                                }
                                Side::Short => {
                                    info!("Close Short, enter Long");
                                    match account.cancel_all_active_orders() {
                                        Err(_) => {
                                            info!("No active orders to cancel");
                                        }
                                        Ok(_) => {
                                            info!("All active orders canceled");
                                        }
                                    }

                                    let account_info = match account.account_info() {
                                        Err(e) => {
                                            error!("Failed to get account info: {}", e);
                                            return Ok(());
                                        }
                                        Ok(account_info) => account_info,
                                    };
                                    let busd_balance =
                                        free_asset(&account_info, &account.quote_asset);
                                    let busd_balance_locked =
                                        locked_asset(&account_info, &account.quote_asset);
                                    let btc_balance =
                                        free_asset(&account_info, &account.base_asset);
                                    let btc_balance_locked =
                                        locked_asset(&account_info, &account.base_asset);
                                    info!(
                                        "BUSD: locked: {}, free: {}\tBTC: locked: {}, free: {}",
                                        busd_balance_locked,
                                        busd_balance,
                                        btc_balance_locked,
                                        btc_balance
                                    );
                                    let total_balances = total_balance(
                                        &account_info,
                                        &account.quote_asset,
                                        &account.base_asset,
                                    );
                                    info!("Total account balance: {}", total_balances);
                                    // calculate quantity of base asset to trade
                                    // Trade with $1000 or as close as the account can get
                                    let long_qty: f64 = if btc_balance * candle.close < 1000.0 {
                                        btc_balance
                                    } else {
                                        BinanceTrade::round_quantity(1000.0 / candle.close)
                                    };

                                    let trade = plpl_long(
                                        account.ticker.clone(),
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        long_qty,
                                    );
                                    let res = account.trade::<LimitOrderResponse>(trade);
                                    match res {
                                        Err(e) => {
                                            error!("Failed to enter Long: {}", e);
                                            return Ok(());
                                        }
                                        Ok(res) => {
                                            debug!("{:?}", res);
                                            info!(
                                                "Long {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                                                kline_event.kline.symbol,
                                                date.to_string(),
                                                prev_candle.close,
                                                candle.close,
                                                plpl
                                            );
                                        }
                                    };
                                }
                            },
                        }
                    } else if plpl_system.short_signal(prev_candle, &candle, plpl) {
                        // if position is Short, ignore
                        // if position is Long, close long and open Short
                        // if position is None, enter Short
                        match account.get_active_order() {
                            None => {
                                info!("No active order, enter Short");
                                match account.cancel_all_active_orders() {
                                    Err(_) => {
                                        info!("No active orders to cancel");
                                    }
                                    Ok(_) => {
                                        info!("All active orders canceled");
                                    }
                                }

                                let account_info = match account.account_info() {
                                    Err(e) => {
                                        error!("Failed to get account info: {}", e);
                                        return Ok(());
                                    }
                                    Ok(account_info) => account_info,
                                };
                                let busd_balance = free_asset(&account_info, &account.quote_asset);
                                let busd_balance_locked =
                                    locked_asset(&account_info, &account.quote_asset);
                                let btc_balance = free_asset(&account_info, &account.base_asset);
                                let btc_balance_locked =
                                    locked_asset(&account_info, &account.base_asset);
                                info!(
                                    "BUSD: locked: {}, free: {}\tBTC: locked: {}, free: {}",
                                    busd_balance_locked,
                                    busd_balance,
                                    btc_balance_locked,
                                    btc_balance
                                );
                                let total_balances = total_balance(
                                    &account_info,
                                    &account.quote_asset,
                                    &account.base_asset,
                                );
                                info!("Total account balance: {}", total_balances);
                                // calculate quantity of base asset to trade
                                // Trade with $1000 or as close as the account can get
                                let short_qty: f64 = if busd_balance < 1000.0 {
                                    busd_balance
                                } else {
                                    BinanceTrade::round_quantity(1000.0 / candle.close)
                                };

                                let trade = plpl_short(
                                    account.ticker.clone(),
                                    &candle,
                                    trailing_stop,
                                    stop_loss_pct,
                                    short_qty,
                                );
                                let res = account.trade::<LimitOrderResponse>(trade);
                                match res {
                                    Err(e) => {
                                        error!("Failed to enter Short: {}", e);
                                        return Ok(());
                                    }
                                    Ok(res) => {
                                        debug!("{:?}", res);
                                        info!(
                                            "Short {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                                            kline_event.kline.symbol,
                                            date.to_string(),
                                            prev_candle.close,
                                            candle.close,
                                            plpl
                                        );
                                    }
                                };
                            }
                            Some(active_order) => match active_order.side() {
                                Side::Long => {
                                    info!("Close Long, enter Short");
                                    match account.cancel_all_active_orders() {
                                        Err(_) => {
                                            info!("No active orders to cancel");
                                        }
                                        Ok(_) => {
                                            info!("All active orders canceled");
                                        }
                                    }

                                    let account_info = match account.account_info() {
                                        Err(e) => {
                                            error!("Failed to get account info: {}", e);
                                            return Ok(());
                                        }
                                        Ok(account_info) => account_info,
                                    };
                                    let busd_balance =
                                        free_asset(&account_info, &account.quote_asset);
                                    let busd_balance_locked =
                                        locked_asset(&account_info, &account.quote_asset);
                                    let btc_balance =
                                        free_asset(&account_info, &account.base_asset);
                                    let btc_balance_locked =
                                        locked_asset(&account_info, &account.base_asset);
                                    info!(
                                        "BUSD: locked: {}, free: {}\tBTC: locked: {}, free: {}",
                                        busd_balance_locked,
                                        busd_balance,
                                        btc_balance_locked,
                                        btc_balance
                                    );
                                    let total_balances = total_balance(
                                        &account_info,
                                        &account.quote_asset,
                                        &account.base_asset,
                                    );
                                    info!("Total account balance: {}", total_balances);
                                    // calculate quantity of base asset to trade
                                    // Trade with $1000 or as close as the account can get
                                    let short_qty: f64 = if busd_balance < 1000.0 {
                                        busd_balance
                                    } else {
                                        BinanceTrade::round_quantity(1000.0 / candle.close)
                                    };

                                    let trade = plpl_short(
                                        account.ticker.clone(),
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        short_qty,
                                    );
                                    let res = account.trade::<LimitOrderResponse>(trade);
                                    match res {
                                        Err(e) => {
                                            error!("Failed to enter Short: {}", e);
                                            return Ok(());
                                        }
                                        Ok(res) => {
                                            debug!("{:?}", res);
                                            info!(
                                                "Short {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                                                kline_event.kline.symbol,
                                                date.to_string(),
                                                prev_candle.close,
                                                candle.close,
                                                plpl
                                            );
                                        }
                                    };
                                }
                                Side::Short => {
                                    info!("Already Short, ignoring");
                                }
                            },
                        }
                    }
                }
                (None, Some(_)) => {
                    error!(
                        "Previous candle is None and current candle is Some. Should never occur!"
                    );
                    unreachable!()
                }
                (Some(_prev_candle), Some(curr_candle)) => {
                    if plpl_system.long_signal(curr_candle, &candle, plpl) {
                        // if position is Long, ignore
                        // if position is Short, close short and enter Long
                        // if position is None, enter Long
                        match account.get_active_order() {
                            None => {
                                info!("No active order, enter Long");
                                match account.cancel_all_active_orders() {
                                    Err(_) => {
                                        info!("No active orders to cancel");
                                    }
                                    Ok(_) => {
                                        info!("All active orders canceled");
                                    }
                                }

                                let account_info = match account.account_info() {
                                    Err(e) => {
                                        error!("Failed to get account info: {}", e);
                                        return Ok(());
                                    }
                                    Ok(account_info) => account_info,
                                };
                                let busd_balance = free_asset(&account_info, &account.quote_asset);
                                let busd_balance_locked =
                                    locked_asset(&account_info, &account.quote_asset);
                                let btc_balance = free_asset(&account_info, &account.base_asset);
                                let btc_balance_locked =
                                    locked_asset(&account_info, &account.base_asset);
                                info!(
                                    "BUSD: locked: {}, free: {}\tBTC: locked: {}, free: {}",
                                    busd_balance_locked,
                                    busd_balance,
                                    btc_balance_locked,
                                    btc_balance
                                );
                                let total_balances = total_balance(
                                    &account_info,
                                    &account.quote_asset,
                                    &account.base_asset,
                                );
                                info!("Total account balance: {}", total_balances);
                                // calculate quantity of base asset to trade
                                // Trade with $1000 or as close as the account can get
                                let long_qty: f64 = if btc_balance * candle.close < 1000.0 {
                                    btc_balance
                                } else {
                                    BinanceTrade::round_quantity(1000.0 / candle.close)
                                };

                                let trade = plpl_long(
                                    account.ticker.clone(),
                                    &candle,
                                    trailing_stop,
                                    stop_loss_pct,
                                    long_qty,
                                );
                                let res = account.trade::<LimitOrderResponse>(trade);
                                match res {
                                    Err(e) => {
                                        error!("Failed to enter Long: {}", e);
                                        return Ok(());
                                    }
                                    Ok(res) => {
                                        debug!("{:?}", res);
                                        info!(
                                            "Long {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                                            kline_event.kline.symbol,
                                            date.to_string(),
                                            curr_candle.close,
                                            candle.close,
                                            plpl
                                        );
                                    }
                                };
                            }
                            Some(active_order) => match active_order.side() {
                                Side::Long => {
                                    info!("Already Long, ignoring");
                                }
                                Side::Short => {
                                    info!("Close Short, enter Long");
                                    match account.cancel_all_active_orders() {
                                        Err(_) => {
                                            info!("No active orders to cancel");
                                        }
                                        Ok(_) => {
                                            info!("All active orders canceled");
                                        }
                                    }

                                    let account_info = match account.account_info() {
                                        Err(e) => {
                                            error!("Failed to get account info: {}", e);
                                            return Ok(());
                                        }
                                        Ok(account_info) => account_info,
                                    };
                                    let busd_balance =
                                        free_asset(&account_info, &account.quote_asset);
                                    let busd_balance_locked =
                                        locked_asset(&account_info, &account.quote_asset);
                                    let btc_balance =
                                        free_asset(&account_info, &account.base_asset);
                                    let btc_balance_locked =
                                        locked_asset(&account_info, &account.base_asset);
                                    info!(
                                        "BUSD: locked: {}, free: {}\tBTC: locked: {}, free: {}",
                                        busd_balance_locked,
                                        busd_balance,
                                        btc_balance_locked,
                                        btc_balance
                                    );
                                    let total_balances = total_balance(
                                        &account_info,
                                        &account.quote_asset,
                                        &account.base_asset,
                                    );
                                    info!("Total account balance: {}", total_balances);
                                    // calculate quantity of base asset to trade
                                    // Trade with $1000 or as close as the account can get
                                    let long_qty: f64 = if btc_balance * candle.close < 1000.0 {
                                        btc_balance
                                    } else {
                                        BinanceTrade::round_quantity(1000.0 / candle.close)
                                    };

                                    let trade = plpl_long(
                                        account.ticker.clone(),
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        long_qty,
                                    );
                                    let res = account.trade::<LimitOrderResponse>(trade);
                                    match res {
                                        Err(e) => {
                                            error!("Failed to enter Long: {}", e);
                                            return Ok(());
                                        }
                                        Ok(res) => {
                                            debug!("{:?}", res);
                                            info!(
                                                "Long {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                                                kline_event.kline.symbol,
                                                date.to_string(),
                                                curr_candle.close,
                                                candle.close,
                                                plpl
                                            );
                                        }
                                    };
                                }
                            },
                        }
                    } else if plpl_system.short_signal(curr_candle, &candle, plpl) {
                        // if position is Short, ignore
                        // if position is Long, close long and enter Short
                        // if position is None, enter Short
                        match account.get_active_order() {
                            None => {
                                info!("No active order, enter Short");
                                match account.cancel_all_active_orders() {
                                    Err(_) => {
                                        info!("No active orders to cancel");
                                    }
                                    Ok(_) => {
                                        info!("All active orders canceled");
                                    }
                                }

                                let account_info = match account.account_info() {
                                    Err(e) => {
                                        error!("Failed to get account info: {}", e);
                                        return Ok(());
                                    }
                                    Ok(account_info) => account_info,
                                };
                                let busd_balance = free_asset(&account_info, &account.quote_asset);
                                let busd_balance_locked =
                                    locked_asset(&account_info, &account.quote_asset);
                                let btc_balance = free_asset(&account_info, &account.base_asset);
                                let btc_balance_locked =
                                    locked_asset(&account_info, &account.base_asset);
                                info!(
                                    "BUSD: locked: {}, free: {}\tBTC: locked: {}, free: {}",
                                    busd_balance_locked,
                                    busd_balance,
                                    btc_balance_locked,
                                    btc_balance
                                );
                                let total_balances = total_balance(
                                    &account_info,
                                    &account.quote_asset,
                                    &account.base_asset,
                                );
                                info!("Total account balance: {}", total_balances);
                                // calculate quantity of base asset to trade
                                // Trade with $1000 or as close as the account can get
                                let short_qty: f64 = if busd_balance < 1000.0 {
                                    busd_balance
                                } else {
                                    BinanceTrade::round_quantity(1000.0 / candle.close)
                                };

                                let trade = plpl_short(
                                    account.ticker.clone(),
                                    &candle,
                                    trailing_stop,
                                    stop_loss_pct,
                                    short_qty,
                                );
                                let res = account.trade::<LimitOrderResponse>(trade);
                                match res {
                                    Err(e) => {
                                        error!("Failed to enter Short: {}", e);
                                        return Ok(());
                                    }
                                    Ok(res) => {
                                        debug!("{:?}", res);
                                        info!(
                                            "Short {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                                            kline_event.kline.symbol,
                                            date.to_string(),
                                            curr_candle.close,
                                            candle.close,
                                            plpl
                                        );
                                    }
                                };
                            }
                            Some(active_order) => match active_order.side() {
                                Side::Long => {
                                    info!("Close Long, enter Short");
                                    match account.cancel_all_active_orders() {
                                        Err(_) => {
                                            info!("No active orders to cancel");
                                        }
                                        Ok(_) => {
                                            info!("All active orders canceled");
                                        }
                                    }

                                    let account_info = match account.account_info() {
                                        Err(e) => {
                                            error!("Failed to get account info: {}", e);
                                            return Ok(());
                                        }
                                        Ok(account_info) => account_info,
                                    };
                                    let busd_balance =
                                        free_asset(&account_info, &account.quote_asset);
                                    let busd_balance_locked =
                                        locked_asset(&account_info, &account.quote_asset);
                                    let btc_balance =
                                        free_asset(&account_info, &account.base_asset);
                                    let btc_balance_locked =
                                        locked_asset(&account_info, &account.base_asset);
                                    info!(
                                        "BUSD: locked: {}, free: {}\tBTC: locked: {}, free: {}",
                                        busd_balance_locked,
                                        busd_balance,
                                        btc_balance_locked,
                                        btc_balance
                                    );
                                    let total_balances = total_balance(
                                        &account_info,
                                        &account.quote_asset,
                                        &account.base_asset,
                                    );
                                    info!("Total account balance: {}", total_balances);
                                    // calculate quantity of base asset to trade
                                    // Trade with $1000 or as close as the account can get
                                    let short_qty: f64 = if busd_balance < 1000.0 {
                                        busd_balance
                                    } else {
                                        BinanceTrade::round_quantity(1000.0 / candle.close)
                                    };

                                    let trade = plpl_short(
                                        account.ticker.clone(),
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        short_qty,
                                    );
                                    let res = account.trade::<LimitOrderResponse>(trade);
                                    match res {
                                        Err(e) => {
                                            error!("Failed to enter Short: {}", e);
                                            return Ok(());
                                        }
                                        Ok(res) => {
                                            debug!("{:?}", res);
                                            info!(
                                                "Short {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                                                kline_event.kline.symbol,
                                                date.to_string(),
                                                curr_candle.close,
                                                candle.close,
                                                plpl
                                            );
                                        }
                                    };
                                }
                                Side::Short => {
                                    info!("Already Short, ignoring");
                                }
                            },
                        }
                    }
                    *prev = Some(curr_candle.clone());
                    *curr = Some(candle);
                }
            }
            let end = SystemTime::now();
            // time to process
            let elapsed = end.duration_since(start).expect("Time went backwards");
            info!("Time to process Kline event: {:?}ms", elapsed.as_millis());
        }
        Ok(())
    });

    let sub = String::from("btcbusd@kline_5m");
    ws.connect_with_config(&sub, &config)
        .expect("Failed to connect to Binance websocket");
    info!("Binance websocket connected");

    if let Err(e) = ws.event_loop(&keep_running) {
        info!("Binance websocket error: {}", e);
    }

    ws.disconnect()
        .expect("Failed to disconnect from Binance websocket");
    info!("Binance websocket disconnected");

    Ok(())
}

fn plpl_long(
    ticker: String,
    candle: &Candle,
    trailing_stop_pct: f64,
    stop_loss_pct: f64,
    qty: f64,
) -> BinanceTrade {
    let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop_pct);
    let stop_loss = BinanceTrade::calc_stop_loss(Side::Long, candle.close, stop_loss_pct);
    let limit = BinanceTrade::round_price(candle.close);
    BinanceTrade::new(
        ticker,
        Side::Long,
        OrderType::TakeProfitLimit,
        qty,
        Some(limit),
        Some(stop_loss),
        Some(trailing_stop),
    )
}

fn plpl_short(
    ticker: String,
    candle: &Candle,
    trailing_stop_pct: f64,
    stop_loss_pct: f64,
    qty: f64,
) -> BinanceTrade {
    let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop_pct);
    let stop_loss = BinanceTrade::calc_stop_loss(Side::Short, candle.close, stop_loss_pct);
    let limit = BinanceTrade::round_price(candle.close);
    BinanceTrade::new(
        ticker,
        Side::Short,
        OrderType::TakeProfitLimit,
        qty,
        Some(limit),
        Some(stop_loss),
        Some(trailing_stop),
    )
}
