use log::*;
use plotters::prelude::full_palette::{GREEN};
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

    let pfs_cycle = env::var("PFS_CYCLE")
      .expect("PFS_CYCLE not set");
    let cycle = pfs_cycle.parse::<u32>().expect("PFS_CYCLE not a number");

    // BTCUSD
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/input/BTC_daily.csv";
    #[allow(unused_variables)]
    let btc_pfs_file = path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_months_" + &pfs_cycle + ".png";

    let start_date = Time::new(start_year, &Month::from_num(start_month), &Day::from_num(start_day), None, None);
    let end_date = Time::new(end_year, &Month::from_num(end_month), &Day::from_num(end_day), None, None);

    let mut btc_daily_ticker = TickerData::new();
    btc_daily_ticker.add_csv_series(&PathBuf::from(btc_daily)).expect("Failed to add BTC 5 minute csv series");

    // btcusd(
    //     start_date,
    //     end_date,
    //     &btc_daily_ticker,
    //     btc_pfs_file,
    //     cycle,
    // ).await;

    btcusd_backtest(
        start_date,
        end_date,
        &btc_daily_ticker,
        vec![cycle]
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
    ticker_data: &TickerData,
    pfs_file: String,
    pfs_cycle: u32,
) {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let daily_pfs = PlotPFS::pfs_months(pfs.start_date, pfs.end_date, ticker_data, pfs_cycle);
    let title = format!("SPX - PFS Months {}", pfs_cycle);
    pfs.plot_pfs(
        &daily_pfs,
        &pfs_file,
        &title,
        &GREEN,
    );
}

#[allow(dead_code)]
async fn btcusd(
    start_date: Time,
    end_date: Time,
    ticker_data: &TickerData,
    pfs_file: String,
    pfs_cycle: u32,
) {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let daily_pfs = PlotPFS::pfs_months(pfs.start_date, pfs.end_date, ticker_data, pfs_cycle);
    let title = format!("BTCUSD - PFS Months {}", pfs_cycle);
    pfs.plot_pfs(
        &daily_pfs,
        &pfs_file,
        &title,
        &GREEN,
    );
}

#[allow(dead_code)]
async fn btcusd_backtest(
    start_date: Time,
    end_date: Time,
    ticker_data: &TickerData,
    cycles: Vec<u32>,
) {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let mut pfs_cycles = vec![];
    for cycle in cycles {
        pfs_cycles.push(PlotPFS::pfs_months(pfs.start_date, pfs.end_date, ticker_data, cycle))
    };

    let backtests = pfs.individual_pfs_correlation(
        ticker_data,
        pfs_cycles
    );
    for backtest in backtests {
        println!(
            "start: {}\tend: {}\tcycle: {}\tcorr: {}\thits: {}\ttotal: {}",
            start_date.to_string(),
            end_date.to_string(),
            backtest.cycle,
            backtest.pct_correlation,
            backtest.hits,
            backtest.total
        );
    }
}