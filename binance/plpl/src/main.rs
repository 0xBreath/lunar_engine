#[macro_use]
extern crate lazy_static;

use ephemeris::*;
use library::*;
use log::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use time_series::{Candle, Day, Month, Time};

mod utils;
use utils::*;

// Binance Spot Test Network API credentials
#[allow(dead_code)]
pub const BINANCE_TEST_API: &str = "https://testnet.binance.vision";
#[allow(dead_code)]
pub const BINANCE_TEST_API_KEY: &str =
    "AekFIdmCDmPkaeQjCjaPtEE9IvYtpoceePvvelkthAh7tEtvMAm7oHzcxkhbmxl0";
#[allow(dead_code)]
pub const BINANCE_TEST_API_SECRET: &str =
    "epU83XZHBcHuvznmccDQCbCcxbGeVq6sl4AspOyALCTqWkeG1CVlJx6BzXIC2wXK";
// Binance Spot Live Network API credentials
#[allow(dead_code)]
pub const BINANCE_LIVE_API: &str = "https://api.binance.us";
#[allow(dead_code)]
pub const BINANCE_LIVE_API_KEY: &str =
    "WeGpjrcMfU4Yndtb8tOqy2MQouEWsGuQbCwNHOwCSKtnxm5MUhqB6EOyQ3u7rBFY";
#[allow(dead_code)]
pub const BINANCE_LIVE_API_SECRET: &str =
    "aLfkivKBnH31bhfcOc1P7qdg7HxLRcjCRBMDdiViVXMfO64TFEYe6V1OKr0MjyJS";
pub const KLINE_STREAM: &str = "btcusdt@kline_5m";
pub const BASE_ASSET: &str = "BTC";
pub const QUOTE_ASSET: &str = "USDT";
pub const TICKER: &str = "BTCUSDT";

lazy_static! {
    static ref ACCOUNT: Mutex<Account> = match std::env::var("TESTNET")
      .expect("ACCOUNT init failed. TESTNET environment variable must be set to either true or false")
      .parse::<bool>()
      .expect("Failed to parse env TESTNET to boolean")
    {
        true => {
            Mutex::new(Account {
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
            })
        },
        false => {
            Mutex::new(Account {
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
            })
        }
    };
    static ref USER_STREAM: Mutex<UserStream> = match std::env::var("TESTNET")
        .expect("USER_STREAM init failed. TESTNET environment variable must be set to either true or false")
        .parse::<bool>()
        .expect("Failed to parse env TESTNET to boolean")
    {
        true => {
            Mutex::new(UserStream {
                client: Client::new(
                    Some(BINANCE_TEST_API_KEY.to_string()),
                    Some(BINANCE_TEST_API_SECRET.to_string()),
                    BINANCE_TEST_API.to_string()
                ),
                recv_window: 10000,
            })
        },
        false => {
            Mutex::new(UserStream {
                client: Client::new(
                    Some(BINANCE_LIVE_API_KEY.to_string()),
                    Some(BINANCE_LIVE_API_SECRET.to_string()),
                    BINANCE_LIVE_API.to_string()
                ),
                recv_window: 10000,
            })
        }
    };
    // cache previous and current Kline/Candle to assess PLPL trade signal
    static ref PREV_CANDLE: Mutex<Option<Candle>> = Mutex::new(None);
    static ref CURR_CANDLE: Mutex<Option<Candle>> = Mutex::new(None);
    static ref COUNTER: Mutex<AtomicUsize> = Mutex::new(AtomicUsize::new(0));
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(&PathBuf::from("plpl.log".to_string()))?;
    info!("Starting Binance PLPL!");

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

    let testnet = std::env::var("TESTNET")
        .map_err(|_| {
            BinanceError::Custom(
                "Failed to read TESTNET env. Must be set to either true or false".to_string(),
            )
        })?
        .parse::<bool>()
        .map_err(|_| BinanceError::Custom("Failed to parse env TESTNET to boolean".to_string()))?;
    let prev_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let curr_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let mut account = ACCOUNT
        .lock()
        .map_err(|e| BinanceError::Custom(e.to_string()))?;

    let mut user_stream_keep_alive_time = SystemTime::now();
    let user_stream = USER_STREAM
        .lock()
        .map_err(|e| BinanceError::Custom(e.to_string()))?;
    let answer = user_stream.start()?;
    let listen_key = answer.listen_key;

    // cancel all open orders to start with a clean slate
    let reset_orders = account.cancel_all_active_orders();
    if let Err(e) = reset_orders {
        if let BinanceError::Binance(err) = &e {
            if err.code != -2011 {
                return Err(e);
            }
        }
    }

    let mut ws = WebSockets::new(testnet, |event: WebSocketEvent| {
        let start = SystemTime::now();
        // check if timestamp is 30 minutes after UserStream last keep alive ping
        let secs_since_keep_alive = start
            .duration_since(user_stream_keep_alive_time)
            .map(|d| d.as_secs())
            .map_err(|e| BinanceError::Custom(e.to_string()))?;

        if secs_since_keep_alive > 30 * 60 {
            let now =
                Time::from_unix_msec(start.duration_since(UNIX_EPOCH).unwrap().as_millis() as i64);
            match user_stream.keep_alive(&listen_key) {
                Ok(_) => info!("Keep alive UserStream @ {}", now.to_string()),
                Err(e) => error!("Error keeping alive UserStream: {}", e),
            }
            user_stream_keep_alive_time = start;
        }

        match event {
            WebSocketEvent::Kline(kline_event) => {
                let kline_event_time = kline_event.event_time as i64;
                let date = Time::from_unix_msec(kline_event_time);
                let client_order_id = format!("{}", kline_event_time);
                let candle = kline_to_candle(&kline_event)?;
                let mut prev = prev_candle.lock().map_err(|_| {
                    BinanceError::Custom("Failed to lock previous candle".to_string())
                })?;
                let mut curr = curr_candle.lock().map_err(|_| {
                    BinanceError::Custom("Failed to lock current candle".to_string())
                })?;

                // compute closest PLPL to current Candle
                let plpl = plpl_system
                    .closest_plpl(&candle)
                    .map_err(|_| BinanceError::Custom("Closest PLPL not found".to_string()))?;
                // active order bundle on Binance
                let active_order = account.get_active_order();
                let mut trade_placed = false;

                // compare previous candle to current candle to check crossover of PLPL signal threshold
                match (&*prev, &*curr) {
                    (None, None) => *prev = Some(candle),
                    (Some(prev_candle), None) => {
                        *curr = Some(candle.clone());
                        trade_placed = handle_signal(
                            &plpl_system,
                            plpl,
                            prev_candle,
                            &candle,
                            &date,
                            client_order_id,
                            &mut account,
                            active_order,
                            trailing_stop,
                            stop_loss_pct,
                        )?;
                    }
                    (None, Some(_)) => {
                        error!(
                            "Previous candle is None and current candle is Some. Should never occur!"
                        );
                    }
                    (Some(_prev_candle), Some(curr_candle)) => {
                        trade_placed = handle_signal(
                            &plpl_system,
                            plpl,
                            curr_candle,
                            &candle,
                            &date,
                            client_order_id,
                            &mut account,
                            active_order,
                            trailing_stop,
                            stop_loss_pct,
                        )?;
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
                    "{},  {},  {} @ {},  Execution: {},  Status: {},  Order: {}",
                    event.symbol,
                    event.new_client_order_id,
                    event.side,
                    event.price,
                    event.execution_type,
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
    match ws.connect_multiple_streams(&subs, testnet) {
        Err(e) => {
            error!("Failed to connect to Binance websocket: {}", e);
            return Err(e);
        }
        Ok(_) => info!("Binance websocket connected"),
    }

    if let Err(e) = ws.event_loop(&AtomicBool::new(true)) {
        error!("Binance websocket error: {}", e);
    }

    user_stream.close(&listen_key)?;

    match ws.disconnect() {
        Err(e) => {
            error!("Failed to disconnect from Binance websocket: {}", e);
            match ws.connect_multiple_streams(&subs, testnet) {
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
