use log::*;
use plotters::prelude::full_palette::{BLUE, GREEN, RED};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::env;
use std::path::PathBuf;
use time_series::*;

// // SPX
// const SPX_PFS_10_FILE: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/SPX/SPX_pfs_10.png";
// const SPX_PFS_19_FILE: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/SPX/SPX_pfs_19.png";
// const SPX_PFS_20_FILE: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/SPX/SPX_pfs_20.png";
// const SPX_DAILY: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/SPX/1960_2023.csv";
// const SPX_HISTORY: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/SPX/SPX_history.csv";
// // BTCUSD
// const BTC_DAILY: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/BTCUSD/BTC_daily.csv";
// const BTC_HISTORY: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/BTCUSD/BTC_history.csv";
// const BTC_PFS_10_FILE: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/BTCUSD/BTC_pfs_10.png";
// const BTC_PFS_19_FILE: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/BTCUSD/BTC_pfs_19.png";
// const BTC_PFS_20_FILE: &str = "/Users/riester/LIFE/Coding/lunar_engine/data/BTCUSD/BTC_pfs_20.png";

#[tokio::main]
async fn main() {
    init_logger();

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
    let spx_pfs_10_file = path_to_dir.clone() + "/data/SPX/SPX_pfs_10.png";
    let spx_pfs_19_file = path_to_dir.clone() + "/data/SPX/SPX_pfs_19.png";
    let spx_pfs_20_file = path_to_dir.clone() + "/data/SPX/SPX_pfs_20.png";
    let spx_daily = path_to_dir.clone() + "/data/SPX/1960_2023.csv";
    let spx_history = path_to_dir.clone() + "/data/SPX/SPX_history.csv";
    // BTCUSD
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/BTC_daily.csv";
    let btc_history = path_to_dir.clone() + "/data/BTCUSD/BTC_history.csv";
    let btc_pfs_10_file = path_to_dir.clone() + "/data/BTCUSD/BTC_pfs_10.png";
    let btc_pfs_19_file = path_to_dir.clone() + "/data/BTCUSD/BTC_pfs_19.png";
    let btc_pfs_20_file = path_to_dir.clone() + "/data/BTCUSD/BTC_pfs_20.png";

    let start_date = Time::new(start_year, &Month::from_num(start_month), &Day::from_num(start_day), None, None);
    let end_date = Time::new(end_year, &Month::from_num(end_month), &Day::from_num(end_day), None, None);

    btcusd(
        start_date,
        end_date,
        btc_daily,
        btc_history,
        btc_pfs_10_file,
        btc_pfs_19_file,
        btc_pfs_20_file,
    ).await;
    spx(
        start_date,
        end_date,
        spx_daily,
        spx_history,
        spx_pfs_10_file,
        spx_pfs_19_file,
        spx_pfs_20_file,
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
async fn spx(
    pfs_start_date: Time,
    pfs_end_date: Time,
    spx_daily: String,
    spx_history: String,
    spx_pfs_10_file: String,
    spx_pfs_19_file: String,
    spx_pfs_20_file: String,
) {
    // load TickerData with SPX price history
    let mut ticker_data = TickerData::new();
    ticker_data
      .add_csv_series(&PathBuf::from(spx_daily))
      .expect("Failed to add CSV to TickerData");

    // TODO: subscribe to RapidAPI
    // stream real-time data from RapidAPI to TickerData
    // let rapid_api = RapidApi::new("SPX".to_string());
    // let candles = rapid_api.query(Interval::Daily).await;
    // ticker_data
    //   .add_series(candles)
    //   .expect("Failed to add API series to TickerData");
    // write full ticker_data history to CSV
    dataframe::ticker_dataframe(&ticker_data, &PathBuf::from(spx_history));

    // ======================== Polarity Factor System ============================
    // TODO: plot all PFS in one chart
    let pfs_10 = PlotPFS::new(10, pfs_start_date, pfs_end_date);
    let daily_pfs_10 = pfs_10.pfs(&ticker_data);
    pfs_10.plot_pfs(
        &daily_pfs_10,
        &spx_pfs_10_file,
        "SPX - PFS 10",
        &GREEN,
        (97.0, 103.0),
    );

    let pfs_19 = PlotPFS::new(19, pfs_start_date, pfs_end_date);
    let daily_pfs_19 = pfs_19.pfs(&ticker_data);
    pfs_19.plot_pfs(
        &daily_pfs_19,
        &spx_pfs_19_file,
        "SPX - PFS 19",
        &BLUE,
        (97.0, 103.0),
    );

    let pfs_20 = PlotPFS::new(20, pfs_start_date, pfs_end_date);
    let daily_pfs_20 = pfs_20.pfs(&ticker_data);
    pfs_20.plot_pfs(
        &daily_pfs_20,
        &spx_pfs_20_file,
        "SPX - PFS 20",
        &RED,
        (97.0, 103.0),
    );
}

#[allow(dead_code)]
async fn btcusd(
    pfs_start_date: Time,
    pfs_end_date: Time,
    btc_daily: String,
    btc_history: String,
    btc_pfs_10_file: String,
    btc_pfs_19_file: String,
    btc_pfs_20_file: String,
) {
    // load TickerData with SPX price history
    let btc_daily = &PathBuf::from(btc_daily);
    let mut ticker_data = TickerData::new();
    ticker_data
      .add_csv_series(&PathBuf::from(btc_daily))
      .expect("Failed to add CSV to TickerData");

    // TODO: subscribe to RapidAPI
    // stream real-time data from RapidAPI to TickerData
    // let rapid_api = RapidApi::new("BTC".to_string());
    // let candles = rapid_api.query(Interval::Daily).await;
    // ticker_data.add_series(candles).expect("Failed to add API series to TickerData");
    // write full ticker_data history to CSV
    dataframe::ticker_dataframe(&ticker_data, &PathBuf::from(btc_history));

    // ======================== Polarity Factor System ============================
    // TODO: plot all PFS in one chart
    let pfs_10 = PlotPFS::new(10, pfs_start_date, pfs_end_date);
    let daily_pfs_10 = pfs_10.pfs(&ticker_data);
    pfs_10.plot_pfs(
        &daily_pfs_10,
        &btc_pfs_10_file,
        "BTCUSD - PFS 10",
        &GREEN,
        (90.0, 120.0),
    );

    let pfs_19 = PlotPFS::new(19, pfs_start_date, pfs_end_date);
    let daily_pfs_19 = pfs_19.pfs(&ticker_data);
    pfs_19.plot_pfs(
        &daily_pfs_19,
        &btc_pfs_19_file,
        "BTCUSD - PFS 19",
        &BLUE,
        (90.0, 120.0),
    );

    let pfs_20 = PlotPFS::new(20, pfs_start_date, pfs_end_date);
    let daily_pfs_20 = pfs_20.pfs(&ticker_data);
    pfs_20.plot_pfs(
        &daily_pfs_20,
        &btc_pfs_20_file,
        "BTCUSD - PFS 20",
        &RED,
        (90.0, 120.0),
    );
}