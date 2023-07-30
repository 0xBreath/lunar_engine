#[macro_use]
extern crate lazy_static;

use crossbeam::channel::unbounded;
use ephemeris::*;
use library::*;
use log::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use std::time::SystemTime;
use time_series::{Candle, Day, Month, Time};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

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
const BINANCE_LIVE_API: &str = "https://api.binance.us";
const BINANCE_LIVE_API_KEY: &str =
    "WeGpjrcMfU4Yndtb8tOqy2MQouEWsGuQbCwNHOwCSKtnxm5MUhqB6EOyQ3u7rBFY";
const BINANCE_LIVE_API_SECRET: &str =
    "aLfkivKBnH31bhfcOc1P7qdg7HxLRcjCRBMDdiViVXMfO64TFEYe6V1OKr0MjyJS";
const KLINE_STREAM: &str = "btcusdt@kline_5m";
const TRADE_STREAM: &str = "btcusdt@trade";
const BASE_ASSET: &str = "BTC";
const QUOTE_ASSET: &str = "USDT";
const TICKER: &str = "BTCUSDT";

lazy_static! {
    static ref ACCOUNT: Arc<Mutex<Account>> = Arc::new(Mutex::new(Account {
        client: Client::new(
            Some(BINANCE_LIVE_API_KEY.to_string()),
            Some(BINANCE_LIVE_API_SECRET.to_string()),
            BINANCE_LIVE_API.to_string()
        ),
        recv_window: 5000,
        base_asset: BASE_ASSET.to_string(),
        quote_asset: QUOTE_ASSET.to_string(),
        ticker: TICKER.to_string(),
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
    // cache previous and current Kline/Candle to assess PLPL trade signal
    static ref PREV_CANDLE: Arc<Mutex<Option<Candle>>> = Arc::new(Mutex::new(None));
    static ref CURR_CANDLE: Arc<Mutex<Option<Candle>>> = Arc::new(Mutex::new(None));
    static ref COUNTER: Arc<Mutex<AtomicUsize>> = Arc::new(Mutex::new(AtomicUsize::new(0)));
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
    let plpl_system_inner = PLPLSystem::new(PLPLSystemConfig {
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
    .map_err(|e| BinanceError::Custom(e.to_string()))?;
    let plpl_system = Arc::new(Mutex::new(plpl_system_inner));

    // queue to process websocket events asynchronously
    let (queue_tx, queue_rx) = unbounded::<WebSocketEvent>();

    std::thread::spawn(move || {
        let runtime = Runtime::new().unwrap();

        // Each queue message is a candle or user order/trade update from Binance
        while let Ok(event) = queue_rx.recv() {
            let start = SystemTime::now();
            let account_lock = ACCOUNT.clone();
            let prev_candle = PREV_CANDLE.clone();
            let curr_candle = CURR_CANDLE.clone();
            let plpl_system = plpl_system.clone();

            match event {
                // if any orders within OrderBundle are None,
                // get event client order ID and set or compare and set if ID matches Some(orders) in bundle
                WebSocketEvent::OrderTrade(event) => {
                    runtime.spawn(async move {
                        let account_lock = ACCOUNT.clone();
                        let mut account = account_lock.lock().await;

                        match account.stream_update_active_order(event).await {
                            Ok(active_order) => {
                                info!("Active order updated {:?}", active_order);
                                Ok(())
                            }
                            Err(e) => {
                                error!("Error updating active order: {:?}", e);
                                Err(e)
                            }
                        }
                    });
                }
                // assert OrderBundle has all 3 as Some
                WebSocketEvent::Kline(event) => {
                    runtime.spawn(async move {
                        let kline_event = Arc::new(event).clone();
                        let mut account = account_lock.lock().await;
                        let mut prev = prev_candle.lock().await;
                        let mut curr = curr_candle.lock().await;
                        let plpl_system = plpl_system.lock().await;

                        let kline_event_time = kline_event.event_time as i64;
                        let date = Time::from_unix_msec(kline_event_time);
                        let client_order_id = format!("{}", kline_event_time);
                        let candle = kline_to_candle(&kline_event);

                        // compute closest PLPL to current Candle
                        let plpl = plpl_system
                            .closest_plpl(&candle).unwrap();
                        // active order bundle on Binance
                        let active_order = account.get_active_order();

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
                                            info!("No active order, enter Long");
                                            let account_info = account.account_info().await.unwrap();
                                            let trades = plpl_long(
                                                &account_info,
                                                &client_order_id,
                                                &candle,
                                                trailing_stop,
                                                stop_loss_pct,
                                                TICKER,
                                                QUOTE_ASSET,
                                                BASE_ASSET
                                            );
                                            for trade in trades {
                                                let _ = account.trade::<LimitOrderResponse>(trade).await.unwrap();
                                            }
                                            info!("Long @ {} | {}", candle.close, date.to_string());
                                        }
                                        Some(active_order) => match active_order.side {
                                            Side::Long => {
                                                info!("Already Long, ignoring");
                                            }
                                            Side::Short => {
                                                info!("Close Short, enter Long");
                                                let account_info = account.account_info().await.unwrap();
                                                let trades = plpl_long(
                                                    &account_info,
                                                    &client_order_id,
                                                    &candle,
                                                    trailing_stop,
                                                    stop_loss_pct,
                                                    TICKER,
                                                    QUOTE_ASSET,
                                                    BASE_ASSET
                                                );
                                                for trade in trades {
                                                    let _ = account.trade::<LimitOrderResponse>(trade).await.unwrap();
                                                }
                                                info!("Long @ {} | {}", candle.close, date.to_string());
                                            }
                                        },
                                    }
                                } else if plpl_system.short_signal(prev_candle, &candle, plpl) {
                                    // if position is Short, ignore
                                    // if position is Long, close long and open Short
                                    // if position is None, enter Short
                                    match active_order {
                                        None => {
                                            info!("No active order, enter Short");
                                            let account_info = account.account_info().await.unwrap();
                                            let trades = plpl_short(
                                                &account_info,
                                                &client_order_id,
                                                &candle,
                                                trailing_stop,
                                                stop_loss_pct,
                                                TICKER,
                                                QUOTE_ASSET,
                                                BASE_ASSET
                                            );
                                            for trade in trades {
                                                let _ = account.trade::<LimitOrderResponse>(trade).await.unwrap();
                                            }
                                            info!("Short @ {}", date.to_string());
                                        }
                                        Some(active_order) => match active_order.side {
                                            Side::Long => {
                                                info!("Close Long, enter Short");
                                                let account_info = account.account_info().await.unwrap();
                                                let trades = plpl_short(
                                                    &account_info,
                                                    &client_order_id,
                                                    &candle,
                                                    trailing_stop,
                                                    stop_loss_pct,
                                                    TICKER,
                                                    QUOTE_ASSET,
                                                    BASE_ASSET
                                                );
                                                for trade in trades {
                                                    let _ = account.trade::<LimitOrderResponse>(trade).await.unwrap();
                                                }
                                                info!("Short @ {} | {}", candle.close, date.to_string());
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
                                    match active_order {
                                        None => {
                                            info!("No active order, enter Long");
                                            let account_info = account.account_info().await.unwrap();
                                            let trades = plpl_long(
                                                &account_info,
                                                &client_order_id,
                                                &candle,
                                                trailing_stop,
                                                stop_loss_pct,
                                                TICKER,
                                                QUOTE_ASSET,
                                                BASE_ASSET
                                            );
                                            for trade in trades {
                                                let _ = account.trade::<LimitOrderResponse>(trade).await.unwrap();
                                            }
                                            info!("Long @ {} | {}", candle.close, date.to_string());
                                        }
                                        Some(active_order) => match active_order.side {
                                            Side::Long => {
                                                info!("Already Long, ignoring");
                                            }
                                            Side::Short => {
                                                info!("Close Short, enter Long");
                                                let account_info = account.account_info().await.unwrap();
                                                let trades = plpl_long(
                                                    &account_info,
                                                    &client_order_id,
                                                    &candle,
                                                    trailing_stop,
                                                    stop_loss_pct,
                                                    TICKER,
                                                    QUOTE_ASSET,
                                                    BASE_ASSET
                                                );
                                                for trade in trades {
                                                    let _ = account.trade::<LimitOrderResponse>(trade).await.unwrap();
                                                }
                                                info!("Long @ {} | {}", candle.close, date.to_string());
                                            }
                                        },
                                    }
                                } else if plpl_system.short_signal(curr_candle, &candle, plpl) {
                                    // if position is Short, ignore
                                    // if position is Long, close long and enter Short
                                    // if position is None, enter Short
                                    match active_order {
                                        None => {
                                            info!("No active order, enter Short");
                                            let account_info = account.account_info().await.unwrap();
                                            let trades = plpl_short(
                                                &account_info,
                                                &client_order_id,
                                                &candle,
                                                trailing_stop,
                                                stop_loss_pct,
                                                TICKER,
                                                QUOTE_ASSET,
                                                BASE_ASSET
                                            );
                                            for trade in trades {
                                                let _ = account.trade::<LimitOrderResponse>(trade).await.unwrap();
                                            }
                                            info!("Short @ {} | {}", candle.close, date.to_string());
                                        }
                                        Some(active_order) => match active_order.side {
                                            Side::Long => {
                                                info!("Close Long, enter Short");
                                                let account_info = account.account_info().await.unwrap();
                                                let trades = plpl_short(
                                                    &account_info,
                                                    &client_order_id,
                                                    &candle,
                                                    trailing_stop,
                                                    stop_loss_pct,
                                                    TICKER,
                                                    QUOTE_ASSET,
                                                    BASE_ASSET
                                                );
                                                for trade in trades {
                                                    let _ = account.trade::<LimitOrderResponse>(trade).await.unwrap();
                                                }
                                                info!("Short @ {} | {}", candle.close, date.to_string());
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
                            .duration_since(start).unwrap();
                        info!("Time to process Kline event: {:?}ms", elapsed.as_millis());
                    });
                }
                _ => (),
            };
        }
    });

    let user_stream = USER_STREAM.lock().await;
    handle_streams(
        &user_stream,
        KLINE_STREAM.to_string(),
        keep_running,
        queue_tx,
    )
    .await?;

    Ok(())
}
