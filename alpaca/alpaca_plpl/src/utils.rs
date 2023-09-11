use crate::{AlpacaError, Result};
use apca::api::v2::order::Side;
use apca::data::v2::stream::Bar;
use log::*;
use num_decimal::Num;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;
use time_series::{f64_to_num, precise_round, Candle, Time};

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

#[derive(Debug, Clone)]
pub enum ExitType {
    Percent(f64),
    Price(f64),
}

impl ExitType {
    pub fn calc_exit(&self, entry_side: &Side, origin: f64) -> f64 {
        match entry_side {
            Side::Buy => match self {
                ExitType::Percent(pct) => {
                    precise_round!(origin - (origin * (*pct) / 100.0), 2)
                }
                ExitType::Price(dollars) => precise_round!(origin - dollars, 2),
            },
            Side::Sell => match self {
                ExitType::Percent(pct) => {
                    precise_round!(origin + (origin * (*pct) / 100.0), 2)
                }
                ExitType::Price(dollars) => precise_round!(origin + dollars, 2),
            },
        }
    }
}

pub struct BracketStopLoss {
    pub stop_type: ExitType,
    pub stop_price: Num,
    pub limit_price: Num,
}

impl BracketStopLoss {
    pub fn new(entry_price: f64, entry_side: Side, stop_type: ExitType) -> Self {
        let (stop_price, limit_price) = match entry_side {
            // entry is buy, so stop loss is sell
            Side::Buy => {
                let limit_price = stop_type.calc_exit(&entry_side, entry_price);
                // stop price is 75% of the way from entry to limit price
                let stop_price =
                    precise_round!(limit_price + ((limit_price - entry_price).abs() / 4.0), 2);
                (limit_price, stop_price)
            }
            // entry is sell, so stop loss is buy
            Side::Sell => {
                let limit_price = stop_type.calc_exit(&entry_side, entry_price);
                // stop price is 75% of the way from entry to limit price
                let stop_price =
                    precise_round!(limit_price - ((limit_price - entry_price).abs() / 4.0), 2);
                (limit_price, stop_price)
            }
        };
        info!("stop_price: {}, limit_price: {}", stop_price, limit_price);
        let stop_price: Num = f64_to_num!(stop_price);
        let limit_price: Num = f64_to_num!(limit_price);
        info!(
            "stop num: {}, limit num: {}",
            stop_price.to_f64().unwrap(),
            limit_price.to_f64().unwrap()
        );
        Self {
            stop_type,
            stop_price,
            limit_price,
        }
    }
}

pub struct BracketTrailingTakeProfit {
    pub trail_type: ExitType,
    pub trail_price: Option<Num>,
    pub trail_percent: Option<Num>,
}

impl BracketTrailingTakeProfit {
    pub fn new(trail_type: ExitType) -> Self {
        match trail_type {
            ExitType::Percent(pct) => {
                let trail_percent = f64_to_num!(pct);
                Self {
                    trail_type,
                    trail_price: None,
                    trail_percent: Some(trail_percent),
                }
            }
            ExitType::Price(dollars) => {
                let trail_price = f64_to_num!(dollars);
                Self {
                    trail_type,
                    trail_price: Some(trail_price),
                    trail_percent: None,
                }
            }
        }
    }
}
