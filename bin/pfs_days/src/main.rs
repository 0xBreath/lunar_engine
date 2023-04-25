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

    let pfs_confluent_years_raw = env::var("PFS_CONFLUENT_YEARS")
      .expect("PFS_CONFLUENT_YEARS not set");
    let mut pfs_confluent_years: Vec<&str> = pfs_confluent_years_raw.split(',').collect();
    pfs_confluent_years = pfs_confluent_years.iter()
      .map(|&x| x.trim())
      .collect();
    // map to u32
    let pfs_confluent_years: Vec<u32> = pfs_confluent_years
      .iter()
      .map(|&x| x.parse::<u32>().expect("PFS_CONFLUENT_YEARS not a number"))
      .collect();

    // BTCUSD
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/input/BTC_daily.csv";
    let btc_pfs_file = path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_days_" + &pfs_cycle + ".png";
    let btc_conf_dir_file = path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_days_conf_dir.csv";
    let btc_conf_rev_file = path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_days_conf_rev.csv";

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

    // btcusd_backtest(
    //     start_date,
    //     end_date,
    //     &btc_daily_ticker,
    //     vec![cycle],
    // ).await;

    btcusd_pfs_confluent_direction(
        start_date,
        end_date,
        &pfs_confluent_years,
        &btc_daily_ticker,
        btc_conf_dir_file
    ).await;

    btcusd_pfs_confluent_reversal(
        start_date,
        end_date,
        &pfs_confluent_years,
        &btc_daily_ticker,
        btc_conf_rev_file,
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
    let daily_pfs = pfs.pfs_days(ticker_data, pfs_cycle);
    let title = format!("SPX - PFS Days {}", pfs_cycle);
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
    let daily_pfs = pfs.pfs_days(ticker_data, pfs_cycle);
    let title = format!("BTCUSD - PFS Days {}", pfs_cycle);
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
        pfs_cycles.push(pfs.pfs_days(ticker_data, cycle))
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

#[allow(dead_code)]
async fn btcusd_pfs_confluent_direction(
    start_date: Time,
    end_date: Time,
    pfs_confluent_years: &[u32],
    ticker_data: &TickerData,
    pfs_confluence_file: String,
) {
    // ======================== Polarity Factor System ============================
    let mut pfs = PlotPFS::new(start_date, end_date);
    let timeframe = PFSTimeframe::Day;
    let backtest_corr = pfs.confluent_pfs_direction(ticker_data, pfs_confluent_years, timeframe,&pfs_confluence_file);
    for corr in backtest_corr {
        println!("Cycle: {:?}, Corr: {}", corr.cycles, corr.pct_correlation);
    }
}

#[allow(dead_code)]
async fn btcusd_pfs_confluent_reversal(
    start_date: Time,
    end_date: Time,
    pfs_confluent_years: &[u32],
    ticker_data: &TickerData,
    pfs_confluence_file: String,
) {
    // ======================== Polarity Factor System ============================
    let mut pfs = PlotPFS::new(start_date, end_date);
    let mut cycles = vec![];
    for cycle in pfs_confluent_years.iter() {
        cycles.push((*cycle, pfs.pfs_days(ticker_data, *cycle)));
    }
    let mut pfs_cycles = vec![];
    for cycle in pfs_confluent_years.iter() {
        pfs_cycles.push((*cycle, pfs.pfs_days(ticker_data, *cycle)));
    }
    let timeframe = PFSTimeframe::Day;
    let backtest_corr = pfs.confluent_pfs_reversal(ticker_data,pfs_confluent_years, timeframe,&pfs_confluence_file);
    for corr in backtest_corr {
        println!("Cycles: {:?}, Corr: {}, Hits: {}, Total: {}", corr.cycles, corr.pct_correlation, corr.hits, corr.total);
    }
}