use log::*;
use plotters::prelude::full_palette::GREEN;
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

    let pfs_cycle_years = env::var("PFS_CYCLE_YEARS").expect("PFS_CYCLE_YEARS not set");

    let pfs_confluent_years_raw =
        env::var("PFS_CONFLUENT_YEARS").expect("PFS_CONFLUENT_YEARS not set");
    let mut pfs_confluent_years: Vec<&str> = pfs_confluent_years_raw.split(',').collect();
    pfs_confluent_years = pfs_confluent_years.iter().map(|&x| x.trim()).collect();
    // map to u32
    #[allow(unused_variables)]
    let pfs_confluent_years: Vec<u32> = pfs_confluent_years
        .iter()
        .map(|&x| x.parse::<u32>().expect("PFS_CONFLUENT_YEARS not a number"))
        .collect();

    // true for percentage, false for pips
    let trailing_stop_use_pct = env::var("TRAILING_STOP_USE_PCT")
        .expect("TRAILING_STOP_USE_PCT not set")
        .parse::<bool>()
        .expect("TRAILING_STOP_USE_PCT not a bool");
    #[allow(unused_variables)]
    let trailing_stop_type: TrailingStopType = if trailing_stop_use_pct {
        TrailingStopType::Percent
    } else {
        TrailingStopType::Pips
    };

    // trailing stop in pips or percentage
    #[allow(unused_variables)]
    let trailing_stop = env::var("TRAILING_STOP")
        .expect("TRAILING_STOP not set")
        .parse::<f64>()
        .expect("TRAILING_STOP not a number");

    #[allow(unused_variables)]
    let stop_loss_pct = env::var("STOP_LOSS_PCT")
        .expect("STOP_LOSS_PCT not set")
        .parse::<f64>()
        .expect("STOP_LOSS_PCT not a number");

    #[allow(unused_variables)]
    let cycle_years = pfs_cycle_years
        .parse::<u32>()
        .expect("PFS_CYCLE_YEARS not a number");

    // SPX
    #[allow(unused_variables)]
    let spx_daily = path_to_dir.clone() + "/data/SPX/input/SPX_daily.csv";
    #[allow(unused_variables)]
    let spx_confluent_direction_file =
        path_to_dir.clone() + "/data/SPX/output/SPX_PFS_confluent_direction.csv";
    #[allow(unused_variables)]
    let spx_confluent_reversal_file =
        path_to_dir.clone() + "/data/SPX/output/SPX_PFS_confluent_reversal.csv";
    #[allow(unused_variables)]
    let spx_confluent_backtest_file =
        path_to_dir.clone() + "/data/SPX/output/SPX_PFS_confluent_backtest.csv";
    #[allow(unused_variables)]
    let spx_pfs_file =
        path_to_dir.clone() + "/data/SPX/output/SPX_pfs_" + &pfs_cycle_years + ".png";
    // BTCUSD
    #[allow(unused_variables)]
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/input/BTC_daily.csv";
    #[allow(unused_variables)]
    let btc_1h = path_to_dir.clone() + "/data/BTCUSD/input/BTC_1h.csv";
    #[allow(unused_variables)]
    let btc_5min = path_to_dir.clone() + "/data/BTCUSD/input/BTC_5min.csv";
    #[allow(unused_variables)]
    let btc_history = path_to_dir.clone() + "/data/BTCUSD/output/BTC_history.csv";
    #[allow(unused_variables)]
    let btc_pfs_file =
        path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_" + &pfs_cycle_years + ".png";

    #[allow(unused_variables)]
    let start_date = Time::new(
        start_year,
        &Month::from_num(start_month),
        &Day::from_num(start_day),
        None,
        None,
    );
    #[allow(unused_variables)]
    let end_date = Time::new(
        end_year,
        &Month::from_num(end_month),
        &Day::from_num(end_day),
        None,
        None,
    );

    let mut btc_ticker_data = TickerData::new();
    btc_ticker_data
        .add_csv_series(&PathBuf::from(btc_daily))
        .expect("Failed to add BTC CSV series");

    // let mut spx_ticker_data = TickerData::new();
    // spx_ticker_data.build_series(
    //   "SPX",
    //   Interval::Daily,
    //   &PathBuf::from(spx_daily),
    // ).await.expect("Failed to add SPX CSV series");

    // btcusd(
    //     start_date,
    //     end_date,
    //     &btc_ticker_data,
    //     btc_pfs_file,
    //     cycle_years,
    // ).await;
    //
    // spx(
    //     start_date,
    //     end_date,
    //     &spx_ticker_data,
    //     spx_pfs_file,
    //     cycle_years,
    // ).await;

    // spx_pfs_confluent_direction(
    //     start_date,
    //     end_date,
    //     pfs_confluent_years.clone(),
    //     &spx_ticker_data,
    //     spx_confluent_direction_file
    // ).await;

    // spx_pfs_confluent_reversal(
    //     start_date,
    //     end_date,
    //     pfs_confluent_years.clone(),
    //    &spx_ticker_data,
    //     spx_confluent_reversal_file
    // ).await;
    //

    // spx_confluent_reversal_backtest(
    //     start_date,
    //     end_date,
    //     pfs_confluent_years,
    //     &spx_ticker_data,
    //     spx_confluent_backtest_file,
    //     1000.0,
    //     trailing_stop_type,
    //     trailing_stop,
    //     stop_loss_pct
    // ).await;
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

/// Expects SPX PFS to be run first to generate the SPX ticker history
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
async fn spx_confluent_reversal_backtest(
    start_date: Time,
    end_date: Time,
    pfs_confluent_years: &[u32],
    ticker_data: &TickerData,
    pfs_backtest_file: String,
    capital: f64,
    trailing_stop_type: TrailingStopType,
    trailing_stop: f64,
    stop_loss_pct: f64,
) {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let _ = pfs.backtest_confluent_pfs_reversal(
        ticker_data,
        pfs_confluent_years,
        PFSTimeframe::Year,
        &pfs_backtest_file,
        capital,
        trailing_stop_type,
        trailing_stop,
        stop_loss_pct,
    );
}

/// Expects SPX PFS to be run first to generate the SPX ticker history
#[allow(dead_code)]
async fn spx_pfs_confluent_direction(
    start_date: Time,
    end_date: Time,
    pfs_confluent_years: &[u32],
    ticker_data: &TickerData,
    pfs_confluence_file: String,
) -> PFSResult<()> {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let backtest_corr = pfs.confluent_pfs_direction(
        ticker_data,
        pfs_confluent_years,
        PFSTimeframe::Year,
        &pfs_confluence_file,
    )?;
    for corr in backtest_corr {
        println!("Cycle: {:?}, Corr: {}", corr.cycles, corr.pct_correlation);
    }
    Ok(())
}

/// Expects SPX PFS to be run first to generate the SPX ticker history
#[allow(dead_code)]
async fn spx_pfs_confluent_reversal(
    start_date: Time,
    end_date: Time,
    pfs_confluent_years: &[u32],
    ticker_data: &TickerData,
    pfs_confluence_file: String,
) -> PFSResult<()> {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let backtest_corr = pfs.confluent_pfs_reversal(
        ticker_data,
        pfs_confluent_years,
        PFSTimeframe::Year,
        &pfs_confluence_file,
    )?;
    for corr in backtest_corr {
        println!(
            "Cycles: {:?}, Corr: {}, Hits: {}, Total: {}",
            corr.cycles, corr.pct_correlation, corr.hits, corr.total
        );
    }
    Ok(())
}

#[allow(dead_code)]
async fn spx(
    start_date: Time,
    end_date: Time,
    ticker_data: &TickerData,
    pfs_file: String,
    pfs_cycle_years: u32,
) {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let daily_pfs = PlotPFS::pfs_years(pfs.start_date, pfs.end_date, ticker_data, pfs_cycle_years);
    let title = format!("SPX - PFS Years {}", pfs_cycle_years);
    pfs.plot_pfs(&daily_pfs, &pfs_file, &title, &GREEN);
}

#[allow(dead_code)]
async fn btcusd(
    start_date: Time,
    end_date: Time,
    ticker_data: &TickerData,
    pfs_file: String,
    pfs_cycle_years: u32,
) {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let daily_pfs = PlotPFS::pfs_years(pfs.start_date, pfs.end_date, ticker_data, pfs_cycle_years);
    let title = format!("BTCUSD - PFS Years {}", pfs_cycle_years);
    pfs.plot_pfs(&daily_pfs, &pfs_file, &title, &GREEN);
}
