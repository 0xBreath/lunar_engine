#![allow(clippy::let_unit_value)]

mod error;
mod plpl;
mod utils;

use apca::data::v2::stream::drive;
use apca::data::v2::stream::RealtimeData;
use apca::data::v2::stream::{CustomUrl, MarketData};
use apca::ApiInfo;
use apca::Client;
use apca::Error;
use error::*;
use futures::FutureExt as _;
use futures::TryStreamExt as _;
use log::*;
use std::path::PathBuf;
use utils::*;

/// Paper trading API credentials
pub const ALPACA_TEST_API_KEY: &str = "PK0BAA17L2UG3CN8MJDQ";
pub const ALPACA_TEST_API_SECRET: &str = "VRBfQRPIHonueNlpuTJI97m3D08Ual0kCQLm7PB2";
pub const ALPACA_API_PAPER_URL: &str = "https://paper-api.alpaca.markets";
/// Live trading API credentials
pub const ALPACA_LIVE_API_KEY: &str = "AK4ZHDVHCN9AZJSLKXET";
pub const ALPACA_LIVE_API_SECRET: &str = "9K0AZhmryDkiKzhI32xg8UvbbPs325MiAcu8pjhY";
pub const ALPACA_API_LIVE_URL: &str = "https://api.alpaca.markets";
/// Data API endpoints (paper or live)
#[allow(dead_code)]
pub const DATA_HTTP_URL: &str = "https://data.alpaca.markets";
#[allow(dead_code)]
pub const DATA_WS_URL: &str = "wss://stream.data.alpaca.markets";

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(&PathBuf::from("alpaca.log"))?;

    let api_info = ApiInfo::from_parts(
        ALPACA_API_LIVE_URL,
        ALPACA_LIVE_API_KEY,
        ALPACA_LIVE_API_SECRET,
    )?;
    let client = Client::new(api_info);

    let (mut stream, mut subscription) = client
        .subscribe::<RealtimeData<CustomUrl<Crypto>>>()
        .await
        .unwrap();
    let mut data = MarketData::default();
    data.set_bars(["BTC/USD"]);

    let subscribe = subscription.subscribe(&data).boxed();
    // Actually subscribe with the websocket server.
    let () = drive(subscribe, &mut stream)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let read = stream
        .map_err(Error::WebSocket)
        .try_for_each(|result| async {
            let data = result.map_err(Error::Json)?;

            match plpl::handle_stream(data) {
                Ok(()) => {}
                Err(e) => error!("error: {}", e),
            }

            Ok(())
        });
    info!("Starting stream...");

    match read.await {
        Ok(()) => info!("done"),
        Err(e) => error!("error: {}", e),
    };

    Ok(())
}
