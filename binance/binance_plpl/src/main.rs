use binance_lib::*;
use ephemeris::*;
use lazy_static::lazy_static;
use log::*;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use time_series::{precise_round, Day, Month, Time};

mod engine;
mod utils;
use engine::*;
use utils::*;

// Binance Spot Test Network API credentials
#[allow(dead_code)]
pub const BINANCE_TEST_API: &str = "https://testnet.binance.vision";
#[allow(dead_code)]
pub const BINANCE_TEST_API_KEY: &str =
    "XEUwQO3rcZj91HOdYRekbD6RSEJb03KHogXFHR4rvILCmTqqgqjxCEpr0O25SFQs";
#[allow(dead_code)]
pub const BINANCE_TEST_API_SECRET: &str =
    "ld3CZkxrqsjwJi5b20aSbTVatjZcSb1J46tcF5wP043OZ4jATtwGDHiroLCllnVw";
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
    static ref USER_STREAM: Mutex<UserStream> =
        match is_testnet().expect("Failed to parse env TESTNET to boolean") {
            true => {
                Mutex::new(UserStream {
                    client: Client::new(
                        Some(BINANCE_TEST_API_KEY.to_string()),
                        Some(BINANCE_TEST_API_SECRET.to_string()),
                        BINANCE_TEST_API.to_string(),
                    ),
                    recv_window: 10000,
                })
            }
            false => {
                Mutex::new(UserStream {
                    client: Client::new(
                        Some(BINANCE_LIVE_API_KEY.to_string()),
                        Some(BINANCE_LIVE_API_SECRET.to_string()),
                        BINANCE_LIVE_API.to_string(),
                    ),
                    recv_window: 10000,
                })
            }
        };
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(&PathBuf::from("plpl.log".to_string()))?;
    info!("Starting Binance PLPL!");

    // PLPL parameters; tuned for 5 minute candles
    let trailing_take_profit = ExitType::Ticks(350);
    let stop_loss = ExitType::Bips(5);
    let planet = Planet::from("Jupiter");
    let plpl_scale = 0.5;
    let plpl_price = 20000.0;
    let num_plpls = 8000;
    let cross_margin_pct = 55.0;

    // initialize PLPL
    let plpl_system = PLPLSystem::new(PLPLSystemConfig {
        planet,
        origin: Origin::Heliocentric,
        first_date: Time::new(2023, &Month::from_num(9), &Day::from_num(1), None, None),
        last_date: Time::new(2050, &Month::from_num(9), &Day::from_num(1), None, None),
        plpl_scale,
        plpl_price,
        num_plpls,
        cross_margin_pct,
    })?;

    let testnet = is_testnet()?;

    let mut engine = match testnet {
        true => Engine::new(
            Client::new(
                Some(BINANCE_TEST_API_KEY.to_string()),
                Some(BINANCE_TEST_API_SECRET.to_string()),
                BINANCE_TEST_API.to_string(),
            ),
            plpl_system,
            10000,
            BASE_ASSET.to_string(),
            QUOTE_ASSET.to_string(),
            TICKER.to_string(),
            trailing_take_profit,
            stop_loss,
        ),
        false => Engine::new(
            Client::new(
                Some(BINANCE_LIVE_API_KEY.to_string()),
                Some(BINANCE_LIVE_API_SECRET.to_string()),
                BINANCE_LIVE_API.to_string(),
            ),
            plpl_system,
            10000,
            BASE_ASSET.to_string(),
            QUOTE_ASSET.to_string(),
            TICKER.to_string(),
            trailing_take_profit,
            stop_loss,
        ),
    };

    let user_stream_keep_alive_time = Mutex::new(SystemTime::now());
    let user_stream = USER_STREAM.lock()?;
    let answer = user_stream.start()?;
    let listen_key = Mutex::new(answer.listen_key);

    // cancel all open orders to start with a clean slate
    engine.cancel_all_open_orders()?;
    // equalize base and quote assets to 50/50
    engine.equalize_assets()?;
    // get initial asset balances
    engine.update_assets()?;
    engine.log_assets();

    let engine = Mutex::new(engine);
    let mut ws = WebSockets::new(testnet, |event: WebSocketEvent| {
        let now = SystemTime::now();
        let mut keep_alive = user_stream_keep_alive_time.lock()?;
        let mut listen_key = listen_key.lock()?;
        // check if timestamp is 10 minutes after last UserStream keep alive ping
        let secs_since_keep_alive = now.duration_since(*keep_alive).map(|d| d.as_secs())?;

        if secs_since_keep_alive > 30 * 60 {
            match user_stream.keep_alive(&listen_key) {
                Ok(_) => {
                    let now = Time::from_unix_msec(
                        now.duration_since(UNIX_EPOCH).unwrap().as_millis() as i64,
                    );
                    info!("Keep alive user stream @ {}", now.to_string())
                }
                Err(e) => error!("🛑 Error on user stream keep alive: {}", e),
            }
            *keep_alive = now;
        }

        let mut engine = engine.lock()?;

        match event {
            WebSocketEvent::Kline(kline_event) => {
                let candle = kline_to_candle(&kline_event)?;

                // compare previous candle to current candle to check crossover of PLPL signal threshold
                match (&engine.prev_candle.clone(), &engine.candle.clone()) {
                    (None, None) => engine.prev_candle = Some(candle),
                    (Some(prev_candle), None) => {
                        engine.candle = Some(candle.clone());
                        engine.process_candle(prev_candle, &candle)?;
                    }
                    (None, Some(_)) => {
                        error!(
                            "🛑 Previous candle is None and current candle is Some. Should never occur."
                        );
                    }
                    (Some(_prev_candle), Some(curr_candle)) => {
                        engine.process_candle(curr_candle, &candle)?;
                        engine.prev_candle = Some(curr_candle.clone());
                        engine.candle = Some(candle);
                    }
                }
            }
            WebSocketEvent::AccountUpdate(account_update) => {
                let assets = account_update.assets(&engine.quote_asset, &engine.base_asset)?;
                debug!(
                    "Account Update, {}: {}, {}: {}",
                    engine.quote_asset, assets.free_quote, engine.base_asset, assets.free_base
                );
            }
            WebSocketEvent::OrderTrade(event) => {
                let order_type = ActiveOrder::client_order_id_suffix(&event.new_client_order_id);
                let entry_price = precise_round!(event.price.parse::<f64>()?, 2);
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
                // update state
                engine.update_active_order(event)?;
                // create or cancel orders depending on state
                engine.check_active_order()?;
                // check trailing take profit and update if necessary
                engine.check_trailing_take_profit()?;
            }
            _ => (),
        };
        Ok(())
    });

    let listen_key_lock = listen_key.lock()?;
    let subs = vec![KLINE_STREAM.to_string(), listen_key_lock.clone()];
    match ws.connect_multiple_streams(&subs, testnet) {
        Err(e) => {
            error!("🛑 Failed to connect to Binance websocket: {}", e);
            return Err(e);
        }
        Ok(_) => info!("Binance websocket connected"),
    }

    if let Err(e) = ws.event_loop(&AtomicBool::new(true)) {
        error!("🛑 Binance websocket error: {}", e);
        return Err(e);
    }

    user_stream.close(&listen_key_lock)?;

    match ws.disconnect() {
        Err(e) => {
            error!("🛑 Failed to disconnect from Binance websocket: {}", e);
            match ws.connect_multiple_streams(&subs, testnet) {
                Err(e) => {
                    error!("🛑 Failed to connect to Binance websocket: {}", e);
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
