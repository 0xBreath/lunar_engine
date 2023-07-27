mod errors;
use errors::*;

use apca::data::v2::stream::{drive, Bar, MarketData, Quote, RealtimeData, Trade, IEX};
use apca::{ApiInfo, Client};
use futures::FutureExt as _;
use futures::StreamExt as _;
use futures::TryStreamExt as _;
use log::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;

pub const APCA_PAPER_API_URL: &str = "https://paper-api.alpaca.markets";
pub const APCA_LIVE_API_URL: &str = "https://api.alpaca.markets";
pub const APCA_API_KEY_ID: &str = "PK4AAQ3LS3JNL2MZVZN9";
pub const APCA_API_SECRET_KEY: &str = "LaGuKhwyuh9fVVPBoSaypERtTZDb5j54qSVZDR9J";

#[tokio::main]
async fn main() -> AlpacaResult<()> {
    let log_file = std::env::var("LOG_FILE").unwrap_or("plpl_alpaca.log".to_string());
    init_logger(&PathBuf::from(log_file));

    let api_info = ApiInfo::from_parts(APCA_PAPER_API_URL, APCA_API_KEY_ID, APCA_API_SECRET_KEY)
        .map_err(AlpacaError::AlpacaError)?;
    let client = Client::new(api_info);

    let (mut stream, mut subscription) = client
        .subscribe::<RealtimeData<IEX, Bar, Quote, Trade>>()
        .await
        .unwrap();

    let mut data = MarketData::default();
    // Subscribe to minute aggregate bars for SPX
    data.set_bars(["SPY"]);

    let subscribe = subscription.subscribe(&data).boxed();
    // Actually subscribe with the websocket server.
    let () = drive(subscribe, &mut stream)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    stream
        // Stop after receiving and printing 50 updates.
        .take(50)
        .map_err(apca::Error::WebSocket)
        .try_for_each(|result| async {
            result
                .map(|data| info!("Data: {data:?}"))
                .map_err(apca::Error::Json)
        })
        .await
        .unwrap();

    Ok(())
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
            File::create(log_file).expect("Failed to create PLPL Alpaca log file"),
        ),
    ])
    .expect("Failed to initialize logger");
}
