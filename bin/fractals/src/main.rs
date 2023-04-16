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
    #[allow(unused_variables)]
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/input/BTC_daily.csv";
    #[allow(unused_variables)]
    let btc_1h = path_to_dir.clone() + "/data/BTCUSD/input/BTC_1h.csv";
    #[allow(unused_variables)]
    let btc_5min = path_to_dir.clone() + "data/BTCUSD/input/BTC_5min.csv";

    // SPX
    let spx_daily = path_to_dir.clone() + "/data/SPX/input/SPX_daily.csv";
    let spx_1h = path_to_dir.clone() + "/data/SPX/input/SPX_1h.csv";
    let spx_5min = path_to_dir.clone() + "/data/SPX/input/SPX_5min.csv";
    let spx_1month = path_to_dir + "/data/SPX/input/SPX_1month.csv";

    // btcusd(
    //     &PathBuf::from(btc_daily),
    //     &PathBuf::from(btc_1h),
    //     &PathBuf::from(btc_5min),
    //     left_bars,
    //     right_bars,
    //     pivots_back,
    //     use_time,
    //     num_compare,
    //     num_forecast,
    // );

    spx(
        &PathBuf::from(spx_daily),
        &PathBuf::from(spx_1h),
        &PathBuf::from(spx_5min),
        &PathBuf::from(spx_1month),
        left_bars,
        right_bars,
        pivots_back,
        use_time,
        num_compare,
        num_forecast,
    ).await;
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

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn btcusd(
    btc_daily: &PathBuf,
    btc_1h: &PathBuf,
    btc_5min: &PathBuf,
    left_bars: usize,
    right_bars: usize,
    pivots_back: usize,
    use_time: bool,
    num_compare: usize,
    num_forecast: usize,
) {
    // BTC daily
    let mut ticker_data_daily = TickerData::new();
    ticker_data_daily
      .add_csv_series(&PathBuf::from(btc_daily))
      .expect("Failed to add BTC daily CSV to TickerData");

    // BTC 1h
    let mut ticker_data_1h = TickerData::new();
    ticker_data_1h
      .add_csv_series(&PathBuf::from(btc_1h))
      .expect("Failed to add BTC 1 hour CSV to TickerData");

    // BTC 5min
    let mut ticker_data_5min = TickerData::new();
    ticker_data_5min
      .add_csv_series(&PathBuf::from(btc_5min))
      .expect("Failed to add BTC 5 minute CSV to TickerData");

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

#[allow(clippy::too_many_arguments)]
async fn spx(
    spx_daily_csv: &PathBuf,
    spx_1h_csv: &PathBuf,
    spx_5min_csv: &PathBuf,
    spx_1month_csv: &PathBuf,
    left_bars: usize,
    right_bars: usize,
    pivots_back: usize,
    use_time: bool,
    num_compare: usize,
    num_forecast: usize,
) {
    let mut spx_daily = TickerData::new();
    spx_daily.build_series(
        "SPX",
        Interval::Daily,
        &PathBuf::from(spx_daily_csv),
    ).await.expect("Failed to add SPX daily CSV series");

    let mut spx_1h = TickerData::new();
    spx_1h.build_series(
        "SPX",
        Interval::OneHour,
        &PathBuf::from(spx_1h_csv),
    ).await.expect("Failed to add SPX 1 hour CSV series");

    let mut spx_5min = TickerData::new();
    spx_5min.build_series(
        "SPX",
        Interval::FiveMinutes,
        &PathBuf::from(spx_5min_csv),
    ).await.expect("Failed to add SPX 5 minute CSV series");

    let mut spx_1month = TickerData::new();
    spx_1month.build_series(
        "SPX",
        Interval::Monthly,
        &PathBuf::from(spx_1month_csv),
    ).await.expect("Failed to add SPX 1 month CSV series");

    let fractal = Fractal::new(left_bars, right_bars, use_time, pivots_back, num_compare, num_forecast);
    let all_time_series = vec![
        TimeSeries {
            series: spx_daily,
            timeframe: Timeframe::Day,
        },
        TimeSeries {
            series: spx_1h,
            timeframe: Timeframe::Hour,
        },
        TimeSeries {
            series: spx_5min,
            timeframe: Timeframe::Min5,
        },
        TimeSeries {
            series: spx_1month,
            timeframe: Timeframe::Month,
        },
    ];
    fractal.fractals(all_time_series);
}