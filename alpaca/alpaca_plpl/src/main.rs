// pub const ALPACA_TEST_API_KEY: &str =
//     "XEUwQO3rcZj91HOdYRekbD6RSEJb03KHogXFHR4rvILCmTqqgqjxCEpr0O25SFQs";
// pub const ALPACA_TEST_API_SECRET: &str =
//     "ld3CZkxrqsjwJi5b20aSbTVatjZcSb1J46tcF5wP043OZ4jATtwGDHiroLCllnVw";
//
// pub const ALPACA_LIVE_API_KEY: &str =
//     "WeGpjrcMfU4Yndtb8tOqy2MQouEWsGuQbCwNHOwCSKtnxm5MUhqB6EOyQ3u7rBFY";
// pub const ALPACA_LIVE_API_SECRET: &str =
//     "aLfkivKBnH31bhfcOc1P7qdg7HxLRcjCRBMDdiViVXMfO64TFEYe6V1OKr0MjyJS";

// /// The API base URL used for paper trading.
// pub(crate) const API_PAPER_URL: &str = "https://paper-api.alpaca.markets";
// /// The API base URL used for live trading.
// pub const API_LIVE_URL: &str = "https://api.alpaca.markets/";
//
// /// The HTTP header representing the key ID.
// pub(crate) const HDR_KEY_ID: &str = "APCA-API-KEY-ID";
// /// The HTTP header representing the secret key.
// pub(crate) const HDR_SECRET: &str = "APCA-API-SECRET-KEY";
//
// /// The API base URL used for retrieving market data.
// pub(crate) const DATA_HTTP_URL: &str = "https://data.alpaca.markets";
// /// The base URL for streaming market data over a websocket connection.
// pub(crate) const DATA_WS_URL: &str = "wss://stream.data.alpaca.markets";

// lazy_static! {
//     static ref ACCOUNT: Mutex<Account> = match is_testnet()
//       .expect("Failed to parse env TESTNET to boolean")
//     {
//         true => {
//             Mutex::new(Account {
//                 client: Client::new(
//                     Some(BINANCE_TEST_API_KEY.to_string()),
//                     Some(BINANCE_TEST_API_SECRET.to_string()),
//                     BINANCE_TEST_API.to_string()
//                 ),
//                 recv_window: 10000,
//                 base_asset: BASE_ASSET.to_string(),
//                 quote_asset: QUOTE_ASSET.to_string(),
//                 ticker: TICKER.to_string(),
//                 active_order: None,
//                 assets: Assets::default(),
//             })
//         },
//         false => {
//             Mutex::new(Account {
//                 client: Client::new(
//                     Some(BINANCE_LIVE_API_KEY.to_string()),
//                     Some(BINANCE_LIVE_API_SECRET.to_string()),
//                     BINANCE_LIVE_API.to_string()
//                 ),
//                 recv_window: 10000,
//                 base_asset: BASE_ASSET.to_string(),
//                 quote_asset: QUOTE_ASSET.to_string(),
//                 ticker: TICKER.to_string(),
//                 active_order: None,
//                 assets: Assets::default(),
//             })
//         }
//     };
//     static ref USER_STREAM: Mutex<UserStream> = match is_testnet()
//         .expect("Failed to parse env TESTNET to boolean")
//     {
//         true => {
//             Mutex::new(UserStream {
//                 client: Client::new(
//                     Some(BINANCE_TEST_API_KEY.to_string()),
//                     Some(BINANCE_TEST_API_SECRET.to_string()),
//                     BINANCE_TEST_API.to_string()
//                 ),
//                 recv_window: 10000,
//             })
//         },
//         false => {
//             Mutex::new(UserStream {
//                 client: Client::new(
//                     Some(BINANCE_LIVE_API_KEY.to_string()),
//                     Some(BINANCE_LIVE_API_SECRET.to_string()),
//                     BINANCE_LIVE_API.to_string()
//                 ),
//                 recv_window: 10000,
//             })
//         }
//     };
//     // cache previous and current Kline/Candle to assess PLPL trade signal
//     static ref PREV_CANDLE: Mutex<Option<Candle>> = Mutex::new(None);
//     static ref CURR_CANDLE: Mutex<Option<Candle>> = Mutex::new(None);
//     static ref COUNTER: Mutex<AtomicUsize> = Mutex::new(AtomicUsize::new(0));
// }

// Copyright (C) 2022-2023 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

#![allow(clippy::let_unit_value)]

use apca::data::v2::stream::drive;
use apca::data::v2::stream::Bar;
use apca::data::v2::stream::MarketData;
use apca::data::v2::stream::Quote;
use apca::data::v2::stream::RealtimeData;
use apca::data::v2::stream::IEX;
use apca::ApiInfo;
use apca::Client;
use apca::Error;
use std::path::PathBuf;

use chrono::DateTime;
use chrono::Utc;

use futures::FutureExt as _;
use futures::StreamExt as _;
use futures::TryStreamExt as _;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;

mod error;
mod utils;

use error::*;
use log::*;
use utils::*;

pub const ALPACA_TEST_API_KEY: &str = "PK0BAA17L2UG3CN8MJDQ";
pub const ALPACA_TEST_API_SECRET: &str = "VRBfQRPIHonueNlpuTJI97m3D08Ual0kCQLm7PB2";
pub const API_PAPER_URL: &str = "https://paper-api.alpaca.markets";

/// A trade for an equity.
///
/// This is a user-defined type with additional fields compared to
/// [`apca::data::v2::stream::Trade`] (the default), illustrating how to
/// work with custom types.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trade {
    /// The trade's symbol.
    #[serde(rename = "S")]
    pub symbol: String,
    /// The trade's ID.
    #[serde(rename = "i")]
    pub trade_id: u64,
    /// The trade's price.
    #[serde(rename = "p")]
    pub trade_price: Num,
    /// The trade's size.
    #[serde(rename = "s")]
    pub trade_size: u64,
    /// The trade's conditions.
    #[serde(rename = "c")]
    pub conditions: Vec<String>,
    /// The trade's time stamp.
    #[serde(rename = "t")]
    pub timestamp: DateTime<Utc>,
    /// The trade's exchange.
    #[serde(rename = "x")]
    pub exchange: String,
    /// The trade's tape.
    #[serde(rename = "z")]
    pub tape: String,
    /// The trade's update, may be "canceled", "corrected", or
    /// "incorrect".
    #[serde(rename = "u", default)]
    pub update: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger(&PathBuf::from("alpaca.log")).expect("Failed to init logger");

    // Requires the following environment variables to be present:
    // - APCA_API_KEY_ID -> your API key
    // - APCA_API_SECRET_KEY -> your secret key
    //
    // Optionally, the following variable is honored:
    // - APCA_API_BASE_URL -> the API base URL to use (set to
    //   https://api.alpaca.markets for live trading)
    let api_info =
        ApiInfo::from_parts(API_PAPER_URL, ALPACA_TEST_API_KEY, ALPACA_TEST_API_SECRET).unwrap();
    let client = Client::new(api_info);

    let (mut stream, mut subscription) = client
        .subscribe::<RealtimeData<IEX, Bar, Quote, Trade>>()
        .await
        .unwrap();

    let mut data = MarketData::default();
    // Subscribe to minute aggregate bars for SPY and XLK...
    data.set_bars(["BTCUSD"]);
    // ... and realtime quotes for AAPL...
    data.set_quotes(["BTCUSD"]);
    // ... and realtime trades for TSLA.
    data.set_trades(["BTCUSD"]);

    let subscribe = subscription.subscribe(&data).boxed();
    // Actually subscribe with the websocket server.
    let () = drive(subscribe, &mut stream)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let () = stream
        // Stop after receiving and printing 50 updates.
        .take(50)
        .map_err(Error::WebSocket)
        .try_for_each(|result| async {
            result.map(|data| info!("{:?}", data)).map_err(Error::Json)
        })
        .await
        .unwrap();

    // Using the provided `subscription` object we could change the
    // symbols for which to receive bars or quotes at any point.

    Ok(())
}
