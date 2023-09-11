use crate::{AlpacaError, Result};
use apca::data::v2::stream::Bar;
use log::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;
use time_series::{Candle, Time};

pub fn init_logger(log_file: &PathBuf) -> Result<()> {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            SimpleLogConfig::default(),
            TerminalMode::Mixed,
            ColorChoice::Always,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            ConfigBuilder::new().set_time_format_rfc3339().build(),
            File::create(log_file)?,
        ),
    ])
    .map_err(AlpacaError::Logger)
}

pub fn bar_to_candle(bar: Bar) -> Candle {
    Candle {
        date: Time::from_datetime(bar.timestamp),
        open: bar.open_price.to_f64().unwrap(),
        high: bar.high_price.to_f64().unwrap(),
        low: bar.low_price.to_f64().unwrap(),
        close: bar.close_price.to_f64().unwrap(),
        volume: None,
    }
}

pub fn is_testnet() -> Result<bool> {
    std::env::var("TESTNET")?
        .parse::<bool>()
        .map_err(AlpacaError::ParseBool)
}
