use ephemeris::*;
use lazy_static::lazy_static;
use library::*;
use log::*;
use model::Assets;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use time_series::{precise_round, Candle, Day, Month, Time};

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
                recv_window: 10000,
                base_asset: BASE_ASSET.to_string(),
                quote_asset: QUOTE_ASSET.to_string(),
                ticker: TICKER.to_string(),
                active_order: None,
                assets: Assets::default(),
            })
        },
        false => {
            Mutex::new(Account {
                client: Client::new(
                    Some(BINANCE_LIVE_API_KEY.to_string()),
                    Some(BINANCE_LIVE_API_SECRET.to_string()),
                    BINANCE_LIVE_API.to_string()
                ),
                recv_window: 10000,
                base_asset: BASE_ASSET.to_string(),
                quote_asset: QUOTE_ASSET.to_string(),
                ticker: TICKER.to_string(),
                active_order: None,
                assets: Assets::default(),
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
    let trailing_take_profit = ExitType::Fixed(350);
    let stop_loss = ExitType::Percent(0.05);
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
    account.cancel_all_open_orders()?;
    // equalize base and quote assets to 50/50
    account.equalize_assets()?;

    let mut ws = WebSockets::new(testnet, |event: WebSocketEvent| {
        let now = SystemTime::now();
        // check if timestamp is 10 minutes after last UserStream keep alive ping
        let secs_since_keep_alive = now
            .duration_since(user_stream_keep_alive_time)
            .map(|d| d.as_secs())
            .map_err(|e| BinanceError::Custom(e.to_string()))?;

        if secs_since_keep_alive > 30 * 60 {
            match user_stream.keep_alive(&listen_key) {
                Ok(_) => {
                    let now = Time::from_unix_msec(
                        now.duration_since(UNIX_EPOCH).unwrap().as_millis() as i64,
                    );
                    info!("Keep alive user stream @ {}", now.to_string())
                }
                Err(e) => error!("Error on user stream keep alive: {}", e),
            }
            user_stream_keep_alive_time = now;
        }

        match event {
            WebSocketEvent::Kline(kline_event) => {
                let kline_event_time = kline_event.event_time as i64;
                let date = Time::from_unix_msec(kline_event_time);
                let timestamp = format!("{}", kline_event_time);
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
                    .map_err(BinanceError::PLPL)?;
                // active order bundle on Binance
                let active_order = account.active_order();
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
                            timestamp,
                            &mut account,
                            active_order,
                            trailing_take_profit.clone(),
                            stop_loss.clone(),
                        )?;
                    }
                    (None, Some(_)) => {
                        error!(
                            "Previous candle is None and current candle is Some. Should never occur."
                        );
                    }
                    (Some(_prev_candle), Some(curr_candle)) => {
                        trade_placed = handle_signal(
                            &plpl_system,
                            plpl,
                            curr_candle,
                            &candle,
                            &date,
                            timestamp,
                            &mut account,
                            active_order,
                            trailing_take_profit.clone(),
                            stop_loss.clone(),
                        )?;
                        *prev = Some(curr_candle.clone());
                        *curr = Some(candle);
                    }
                }
                // time to process
                let elapsed = SystemTime::now().duration_since(now).map_err(|e| {
                    BinanceError::Custom(format!("Failed to get duration since: {}", e))
                })?;
                if trade_placed {
                    debug!("Time to process PLPL trade: {:?}ms", elapsed.as_millis());
                }
            }
            WebSocketEvent::AccountUpdate(account_update) => {
                let assets = account_update.assets(&account.quote_asset, &account.base_asset)?;
                debug!(
                    "Account Update, {}: {}, {}: {}",
                    account.quote_asset, assets.free_quote, account.base_asset, assets.free_base
                );
            }
            WebSocketEvent::OrderTrade(event) => {
                let order_type = OrderBundle::client_order_id_suffix(&event.new_client_order_id);
                let entry_price = precise_round!(
                    event
                        .price
                        .parse::<f64>()
                        .map_err(BinanceError::ParseFloat)?,
                    2
                );
                debug!(
                    "{},  {},  {} @ {},  Execution: {},  Status: {},  Order: {}",
                    event.symbol,
                    event.new_client_order_id,
                    event.side,
                    entry_price,
                    event.execution_type,
                    event.order_status,
                    order_type
                );
                return match account.update_active_order(event) {
                    Ok(_) => {
                        account.log_active_order();
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
        return Err(e);
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
            warn!("Binance websocket disconnected");
            Ok(())
        }
    }
}
