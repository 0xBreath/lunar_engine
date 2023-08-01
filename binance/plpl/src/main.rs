#[macro_use]
extern crate lazy_static;

use ephemeris::*;
use library::*;
use log::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Mutex;
use std::time::SystemTime;
use time_series::{Candle, Day, Month, Time};

mod utils;
use utils::*;

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
#[allow(dead_code)]
const BINANCE_LIVE_API: &str = "https://api.binance.us";
#[allow(dead_code)]
const BINANCE_LIVE_API_KEY: &str =
    "WeGpjrcMfU4Yndtb8tOqy2MQouEWsGuQbCwNHOwCSKtnxm5MUhqB6EOyQ3u7rBFY";
#[allow(dead_code)]
const BINANCE_LIVE_API_SECRET: &str =
    "aLfkivKBnH31bhfcOc1P7qdg7HxLRcjCRBMDdiViVXMfO64TFEYe6V1OKr0MjyJS";
const KLINE_STREAM: &str = "btcusdt@kline_5m";
const BASE_ASSET: &str = "BTC";
const QUOTE_ASSET: &str = "USDT";
const TICKER: &str = "BTCUSDT";
const IS_TESTNET: bool = true;

lazy_static! {
    static ref ACCOUNT: Mutex<Account> = Mutex::new(Account {
        // client: Client::new(
        //     Some(BINANCE_LIVE_API_KEY.to_string()),
        //     Some(BINANCE_LIVE_API_SECRET.to_string()),
        //     BINANCE_LIVE_API.to_string()
        // ),
        client: Client::new(
            Some(BINANCE_TEST_API_KEY.to_string()),
            Some(BINANCE_TEST_API_SECRET.to_string()),
            BINANCE_TEST_API.to_string()
        ),
        recv_window: 5000,
        base_asset: BASE_ASSET.to_string(),
        quote_asset: QUOTE_ASSET.to_string(),
        ticker: TICKER.to_string(),
        active_order: None,
    });
    static ref USER_STREAM: Mutex<UserStream> = Mutex::new(UserStream {
        // client: Client::new(
        //     Some(BINANCE_LIVE_API_KEY.to_string()),
        //     Some(BINANCE_LIVE_API_SECRET.to_string()),
        //     BINANCE_LIVE_API.to_string()
        // ),
        client: Client::new(
            Some(BINANCE_TEST_API_KEY.to_string()),
            Some(BINANCE_TEST_API_SECRET.to_string()),
            BINANCE_TEST_API.to_string()
        ),
        recv_window: 10000,
    });
    // cache previous and current Kline/Candle to assess PLPL trade signal
    static ref PREV_CANDLE: Mutex<Option<Candle>> = Mutex::new(None);
    static ref CURR_CANDLE: Mutex<Option<Candle>> = Mutex::new(None);
    static ref COUNTER: Mutex<AtomicUsize> = Mutex::new(AtomicUsize::new(0));
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(&PathBuf::from("plpl_binance.log".to_string()));

    info!("Starting Binance PLPL!");
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
    .map_err(|e| BinanceError::Custom(e.to_string()))?;

    let prev_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let curr_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let mut account = ACCOUNT
        .lock()
        .map_err(|e| BinanceError::Custom(e.to_string()))?;

    let user_stream = USER_STREAM
        .lock()
        .map_err(|e| BinanceError::Custom(e.to_string()))?;
    let answer = user_stream.start()?;
    let listen_key = answer.listen_key;

    let mut ws = WebSockets::new(IS_TESTNET, |event: WebSocketEvent| {
        let start = SystemTime::now();
        match event {
            WebSocketEvent::Kline(kline_event) => {
                let kline_event_time = kline_event.event_time as i64;
                let date = Time::from_unix_msec(kline_event_time);
                let client_order_id = format!("{}", kline_event_time);
                let candle = kline_to_candle(&kline_event);
                let mut prev = prev_candle.lock().map_err(|_| {
                    BinanceError::Custom("Failed to lock previous candle".to_string())
                })?;
                let mut curr = curr_candle.lock().map_err(|_| {
                    BinanceError::Custom("Failed to lock current candle".to_string())
                })?;

                // compute closest PLPL to current Candle
                let plpl = plpl_system
                    .closest_plpl(&candle)
                    .expect("Closest PLPL not found");
                // active order bundle on Binance
                let active_order = account.get_active_order();
                let mut trade_placed = false;

                // compare previous candle to current candle to check crossover of PLPL signal threshold
                match (&*prev, &*curr) {
                    (None, None) => *prev = Some(candle),
                    (Some(prev_candle), None) => {
                        *curr = Some(candle.clone());
                        if plpl_system.long_signal(prev_candle, &candle, plpl) {
                            // if position is Long, ignore
                            // if position is Short, close short and open Long
                            // if position is None, enter Long
                            match active_order {
                                None => {
                                    info!(
                                        "No active order, enter Long @ {} | {}",
                                        candle.close,
                                        date.to_string()
                                    );
                                    let account_info = account.account_info()?;
                                    let trades = plpl_long(
                                        &account_info,
                                        &client_order_id,
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        TICKER,
                                        QUOTE_ASSET,
                                        BASE_ASSET,
                                    );
                                    for trade in trades {
                                        let side = trade.side.clone();
                                        let order_type = trade.order_type.clone();
                                        if let Err(e) = account.trade::<LimitOrderResponse>(trade) {
                                            error!(
                                                "Error entering {} for {}: {:?}",
                                                side.fmt_binance(),
                                                order_type.fmt_binance(),
                                                e
                                            );
                                        }
                                    }
                                    trade_placed = true;
                                }
                                Some(active_order) => match active_order.side {
                                    Side::Long => {
                                        debug!("Already Long, ignoring");
                                    }
                                    Side::Short => {
                                        info!(
                                            "Close Short, enter Long @ {} | {}",
                                            candle.close,
                                            date.to_string()
                                        );
                                        let account_info = account.account_info()?;
                                        let trades = plpl_long(
                                            &account_info,
                                            &client_order_id,
                                            &candle,
                                            trailing_stop,
                                            stop_loss_pct,
                                            TICKER,
                                            QUOTE_ASSET,
                                            BASE_ASSET,
                                        );
                                        for trade in trades {
                                            let side = trade.side.clone();
                                            let order_type = trade.order_type.clone();
                                            if let Err(e) =
                                                account.trade::<LimitOrderResponse>(trade)
                                            {
                                                error!(
                                                    "Error entering {} for {}: {:?}",
                                                    side.fmt_binance(),
                                                    order_type.fmt_binance(),
                                                    e
                                                );
                                            }
                                        }
                                        trade_placed = true;
                                    }
                                },
                            }
                        } else if plpl_system.short_signal(prev_candle, &candle, plpl) {
                            // if position is Short, ignore
                            // if position is Long, close long and open Short
                            // if position is None, enter Short
                            match active_order {
                                None => {
                                    info!(
                                        "No active order, enter Short @ {} | {}",
                                        candle.close,
                                        date.to_string()
                                    );
                                    let account_info = account.account_info()?;
                                    let trades = plpl_short(
                                        &account_info,
                                        &client_order_id,
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        TICKER,
                                        QUOTE_ASSET,
                                        BASE_ASSET,
                                    );
                                    for trade in trades {
                                        let side = trade.side.clone();
                                        let order_type = trade.order_type.clone();
                                        if let Err(e) = account.trade::<LimitOrderResponse>(trade) {
                                            error!(
                                                "Error entering {} for {}: {:?}",
                                                side.fmt_binance(),
                                                order_type.fmt_binance(),
                                                e
                                            );
                                        }
                                    }
                                    trade_placed = true;
                                }
                                Some(active_order) => match active_order.side {
                                    Side::Long => {
                                        info!(
                                            "Close Long, enter Short @ {} | {}",
                                            candle.close,
                                            date.to_string()
                                        );
                                        let account_info = account.account_info()?;
                                        let trades = plpl_short(
                                            &account_info,
                                            &client_order_id,
                                            &candle,
                                            trailing_stop,
                                            stop_loss_pct,
                                            TICKER,
                                            QUOTE_ASSET,
                                            BASE_ASSET,
                                        );
                                        for trade in trades {
                                            let side = trade.side.clone();
                                            let order_type = trade.order_type.clone();
                                            if let Err(e) =
                                                account.trade::<LimitOrderResponse>(trade)
                                            {
                                                error!(
                                                    "Error entering {} for {}: {:?}",
                                                    side.fmt_binance(),
                                                    order_type.fmt_binance(),
                                                    e
                                                );
                                            }
                                        }
                                        trade_placed = true;
                                    }
                                    Side::Short => {
                                        debug!("Already Short, ignoring");
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
                            match active_order {
                                None => {
                                    info!(
                                        "No active order, enter Long @ {} | {}",
                                        candle.close,
                                        date.to_string()
                                    );
                                    let account_info = account.account_info()?;
                                    let trades = plpl_long(
                                        &account_info,
                                        &client_order_id,
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        TICKER,
                                        QUOTE_ASSET,
                                        BASE_ASSET,
                                    );
                                    for trade in trades {
                                        let side = trade.side.clone();
                                        let order_type = trade.order_type.clone();
                                        if let Err(e) = account.trade::<LimitOrderResponse>(trade) {
                                            error!(
                                                "Error entering {} for {}: {:?}",
                                                side.fmt_binance(),
                                                order_type.fmt_binance(),
                                                e
                                            );
                                        }
                                    }
                                    trade_placed = true;
                                }
                                Some(active_order) => match active_order.side {
                                    Side::Long => {
                                        debug!("Already Long, ignoring");
                                    }
                                    Side::Short => {
                                        info!(
                                            "Close Short, enter Long @ {} | {}",
                                            candle.close,
                                            date.to_string()
                                        );
                                        let account_info = account.account_info()?;
                                        let trades = plpl_long(
                                            &account_info,
                                            &client_order_id,
                                            &candle,
                                            trailing_stop,
                                            stop_loss_pct,
                                            TICKER,
                                            QUOTE_ASSET,
                                            BASE_ASSET,
                                        );
                                        for trade in trades {
                                            let side = trade.side.clone();
                                            let order_type = trade.order_type.clone();
                                            if let Err(e) =
                                                account.trade::<LimitOrderResponse>(trade)
                                            {
                                                error!(
                                                    "Error entering {} for {}: {:?}",
                                                    side.fmt_binance(),
                                                    order_type.fmt_binance(),
                                                    e
                                                );
                                            }
                                        }
                                        trade_placed = true;
                                    }
                                },
                            }
                        } else if plpl_system.short_signal(curr_candle, &candle, plpl) {
                            // if position is Short, ignore
                            // if position is Long, close long and enter Short
                            // if position is None, enter Short
                            match active_order {
                                None => {
                                    info!(
                                        "No active order, enter Short @ {} | {}",
                                        candle.close,
                                        date.to_string()
                                    );
                                    let account_info = account.account_info()?;
                                    let trades = plpl_short(
                                        &account_info,
                                        &client_order_id,
                                        &candle,
                                        trailing_stop,
                                        stop_loss_pct,
                                        TICKER,
                                        QUOTE_ASSET,
                                        BASE_ASSET,
                                    );
                                    for trade in trades {
                                        let side = trade.side.clone();
                                        let order_type = trade.order_type.clone();
                                        if let Err(e) = account.trade::<LimitOrderResponse>(trade) {
                                            error!(
                                                "Error entering {} for {}: {:?}",
                                                side.fmt_binance(),
                                                order_type.fmt_binance(),
                                                e
                                            );
                                        }
                                    }
                                    trade_placed = true;
                                }
                                Some(active_order) => match active_order.side {
                                    Side::Long => {
                                        info!(
                                            "Close Long, enter Short @ {} | {}",
                                            candle.close,
                                            date.to_string()
                                        );
                                        let account_info = account.account_info()?;
                                        let trades = plpl_short(
                                            &account_info,
                                            &client_order_id,
                                            &candle,
                                            trailing_stop,
                                            stop_loss_pct,
                                            TICKER,
                                            QUOTE_ASSET,
                                            BASE_ASSET,
                                        );
                                        for trade in trades {
                                            let side = trade.side.clone();
                                            let order_type = trade.order_type.clone();
                                            if let Err(e) =
                                                account.trade::<LimitOrderResponse>(trade)
                                            {
                                                error!(
                                                    "Error entering {} for {}: {:?}",
                                                    side.fmt_binance(),
                                                    order_type.fmt_binance(),
                                                    e
                                                );
                                            }
                                        }
                                        trade_placed = true;
                                    }
                                    Side::Short => {
                                        debug!("Already Short, ignoring");
                                    }
                                },
                            }
                        }
                        *prev = Some(curr_candle.clone());
                        *curr = Some(candle);
                    }
                }
                // time to process
                let elapsed = SystemTime::now().duration_since(start).unwrap();
                if trade_placed {
                    info!("Time to process PLPL trade: {:?}ms", elapsed.as_millis());
                }
            }
            WebSocketEvent::AccountUpdate(account_update) => {
                for balance in &account_update.data.balances {
                    info!(
                        "Asset: {}, wallet_balance: {}, cross_wallet_balance: {}, balance: {}",
                        balance.asset,
                        balance.wallet_balance,
                        balance.cross_wallet_balance,
                        balance.balance_change
                    );
                }
            }
            WebSocketEvent::OrderTrade(event) => {
                info!(
                    "Ticker: {}, ID: {}, Side: {}, Price: {}, Status: {}, Type: {}",
                    event.symbol,
                    event.new_client_order_id,
                    event.side,
                    event.price,
                    event.order_status,
                    event.order_type
                );
                return match account.stream_update_active_order(event) {
                    Ok(active_order) => {
                        info!("Active order updated {:?}", active_order);
                        Ok(())
                    }
                    Err(e) => {
                        error!("Error updating active order: {:?}", e);
                        Err(e)
                    }
                };
            }
            _ => (),
        };
        Ok(())
    });

    let subs = vec![KLINE_STREAM.to_string(), listen_key.clone()];
    match ws.connect_multiple_streams(&subs, IS_TESTNET) {
        Err(e) => {
            error!("Failed to connect to Binance websocket: {}", e);
            return Err(e);
        }
        Ok(_) => info!("Binance websocket connected"),
    }

    if let Err(e) = ws.event_loop(&keep_running) {
        error!("Binance websocket error: {}", e);
    }

    user_stream.close(&listen_key)?;

    match ws.disconnect() {
        Err(e) => {
            error!("Failed to disconnect from Binance websocket: {}", e);
            match ws.connect_multiple_streams(&subs, IS_TESTNET) {
                Err(e) => {
                    error!("Failed to connect to Binance websocket: {}", e);
                    Err(e)
                }
                Ok(_) => {
                    info!("Binance websocket reconnected");
                    Ok(())
                }
            }
        }
        Ok(_) => {
            info!("Binance websocket disconnected");
            Ok(())
        }
    }
}
