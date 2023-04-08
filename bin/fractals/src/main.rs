use log::*;
// use plotters::prelude::full_palette::{BLUE, GREEN, RED};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::env;
use std::path::PathBuf;
use time_series::*;

#[tokio::main]
async fn main() {
    init_logger();

    let left_bars = env::var("LEFT_BARS")
        .expect("LEFT_BARS not set")
        .parse::<usize>()
        .expect("LEFT_BARS not a number");
    let right_bars = env::var("RIGHT_BARS")
        .expect("RIGHT_BARS not set")
        .parse::<usize>()
        .expect("RIGHT_BARS not a number");
    let pivots_back = match env::var("PIVOTS_BACK") {
        Ok(pivots_back) => pivots_back
            .parse::<usize>()
            .expect("PIVOTS_BACK not a number"),
        Err(_) => 0,
    };
    let use_time = match env::var("USE_TIME") {
        Ok(use_time) => use_time.parse::<bool>().expect("USE_TIME not a bool"),
        Err(_) => false,
    };
    let num_compare = match env::var("NUM_COMPARE") {
        Ok(num_compare) => num_compare
            .parse::<usize>()
            .expect("NUM_COMPARE not a number"),
        Err(_) => 3,
    };
    let num_forecast = match env::var("NUM_FORECAST") {
        Ok(num_forecast) => num_forecast
            .parse::<usize>()
            .expect("NUM_FORECAST not a number"),
        Err(_) => 10,
    };

    let path_to_dir = env::var("PATH_TO_DIR").expect("PATH_TO_DIR not set");

    // BTCUSD
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/BTC_daily.csv";
    let btc_1h = path_to_dir.clone() + "/data/BTCUSD/BTC_1h.csv";
    let btc_5min = path_to_dir + "data/BTCUSD/BTC_5min.csv";

    // BTC daily
    let mut ticker_data_daily = TickerData::new();
    ticker_data_daily
        .add_csv_series(&PathBuf::from(btc_daily))
        .expect("Failed to add CSV to TickerData");

    // BTC 1h
    let mut ticker_data_1h = TickerData::new();
    ticker_data_1h
        .add_csv_series(&PathBuf::from(btc_1h))
        .expect("Failed to add CSV to TickerData");

    // BTC 5min
    let mut ticker_data_5min = TickerData::new();
    ticker_data_5min
        .add_csv_series(&PathBuf::from(btc_5min))
        .expect("Failed to add CSV to TickerData");

    // stream real-time data from RapidAPI to TickerData
    // let rapid_api = RapidApi::new("BTC".to_string());
    // let candles = rapid_api.query(Interval::Daily).await;
    // ticker_data.add_series(candles).expect("Failed to add API series to TickerData");
    // write full ticker_data history to CSV

    let fractal = Fractal::new(left_bars, right_bars, use_time, pivots_back, num_compare, num_forecast);
    let all_time_series = vec![
        TimeSeries {
            series: ticker_data_daily,
            timeframe: Timeframe::Day,
        },
        TimeSeries {
            series: ticker_data_1h,
            timeframe: Timeframe::Hour,
        },
        TimeSeries {
            series: ticker_data_5min,
            timeframe: Timeframe::Min5,
        },
    ];
    fractal.fractals(all_time_series);
}

pub fn init_logger() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("failed to initialize logger");
}