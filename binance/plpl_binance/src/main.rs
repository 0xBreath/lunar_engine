#[macro_use]
extern crate lazy_static;

use binance_lib::*;
use crossbeam::channel::unbounded;
use ephemeris::*;
use log::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use time_series::{Candle, Day, Month, Time};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

// Binance Spot Test Network API credentials
#[allow(dead_code)]
const BINANCE_TEST_API: &str = "https://testnet.binance.vision";
#[allow(dead_code)]
const BINANCE_TEST_API_KEY: &str =
    "AekFIdmCDmPkaeQjCjaPtEE9IvYtpoceePvvelkthAh7tEtvMAm7oHzcxkhbmxl0";
#[allow(dead_code)]
const BINANCE_TEST_API_SECRET: &str =
    "epU83XZHBcHuvznmccDQCbCcxbGeVq6sl4AspOyALCTqWkeG1CVlJx6BzXIC2wXK";
// Binance Spot Live Network API credentials
const BINANCE_LIVE_API: &str = "https://api.binance.us";
const BINANCE_LIVE_API_KEY: &str =
    "WeGpjrcMfU4Yndtb8tOqy2MQouEWsGuQbCwNHOwCSKtnxm5MUhqB6EOyQ3u7rBFY";
const BINANCE_LIVE_API_SECRET: &str =
    "aLfkivKBnH31bhfcOc1P7qdg7HxLRcjCRBMDdiViVXMfO64TFEYe6V1OKr0MjyJS";
const KLINE_STREAM: &str = "btcusdt@kline_5m";

lazy_static! {
    static ref ACCOUNT: Arc<Mutex<Account>> = Arc::new(Mutex::new(Account {
        client: Client::new(
            Some(BINANCE_LIVE_API_KEY.to_string()),
            Some(BINANCE_LIVE_API_SECRET.to_string()),
            BINANCE_LIVE_API.to_string()
        ),
        recv_window: 5000,
        base_asset: "BTC".to_string(),
        quote_asset: "USDT".to_string(),
        ticker: "BTCUSDT".to_string(),
        active_order: None,
    }));
    static ref USER_STREAM: Arc<Mutex<UserStream>> = Arc::new(Mutex::new(UserStream {
        client: Client::new(
            Some(BINANCE_LIVE_API_KEY.to_string()),
            Some(BINANCE_LIVE_API_SECRET.to_string()),
            BINANCE_LIVE_API.to_string()
        ),
        recv_window: 10000,
    }));
    static ref PREV_CANDLE: Arc<Mutex<Option<Candle>>> = Arc::new(Mutex::new(None));
    static ref CURR_CANDLE: Arc<Mutex<Option<Candle>>> = Arc::new(Mutex::new(None));
    static ref COUNTER: Arc<Mutex<AtomicUsize>> = Arc::new(Mutex::new(AtomicUsize::new(0)));
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
            File::create(log_file).expect("Failed to create PLPL Binance log file"),
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

struct Assets {
    free_quote: f64,
    locked_quote: f64,
    free_base: f64,
    locked_base: f64,
}

fn account_assets(account: &AccountInfoResponse, quote_asset: &str, base_asset: &str) -> Assets {
    let free_quote = free_asset(account, quote_asset);
    let locked_quote = locked_asset(account, quote_asset);
    let free_base = free_asset(account, base_asset);
    let locked_base = locked_asset(account, base_asset);
    Assets {
        free_quote,
        locked_quote,
        free_base,
        locked_base,
    }
}

fn trade_qty(
    account_info: &AccountInfoResponse,
    quote_asset: &str,
    base_asset: &str,
    side: Side,
    candle: &Candle,
) -> f64 {
    let assets = account_assets(account_info, quote_asset, base_asset);
    info!(
        "{}: {}, {}: {}",
        quote_asset,
        assets.free_quote + assets.locked_quote,
        base_asset,
        assets.free_base + assets.locked_base,
    );
    match side {
        Side::Long => {
            let qty = assets.free_quote / candle.close * 0.99;
            BinanceTrade::round_quantity(qty, 5)
        }
        Side::Short => {
            let qty = assets.free_base * 0.99;
            BinanceTrade::round_quantity(qty, 5)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(&PathBuf::from("plpl_binance.log".to_string()));

    info!("Starting Binance PLPL!");
    let config = Config::default();
    let keep_running = AtomicBool::new(true);

    // PLPL parameters; tuned for 5 minute candles
    let trailing_stop = 0.5;
    let stop_loss_pct = 0.05;
    let planet = Planet::from("Jupiter");
    let plpl_scale = 0.5;
    let plpl_price = 20000.0;
    let num_plpls = 8000;
    let cross_margin_pct = 55.0;

    // initialize PLPL
    let plpl_system = PLPLSystem::new(PLPLSystemConfig {
        planet,
        origin: Origin::Heliocentric,
        first_date: Time::new(2023, &Month::from_num(7), &Day::from_num(1), None, None),
        last_date: Time::new(2050, &Month::from_num(7), &Day::from_num(1), None, None),
        plpl_scale,
        plpl_price,
        num_plpls,
        cross_margin_pct,
    })
    .await
    .expect("Failed to initialize PLPL system");
    let plpl_system = Arc::new(Mutex::new(plpl_system));

    // queue to process websocket events asynchronously
    let (queue_tx, queue_rx) = unbounded::<KlineEvent>();

    std::thread::spawn(move || {
        let runtime = Runtime::new().unwrap();
        info!("Starting thread to process queue messages.");
        while let Ok(event) = queue_rx.recv() {
            let kline_event = Arc::new(event).clone();
            let account = ACCOUNT.clone();
            let prev_candle = PREV_CANDLE.clone();
            let curr_candle = CURR_CANDLE.clone();
            let update_counter = COUNTER.clone();
            let plpl_system = plpl_system.clone();

            let queue_size = queue_rx.len();
            runtime.spawn(async move {
                let mut account = account.lock().await;
                let mut prev = prev_candle.lock().await;
                let mut curr = curr_candle.lock().await;
                let update_counter = update_counter.lock().await;
                let plpl_system = plpl_system.lock().await;

                let start = SystemTime::now();
                let count = update_counter.fetch_add(1, Ordering::SeqCst);
                trace!("websocket stream atomic counter: {}", count);
                if !kline_event.kline.is_final_bar {
                    return;
                }
                trace!("websocket processing queue size: {:?}", queue_size);

                let date = Time::from_unix_msec(kline_event.event_time as i64);
                // cache previous and current candle to assess PLPL trade conditions
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
                                    match account.cancel_all_active_orders().await {
                                        Err(_) => {
                                            info!("No active orders to cancel")
                                        }
                                        Ok(_) => {
                                            info!("All active orders canceled");
                                        }
                                    }

                                    let account_info = match account.account_info().await {
                                        Err(e) => {
                                            error!("Failed to get account info: {}", e);
                                            return;
                                        }
                                        Ok(account_info) => account_info,
                                    };
                                    let long_qty = trade_qty(
                                        &account_info,
                                        &account.quote_asset,
                                        &account.base_asset,
                                        Side::Long,
                                        &candle,
                                    );

                                    let trade = plpl_long(
                                        account.ticker.clone(),
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        long_qty,
                                    );
                                    let res = account.trade::<LimitOrderResponse>(trade).await;
                                    match res {
                                        Err(e) => {
                                            error!("Failed to enter Long: {}", e);
                                            return;
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
                                        match account.cancel_all_active_orders().await {
                                            Err(_) => {
                                                info!("No active orders to cancel");
                                            }
                                            Ok(_) => {
                                                info!("All active orders canceled");
                                            }
                                        }

                                        let account_info = match account.account_info().await {
                                            Err(e) => {
                                                error!("Failed to get account info: {}", e);
                                                return;
                                            }
                                            Ok(account_info) => account_info,
                                        };
                                        let long_qty = trade_qty(
                                            &account_info,
                                            &account.quote_asset,
                                            &account.base_asset,
                                            Side::Long,
                                            &candle,
                                        );

                                        let trade = plpl_long(
                                            account.ticker.clone(),
                                            &candle,
                                            trailing_stop,
                                            stop_loss_pct,
                                            long_qty,
                                        );
                                        let res = account.trade::<LimitOrderResponse>(trade).await;
                                        match res {
                                            Err(e) => {
                                                error!("Failed to enter Long: {}", e);
                                                return;
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
                                    match account.cancel_all_active_orders().await {
                                        Err(_) => {
                                            info!("No active orders to cancel");
                                        }
                                        Ok(_) => {
                                            info!("All active orders canceled");
                                        }
                                    }

                                    let account_info = match account.account_info().await {
                                        Err(e) => {
                                            error!("Failed to get account info: {}", e);
                                            return;
                                        }
                                        Ok(account_info) => account_info,
                                    };
                                    let short_qty = trade_qty(
                                        &account_info,
                                        &account.quote_asset,
                                        &account.base_asset,
                                        Side::Short,
                                        &candle,
                                    );

                                    let trade = plpl_short(
                                        account.ticker.clone(),
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        short_qty,
                                    );
                                    let res = account.trade::<LimitOrderResponse>(trade).await;
                                    match res {
                                        Err(e) => {
                                            error!("Failed to enter Short: {}", e);
                                            return;
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
                                        match account.cancel_all_active_orders().await {
                                            Err(_) => {
                                                info!("No active orders to cancel");
                                            }
                                            Ok(_) => {
                                                info!("All active orders canceled");
                                            }
                                        }

                                        let account_info = match account.account_info().await {
                                            Err(e) => {
                                                error!("Failed to get account info: {}", e);
                                                return;
                                            }
                                            Ok(account_info) => account_info,
                                        };
                                        let short_qty = trade_qty(
                                            &account_info,
                                            &account.quote_asset,
                                            &account.base_asset,
                                            Side::Short,
                                            &candle,
                                        );

                                        let trade = plpl_short(
                                            account.ticker.clone(),
                                            &candle,
                                            trailing_stop,
                                            stop_loss_pct,
                                            short_qty,
                                        );
                                        let res = account.trade::<LimitOrderResponse>(trade).await;
                                        match res {
                                            Err(e) => {
                                                error!("Failed to enter Short: {}", e);
                                                return;
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
                    }
                    (Some(_prev_candle), Some(curr_candle)) => {
                        if plpl_system.long_signal(curr_candle, &candle, plpl) {
                            // if position is Long, ignore
                            // if position is Short, close short and enter Long
                            // if position is None, enter Long
                            match account.get_active_order() {
                                None => {
                                    info!("No active order, enter Long");
                                    match account.cancel_all_active_orders().await {
                                        Err(_) => {
                                            info!("No active orders to cancel");
                                        }
                                        Ok(_) => {
                                            info!("All active orders canceled");
                                        }
                                    }

                                    let account_info = match account.account_info().await {
                                        Err(e) => {
                                            error!("Failed to get account info: {}", e);
                                            return;
                                        }
                                        Ok(account_info) => account_info,
                                    };
                                    let long_qty = trade_qty(
                                        &account_info,
                                        &account.quote_asset,
                                        &account.base_asset,
                                        Side::Long,
                                        &candle,
                                    );

                                    let trade = plpl_long(
                                        account.ticker.clone(),
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        long_qty,
                                    );
                                    let res = account.trade::<LimitOrderResponse>(trade).await;
                                    match res {
                                        Err(e) => {
                                            error!("Failed to enter Long: {}", e);
                                            return;
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
                                        match account.cancel_all_active_orders().await {
                                            Err(_) => {
                                                info!("No active orders to cancel");
                                            }
                                            Ok(_) => {
                                                info!("All active orders canceled");
                                            }
                                        }

                                        let account_info = match account.account_info().await {
                                            Err(e) => {
                                                error!("Failed to get account info: {}", e);
                                                return;
                                            }
                                            Ok(account_info) => account_info,
                                        };
                                        let long_qty = trade_qty(
                                            &account_info,
                                            &account.quote_asset,
                                            &account.base_asset,
                                            Side::Long,
                                            &candle,
                                        );

                                        let trade = plpl_long(
                                            account.ticker.clone(),
                                            &candle,
                                            trailing_stop,
                                            stop_loss_pct,
                                            long_qty,
                                        );
                                        let res = account.trade::<LimitOrderResponse>(trade).await;
                                        match res {
                                            Err(e) => {
                                                error!("Failed to enter Long: {}", e);
                                                return;
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
                                    match account.cancel_all_active_orders().await {
                                        Err(_) => {
                                            info!("No active orders to cancel");
                                        }
                                        Ok(_) => {
                                            info!("All active orders canceled");
                                        }
                                    }

                                    let account_info = match account.account_info().await {
                                        Err(e) => {
                                            error!("Failed to get account info: {}", e);
                                            return;
                                        }
                                        Ok(account_info) => account_info,
                                    };
                                    let short_qty = trade_qty(
                                        &account_info,
                                        &account.quote_asset,
                                        &account.base_asset,
                                        Side::Short,
                                        &candle,
                                    );

                                    let trade = plpl_short(
                                        account.ticker.clone(),
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        short_qty,
                                    );
                                    let res = account.trade::<LimitOrderResponse>(trade).await;
                                    match res {
                                        Err(e) => {
                                            error!("Failed to enter Short: {}", e);
                                            return;
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
                                        match account.cancel_all_active_orders().await {
                                            Err(_) => {
                                                info!("No active orders to cancel");
                                            }
                                            Ok(_) => {
                                                info!("All active orders canceled");
                                            }
                                        }

                                        let account_info = match account.account_info().await {
                                            Err(e) => {
                                                error!("Failed to get account info: {}", e);
                                                return;
                                            }
                                            Ok(account_info) => account_info,
                                        };
                                        let short_qty = trade_qty(
                                            &account_info,
                                            &account.quote_asset,
                                            &account.base_asset,
                                            Side::Short,
                                            &candle,
                                        );

                                        let trade = plpl_short(
                                            account.ticker.clone(),
                                            &candle,
                                            trailing_stop,
                                            stop_loss_pct,
                                            short_qty,
                                        );
                                        let res = account.trade::<LimitOrderResponse>(trade).await;
                                        match res {
                                            Err(e) => {
                                                error!("Failed to enter Short: {}", e);
                                                return;
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
                // time to process
                let elapsed = SystemTime::now()
                    .duration_since(start)
                    .expect("Time went backwards");
                info!("Time to process Kline event: {:?}ms", elapsed.as_millis());
            });
        }
    });

    // Kline Websocket
    let mut ws = WebSockets::new(|event: WebSocketEvent| {
        if let WebSocketEvent::Kline(kline_event) = event {
            let res = queue_tx.send(kline_event);
            if let Err(e) = res {
                error!("Failed to send Kline event to queue: {}", e);
                return Ok(());
            }
        }
        Ok(())
    });

    let sub = String::from(KLINE_STREAM);
    match ws.connect_with_config(&sub, &config) {
        Err(e) => {
            error!("Failed to connect to Binance websocket: {}", e);
            return Err(e);
        }
        Ok(_) => info!("Binance websocket connected"),
    }

    if let Err(e) = ws.event_loop(&keep_running) {
        error!("Binance websocket error: {}", e);
    }

    match ws.disconnect() {
        Err(e) => {
            error!("Failed to disconnect from Binance websocket: {}", e);
            match ws.connect_with_config(&sub, &config) {
                Err(e) => {
                    error!("Failed to reconnect to Binance websocket: {}", e);
                    return Err(e);
                }
                Ok(_) => info!("Binance websocket reconnected"),
            }
        }
        Ok(_) => info!("Binance websocket disconnected"),
    }

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
        Some(5000),
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
        Some(5000),
    )
}
