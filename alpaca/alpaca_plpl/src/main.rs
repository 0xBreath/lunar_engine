#![allow(clippy::let_unit_value)]

mod endpoints;
mod error;
mod plpl;
mod utils;

use crate::plpl::*;
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
pub const ALPACA_TEST_API_KEY: &str = "PK0BAA17L2UG3CN8MJDQ";
pub const ALPACA_TEST_API_SECRET: &str = "VRBfQRPIHonueNlpuTJI97m3D08Ual0kCQLm7PB2";
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
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(&PathBuf::from("alpaca.log"))?;

    // PLPL parameters; tuned for 5 minute candles
    let trailing_take_profit = ExitType::Price(3.5);
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
        first_date: Time::new(2023, &Month::from_num(9), &Day::from_num(1), None, None),
        last_date: Time::new(2050, &Month::from_num(9), &Day::from_num(1), None, None),
        plpl_scale,
        plpl_price,
        num_plpls,
        cross_margin_pct,
    })?;

    let prev_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let curr_candle: Mutex<Option<Candle>> = Mutex::new(None);

    // Subscribe with the websocket server.
    let client = Client::new(API_INFO.clone());
    let (mut stream, mut subscription) = client
        .subscribe::<RealtimeData<CustomUrl<Crypto>>>()
        .await
        .unwrap();
    let mut data = MarketData::default();
    data.set_bars(["BTC/USD"]);
    data.set_trades(["BTC/USD"]);
    let subscribe = subscription.subscribe(&data).boxed();
    let () = drive(subscribe, &mut stream).await?.unwrap()?;

    // handle each websocket message
    let () = stream
        .map_err(AlpacaError::WebSocket)
        .try_for_each(|result| async {
            let data = result.map_err(AlpacaError::Json)?;

            let mut prev = prev_candle.lock()?;
            let mut curr = curr_candle.lock()?;

            match data {
                Data::Quote(quote) => {
                    debug!("quote: {:?}", quote);
                }
                Data::Trade(trade) => {
                    info!("{:?}", trade);
                }
                Data::Bar(bar) => {
                    debug!("bar: {:?}", bar);
                    // compute closest PLPL to current Candle
                    let candle = utils::bar_to_candle(bar);
                    // compare previous candle to current candle to check crossover of PLPL signal threshold
                    match (&*prev, &*curr) {
                        (None, None) => *prev = Some(candle),
                        (Some(prev_candle), None) => {
                            *curr = Some(candle.clone());

                            process_candle(
                                &client,
                                &plpl_system,
                                prev_candle,
                                &candle,
                                candle.date.to_unix_ms().to_string(),
                                trailing_take_profit.clone(),
                                stop_loss.clone(),
                            ).await?;
                        }
                        (None, Some(_)) => {
                            error!(
                                "🛑 Previous candle is None and current candle is Some. Should never occur."
                            );
                        }
                        (Some(_prev_candle), Some(curr_candle)) => {
                            process_candle(
                                &client,
                                &plpl_system,
                                curr_candle,
                                &candle,
                                candle.date.to_unix_ms().to_string(),
                                trailing_take_profit.clone(),
                                stop_loss.clone(),
                            ).await?;

                            *prev = Some(curr_candle.clone());
                            *curr = Some(candle);
                        }
                    }
                }
                _ => {
                    debug!("other: {:?}", data);
                }
            }
            Ok(())
        })
        .await?;

    Ok(())
}
