use log::*;
use plotters::prelude::full_palette::{BLUE, GREEN, RED};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::env;
use std::path::PathBuf;
use time_series::*;

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

    let pfs_cycle_years = env::var("PFS_CYCLE_YEARS")
      .expect("PFS_CYCLE_YEARS not set");
    let btc_pfs_file = path_to_dir.clone() + "/data/BTCUSD/BTC_pfs_" + &pfs_cycle_years + ".png";
    let spx_pfs_file = path_to_dir.clone() + "/data/SPX/SPX_pfs_" + &pfs_cycle_years + ".png";
    let cycle_years = pfs_cycle_years.parse::<u32>().expect("PFS_CYCLE_YEARS not a number");

    // SPX
    let spx_daily = path_to_dir.clone() + "/data/SPX/1960_2023.csv";
    let spx_history = path_to_dir.clone() + "/data/SPX/SPX_history.csv";
    // BTCUSD
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/BTC_daily.csv";
    let btc_history = path_to_dir.clone() + "/data/BTCUSD/BTC_history.csv";

    let start_date = Time::new(start_year, &Month::from_num(start_month), &Day::from_num(start_day), None, None);
    let end_date = Time::new(end_year, &Month::from_num(end_month), &Day::from_num(end_day), None, None);

    btcusd(
        start_date,
        end_date,
        btc_daily,
        btc_history,
        btc_pfs_file,
        cycle_years
    ).await;
    spx(
        start_date,
        end_date,
        spx_daily,
        spx_history,
        spx_pfs_file,
        cycle_years
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
    start_date: Time,
    end_date: Time,
    daily_ticker: String,
    full_history_file: String,
    pfs_file: String,
    pfs_cycle_years: u32,
) {
    // load TickerData with SPX price history
    let mut ticker_data = TickerData::new();
    ticker_data
      .add_csv_series(&PathBuf::from(daily_ticker))
      .expect("Failed to add CSV to TickerData");

    // TODO: subscribe to RapidAPI
    // stream real-time data from RapidAPI to TickerData
    // let rapid_api = RapidApi::new("SPX".to_string());
    // let candles = rapid_api.query(Interval::Daily).await;
    // ticker_data
    //   .add_series(candles)
    //   .expect("Failed to add API series to TickerData");
    // write full ticker_data history to CSV
    dataframe::ticker_dataframe(&ticker_data, &PathBuf::from(full_history_file));

    // ======================== Polarity Factor System ============================
    // TODO: plot all PFS in one chart
    let pfs = PlotPFS::new(pfs_cycle_years, start_date, end_date);
    let daily_pfs = pfs.pfs(&ticker_data);
    let title = format!("SPX - PFS {}", pfs_cycle_years);
    pfs.plot_pfs(
        &daily_pfs,
        &pfs_file,
        &title,
        &GREEN,
        (97.0, 103.0),
    );
}

#[allow(dead_code)]
async fn btcusd(
    start_date: Time,
    end_date: Time,
    daily_ticker: String,
    full_history_file: String,
    pfs_file: String,
    pfs_cycle_years: u32,
) {
    // load TickerData with SPX price history
    let mut ticker_data = TickerData::new();
    ticker_data
      .add_csv_series(&PathBuf::from(daily_ticker))
      .expect("Failed to add CSV to TickerData");

    // TODO: subscribe to RapidAPI
    // stream real-time data from RapidAPI to TickerData
    // let rapid_api = RapidApi::new("BTC".to_string());
    // let candles = rapid_api.query(Interval::Daily).await;
    // ticker_data.add_series(candles).expect("Failed to add API series to TickerData");
    // write full ticker_data history to CSV
    dataframe::ticker_dataframe(&ticker_data, &PathBuf::from(full_history_file));

    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(pfs_cycle_years, start_date, end_date);
    let daily_pfs = pfs.pfs(&ticker_data);
    let title = format!("BTCUSD - PFS {}", pfs_cycle_years);
    pfs.plot_pfs(
        &daily_pfs,
        &pfs_file,
        &title,
        &GREEN,
        (90.0, 120.0),
    );
}