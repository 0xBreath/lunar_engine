use log::*;
use plotters::prelude::full_palette::BLUE;
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
    let hda_margin = env::var("HDA_MARGIN")
      .expect("HDA_MARGIN not set")
      .parse::<usize>()
      .expect("HDA_MARGIN not a number");
    let start_year = env::var("START_YEAR")
      .expect("START_YEAR not set")
      .parse::<i32>()
      .expect("START_YEAR not a number");
    let start_month = env::var("START_MONTH")
      .expect("START_MONTH not set")
      .parse::<u32>()
      .expect("START_MONTH not a number");
    let start_day = env::var("START_DAY")
      .expect("START_DAY not set")
      .parse::<u32>()
      .expect("START_DAY not a number");
    let end_year = env::var("END_YEAR")
      .expect("END_YEAR not set")
      .parse::<i32>()
      .expect("END_YEAR not a number");
    let end_month = env::var("END_MONTH")
      .expect("END_MONTH not set")
      .parse::<u32>()
      .expect("END_MONTH not a number");
    let end_day = env::var("END_DAY")
      .expect("END_DAY not set")
      .parse::<u32>()
      .expect("END_DAY not a number");
    let path_to_dir = env::var("PATH_TO_DIR").expect("PATH_TO_DIR not set");

    // SPX
    let spx_daily = path_to_dir.clone() + "/data/SPX/input/SPX_daily.csv";
    let spx_history = path_to_dir.clone() + "/data/SPX/output/SPX_history.csv";
    let spx_hda_file = path_to_dir.clone() + "/data/SPX/output/SPX_hda.png";
    // BTCUSD
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/input/BTC_daily.csv";
    #[allow(unused_variables)]
    let btc_history = path_to_dir.clone() + "/data/BTCUSD/output/BTC_history.csv";
    let btc_hda_file = path_to_dir.clone() + "/data/BTCUSD/output/BTC_hda.png";

    let start_date = Time::new(start_year, &Month::from_num(start_month), &Day::from_num(start_day), None, None);
    let end_date = Time::new(end_year, &Month::from_num(end_month), &Day::from_num(end_day), None, None);

    let mut btc_ticker_data = TickerData::new();
    btc_ticker_data.add_csv_series(&PathBuf::from(btc_daily)).expect("Failed to add BTC CSV series");

    let mut spx_ticker_data = TickerData::new();
    spx_ticker_data.build_series(
        "SPX",
        Interval::Daily,
        &PathBuf::from(spx_daily),
    ).await.expect("Failed to add SPX CSV series");

    btcusd(
        start_date,
        end_date,
        left_bars,
        right_bars,
        hda_margin,
        &btc_ticker_data,
        btc_hda_file
    ).await;

    spx(
        start_date,
        end_date,
        left_bars,
        right_bars,
        hda_margin,
        &spx_ticker_data,
        spx_hda_file
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

#[allow(clippy::too_many_arguments)]
async fn spx(
    hda_start_date: Time,
    hda_end_date: Time,
    pivot_left_bars: usize,
    pivot_right_bars: usize,
    hda_margin: usize,
    ticker_data: &TickerData,
    spx_hda_file: String
) {
    // ======================== Historical Date Analysis ============================
    let hda = PlotHDA::new(
        hda_start_date,
        hda_end_date,
        pivot_left_bars,
        pivot_right_bars,
        hda_margin,
    );
    let daily_hda = hda.hda(ticker_data);
    hda.plot_hda(&daily_hda, &spx_hda_file, "SPX - HDA", &BLUE);
}

#[allow(clippy::too_many_arguments)]
async fn btcusd(
    hda_start_date: Time,
    hda_end_date: Time,
    pivot_left_bars: usize,
    pivot_right_bars: usize,
    hda_margin: usize,
    ticker_data: &TickerData,
    btc_hda_file: String,
) {
    // ======================== Historical Date Analysis ============================
    let hda = PlotHDA::new(
        hda_start_date,
        hda_end_date,
        pivot_left_bars,
        pivot_right_bars,
        hda_margin,
    );
    let daily_hda = hda.hda(ticker_data);
    hda.plot_hda(&daily_hda, &btc_hda_file, "BTCUSD - HDA", &BLUE);
}