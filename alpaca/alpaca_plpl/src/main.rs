#![allow(clippy::let_unit_value)]
#![allow(dead_code)]

mod endpoints;
mod error;
mod plpl;
mod utils;

use crate::plpl::*;
use apca::api::v2::updates::OrderUpdates;
use apca::data::v2::stream::RealtimeData;
use apca::data::v2::stream::{drive, Data};
use apca::data::v2::stream::{CustomUrl, MarketData};
use apca::ApiInfo;
use apca::Client;
use endpoints::*;
use ephemeris::*;
use error::*;
use futures::FutureExt as _;
use futures::TryStreamExt as _;
use lazy_static::lazy_static;
use log::*;
use std::path::PathBuf;
use std::sync::Mutex;
use time_series::{Candle, Day, Month, Time};
use utils::*;

/// Paper trading API credentials
pub const ALPACA_TEST_API_KEY: &str = "PKYIM924ABM29BNIRAXA";
pub const ALPACA_TEST_API_SECRET: &str = "gFedYbEywA95f41z5k7VTDkikAAYsZbnGgebfXHX";
/// Live trading API credentials
pub const ALPACA_LIVE_API_KEY: &str = "AK4ZHDVHCN9AZJSLKXET";
pub const ALPACA_LIVE_API_SECRET: &str = "9K0AZhmryDkiKzhI32xg8UvbbPs325MiAcu8pjhY";

lazy_static! {
    static ref API_INFO: ApiInfo =
        match is_testnet().expect("Failed to parse env TESTNET to boolean") {
            true => {
                ApiInfo::from_parts(
                    ALPACA_API_PAPER_URL,
                    ALPACA_TEST_API_KEY,
                    ALPACA_TEST_API_SECRET,
                ).unwrap()
            }
            false => {
                ApiInfo::from_parts(
                    ALPACA_API_LIVE_URL,
                    ALPACA_LIVE_API_KEY,
                    ALPACA_LIVE_API_SECRET,
                ).unwrap()
            }
        };
    // cache previous and current Kline/Candle to assess PLPL trade signal
    static ref PREV_CANDLE: Mutex<Option<Candle>> = Mutex::new(None);
    static ref CURR_CANDLE: Mutex<Option<Candle>> = Mutex::new(None);
    static ref TICKER: String = "BTC/USD".to_string();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(&PathBuf::from("alpaca.log"))?;

    // PLPL parameters; tuned for 5 minute candles
    let trailing_take_profit = ExitType::Percent(0.5);
    let stop_loss = ExitType::Percent(0.5);
    let planet = Planet::from("Jupiter");
    let plpl_scale = 0.5;
    let plpl_price = 400.0;
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

    let client = Client::new(API_INFO.clone());

    // PLPL engine
    let engine = Engine::new(
        client,
        TICKER.clone(),
        plpl_system,
        trailing_take_profit,
        stop_loss,
    );

    // Subscribe to websocket bar updates.
    let (mut stream, mut subscription) = engine
        .client
        .subscribe::<RealtimeData<CustomUrl<Crypto>>>()
        .await?;
    let mut data = MarketData::default();
    data.set_bars([TICKER.as_str()]);
    let subscribe = subscription.subscribe(&data).boxed();
    let () = drive(subscribe, &mut stream).await?.unwrap()?;

    // Subscribe to websocket order updates.
    let (stream_orders, _subscription_orders) = engine.client.subscribe::<OrderUpdates>().await?;

    let engine = Mutex::new(engine);

    // handle websocket order updates
    let () = stream_orders
        .map_err(AlpacaError::WebSocket)
        .try_for_each(|result| async {
            let data = result.map_err(AlpacaError::Json)?;
            info!("{:?}", data.order.id);
            let suffix = order_id_suffix(&data.order);
            let mut engine = engine.lock()?;
            if suffix == "ENTRY" || suffix == "TAKE_PROFIT" || suffix == "STOP_LOSS" {
                engine.update_active_order(data)?;
                engine.check_active_order().await?;
            }
            Ok(())
        })
        .await?;

    // handle websocket bar updates
    let () = stream
        .map_err(AlpacaError::WebSocket)
        .try_for_each(|result| async {
            let data = result.map_err(AlpacaError::Json)?;

            let mut prev = PREV_CANDLE.lock()?;
            let mut curr = CURR_CANDLE.lock()?;
            let mut engine = engine.lock()?;

            match data {
                Data::Quote(quote) => {
                    debug!("quote: {:?}", quote);
                }
                Data::Trade(trade) => {
                    debug!("{:?}", trade);
                }
                Data::Bar(bar) => {
                    debug!("bar: {:?}", bar);
                    // compute closest PLPL to current Candle
                    let candle = bar_to_candle(bar);
                    // compare previous candle to current candle to check crossover of PLPL signal threshold
                    match (&*prev, &*curr) {
                        (None, None) => *prev = Some(candle),
                        (Some(prev_candle), None) => {
                            *curr = Some(candle.clone());

                            engine.process_candle(
                                prev_candle,
                                &candle,
                                candle.date.to_unix_ms().to_string(),
                            ).await?;
                        }
                        (None, Some(_)) => {
                            error!(
                                "🛑 Previous candle is None and current candle is Some. Should never occur."
                            );
                        }
                        (Some(_prev_candle), Some(curr_candle)) => {
                            engine.process_candle(
                                curr_candle,
                                &candle,
                                candle.date.to_unix_ms().to_string(),
                            ).await?;

                            *prev = Some(curr_candle.clone());
                            *curr = Some(candle);
                        }
                    }
                }
                _ => {
                    debug!("Other websocket data: {:?}", data);
                }
            }
            Ok(())
        })
        .await?;

    Ok(())
}
