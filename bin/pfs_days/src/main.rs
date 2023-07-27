use log::*;
use plotters::prelude::full_palette::GREEN;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use time_series::*;

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> PFSResult<()> {
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

    let pfs_cycle = env::var("PFS_CYCLE").expect("PFS_CYCLE not set");
    #[allow(unused_variables)]
    let cycle = pfs_cycle.parse::<u32>().expect("PFS_CYCLE not a number");

    let pfs_confluent_cycles_raw =
        env::var("PFS_CONFLUENT_CYCLES").expect("PFS_CONFLUENT_CYCLES not set");
    let mut pfs_confluent_cycles: Vec<&str> = pfs_confluent_cycles_raw.split(',').collect();
    pfs_confluent_cycles = pfs_confluent_cycles.iter().map(|&x| x.trim()).collect();
    // map to u32
    let pfs_confluent_cycles: Vec<u32> = pfs_confluent_cycles
        .iter()
        .map(|&x| x.parse::<u32>().expect("PFS_CONFLUENT_CYCLES not a number"))
        .collect();

    // true for percentage, false for pips
    let trailing_stop_use_pct = env::var("TRAILING_STOP_USE_PCT")
        .expect("TRAILING_STOP_USE_PCT not set")
        .parse::<bool>()
        .expect("TRAILING_STOP_USE_PCT not a bool");
    let trailing_stop_type: TrailingStopType = if trailing_stop_use_pct {
        TrailingStopType::Percent
    } else {
        TrailingStopType::Pips
    };

    // trailing stop in pips or percentage
    let trailing_stop = env::var("TRAILING_STOP")
        .expect("TRAILING_STOP not set")
        .parse::<f64>()
        .expect("TRAILING_STOP not a number");

    let stop_loss_pct = env::var("STOP_LOSS_PCT")
        .expect("STOP_LOSS_PCT not set")
        .parse::<f64>()
        .expect("STOP_LOSS_PCT not a number");

    // BTCUSD
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/input/BTC_daily.csv";
    #[allow(unused_variables)]
    let btc_pfs_file =
        path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_days_" + &pfs_cycle + ".png";
    #[allow(unused_variables)]
    let btc_conf_dir_file = path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_days_conf_dir.csv";
    #[allow(unused_variables)]
    let btc_conf_rev_file = path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_days_conf_rev.csv";
    #[allow(unused_variables)]
    let btc_conf_dir_backtest_file =
        path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_days_conf_dir_backtest.csv";
    #[allow(unused_variables)]
    let btc_conf_rev_backtest_file =
        path_to_dir.clone() + "/data/BTCUSD/output/BTC_pfs_days_conf_rev_backtest.csv";

    // SPX
    #[allow(unused_variables)]
    let spx_daily = path_to_dir.clone() + "/data/SPX/input/SPX_daily.csv";
    #[allow(unused_variables)]
    let spx_pfs_file = path_to_dir.clone() + "/data/SPX/output/SPX_pfs_days_" + &pfs_cycle + ".png";
    #[allow(unused_variables)]
    let spx_conf_dir_file = path_to_dir.clone() + "/data/SPX/output/SPX_pfs_days_conf_dir.csv";
    #[allow(unused_variables)]
    let spx_conf_rev_file = path_to_dir.clone() + "/data/SPX/output/SPX_pfs_days_conf_rev.csv";
    #[allow(unused_variables)]
    let spx_conf_dir_backtest_file =
        path_to_dir.clone() + "/data/SPX/output/SPX_pfs_days_conf_dir_backtest.csv";

    let start_date = Time::new(
        start_year,
        &Month::from_num(start_month),
        &Day::from_num(start_day),
        None,
        None,
    );
    let end_date = Time::new(
        end_year,
        &Month::from_num(end_month),
        &Day::from_num(end_day),
        None,
        None,
    );

    let mut btc_daily_ticker = TickerData::new();
    btc_daily_ticker
        .add_csv_series(&PathBuf::from(btc_daily))
        .expect("Failed to add BTC 5 minute csv series");

    // let mut spx_daily_ticker = TickerData::new();
    // spx_daily_ticker.build_series("SPX", Interval::Daily, &PathBuf::from(spx_daily))
    //   .await
    //   .expect("Failed to build SPX daily series");

    // btcusd(
    //     start_date,
    //     end_date,
    //     &btc_daily_ticker,
    //     btc_pfs_file,
    //     cycle,
    // ).await;

    // spx(
    //     start_date,
    //     end_date,
    //     &spx_daily_ticker,
    //     spx_pfs_file,
    //     cycle
    // ).await;

    // btcusd_individual_pfs_backtest(
    //     start_date,
    //     end_date,
    //     &btc_daily_ticker,
    //     vec![cycle],
    // ).await;

    // let ticker_data = btc_daily_ticker.clone();
    // let pfs_conf_cycles = pfs_confluent_cycles.clone();
    // let conf_rev_backtest = tokio::spawn(async move {
    //     let conf_rev = btcusd_pfs_confluent_reversal(
    //         start_date,
    //         end_date,
    //         &pfs_conf_cycles,
    //         &ticker_data,
    //         btc_conf_rev_file.clone(),
    //     ).await;
    //     println!("Confluent PFS reversal results have been saved to {}", btc_conf_rev_file);
    //     let res = btcusd_confluent_reversal_backtest(
    //         conf_rev,
    //         &ticker_data,
    //         &btc_conf_rev_backtest_file,
    //         trailing_stop_type,
    //         trailing_stop,
    //         stop_loss_pct
    //     );
    //     println!("Confluent PFS reversal backtest results have been saved to {}", btc_conf_rev_backtest_file);
    //     res
    // });
    // conf_rev_backtest.await.expect("Failed to run BTCUSD confluent reversal backtest");

    let ticker_data = btc_daily_ticker.clone();
    let pfs_conf_cycles = pfs_confluent_cycles;
    let conf_dir = btcusd_pfs_confluent_direction(
        start_date,
        end_date,
        &pfs_conf_cycles,
        &ticker_data,
        btc_conf_dir_file.clone(),
    )
    .await?;
    println!(
        "Confluent PFS direction results have been saved to {}",
        btc_conf_dir_file
    );
    let _res = btcusd_confluent_direction_backtest(
        conf_dir,
        &btc_daily_ticker,
        &btc_conf_dir_backtest_file,
        trailing_stop_type,
        trailing_stop,
        stop_loss_pct,
    );
    println!(
        "Confluent PFS direction backtest results have been saved to {}",
        btc_conf_dir_backtest_file
    );
    Ok(())
}

pub fn init_logger() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");
}

#[allow(dead_code)]
async fn spx(
    start_date: Time,
    end_date: Time,
    ticker_data: &TickerData,
    pfs_file: String,
    pfs_cycle: u32,
) -> PFSResult<()> {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let daily_pfs = PlotPFS::pfs_days(pfs.start_date, pfs.end_date, ticker_data, pfs_cycle)?;
    let title = format!("SPX - PFS Days {}", pfs_cycle);
    pfs.plot_pfs(&daily_pfs, &pfs_file, &title, &GREEN);
    Ok(())
}

#[allow(dead_code)]
async fn btcusd(
    start_date: Time,
    end_date: Time,
    ticker_data: &TickerData,
    pfs_file: String,
    pfs_cycle: u32,
) -> PFSResult<()> {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let daily_pfs = PlotPFS::pfs_days(pfs.start_date, pfs.end_date, ticker_data, pfs_cycle)?;
    let title = format!("BTCUSD - PFS Days {}", pfs_cycle);
    pfs.plot_pfs(&daily_pfs, &pfs_file, &title, &GREEN);
    Ok(())
}

#[allow(dead_code)]
async fn btcusd_individual_pfs_backtest(
    start_date: Time,
    end_date: Time,
    ticker_data: &TickerData,
    cycles: Vec<u32>,
) -> PFSResult<()> {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let mut pfs_cycles = vec![];
    for cycle in cycles {
        pfs_cycles.push(PlotPFS::pfs_days(
            pfs.start_date,
            pfs.end_date,
            ticker_data,
            cycle,
        )?)
    }

    let backtests = pfs.individual_pfs_correlation(ticker_data, pfs_cycles);
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
    Ok(())
}

#[allow(dead_code)]
async fn btcusd_pfs_confluent_direction(
    start_date: Time,
    end_date: Time,
    pfs_confluent_years: &[u32],
    ticker_data: &TickerData,
    pfs_confluence_file: String,
) -> PFSResult<Vec<ConfluentPFSCorrelation>> {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let timeframe = PFSTimeframe::Day;
    let backtest_corr = pfs.confluent_pfs_direction(
        ticker_data,
        pfs_confluent_years,
        timeframe,
        &pfs_confluence_file,
    )?;
    println!("##### Confluent PFS Direction Correlation #####");
    for corr in backtest_corr.iter() {
        println!(
            "Cycles: {:?}, Corr: {}, Hits: {}, Total: {}",
            corr.cycles, corr.pct_correlation, corr.hits, corr.total
        );
    }
    Ok(backtest_corr)
}

#[allow(dead_code)]
async fn btcusd_pfs_confluent_reversal(
    start_date: Time,
    end_date: Time,
    pfs_confluent_cycles: &[u32],
    ticker_data: &TickerData,
    pfs_confluence_file: String,
) -> PFSResult<Vec<ConfluentPFSCorrelation>> {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    let mut pfs_cycles = vec![];
    for cycle in pfs_confluent_cycles.iter() {
        pfs_cycles.push((
            *cycle,
            PlotPFS::pfs_days(pfs.start_date, pfs.end_date, ticker_data, *cycle)?,
        ));
    }
    let timeframe = PFSTimeframe::Day;
    let backtest_corr = pfs.confluent_pfs_reversal(
        ticker_data,
        pfs_confluent_cycles,
        timeframe,
        &pfs_confluence_file,
    )?;
    println!("##### Confluent PFS Reversal Correlation #####");
    for corr in backtest_corr.iter() {
        println!(
            "Cycles: {:?}, Corr: {}, Hits: {}, Total: {}",
            corr.cycles, corr.pct_correlation, corr.hits, corr.total
        );
    }
    Ok(backtest_corr)
}

fn write_backtest_csv(
    backtests: Vec<(Backtest, Vec<u32>)>,
    out_file: &str,
) -> Result<(), Box<dyn Error>> {
    if backtests.is_empty() {
        return Err("No backtests found".into());
    }
    let mut file = File::create(out_file)?;

    writeln!(
        file,
        "start_date,end_date,pnl,avg_trade,avg_win,avg_loss,win_trades,loss_trades,trades,cycles"
    )?;
    for backtest in backtests.iter() {
        if backtest.0.trades.is_empty() {
            continue;
        }
        let cycles = backtest
            .1
            .iter()
            .map(|cycle| cycle.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let start_date = backtest
            .0
            .start_date
            .expect("No start date found")
            .to_string_daily();
        let end_date = backtest
            .0
            .end_date
            .expect("No end date found")
            .to_string_daily();
        let pnl = backtest.0.pnl.unwrap_or(0.0);
        let avg_trade = backtest.0.avg_trade_pnl.unwrap_or(0.0);
        let avg_win = backtest.0.avg_win_trade_pnl.unwrap_or(0.0);
        let avg_loss = backtest.0.avg_loss_trade_pnl.unwrap_or(0.0);
        let win_trades = backtest.0.num_win_trades();
        let loss_trades = backtest.0.num_loss_trades();
        let trades = backtest.0.trades.len();
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},[{}]",
            start_date,
            end_date,
            pnl,
            avg_trade,
            avg_win,
            avg_loss,
            win_trades,
            loss_trades,
            trades,
            cycles
        )?;
    }
    Ok(())
}

fn stop_triggered(
    order: &Order,
    trailing_stop: &Option<f64>,
    stop_loss: &Option<f64>,
    candle: &Candle,
) -> bool {
    match order {
        Order::Long => {
            if trailing_stop.is_some() && stop_loss.is_some() {
                candle.close < trailing_stop.unwrap() || candle.close < stop_loss.unwrap()
            } else if trailing_stop.is_some() && stop_loss.is_none() {
                candle.close < trailing_stop.unwrap()
            } else if trailing_stop.is_none() && stop_loss.is_some() {
                candle.close < stop_loss.unwrap()
            } else {
                false
            }
        }
        Order::Short => {
            if trailing_stop.is_some() && stop_loss.is_some() {
                candle.close > trailing_stop.unwrap() || candle.close > stop_loss.unwrap()
            } else if trailing_stop.is_some() && stop_loss.is_none() {
                candle.close > trailing_stop.unwrap()
            } else if trailing_stop.is_none() && stop_loss.is_some() {
                candle.close > stop_loss.unwrap()
            } else {
                false
            }
        }
    }
}

fn btcusd_confluent_direction_backtest(
    conf_pfs_dir: Vec<ConfluentPFSCorrelation>,
    ticker_data: &TickerData,
    backtest_file: &str,
    trailing_stop_type: TrailingStopType,
    trailing_stop: f64,
    stop_loss_pct: f64,
) -> Vec<(Backtest, Vec<u32>)> {
    let capital = 1000.0;
    let mut backtests = Vec::<(Backtest, Vec<u32>)>::new();
    // iterate through PFS cycle combinations
    let mut threads = vec![];
    for corr in conf_pfs_dir.into_iter() {
        let ticker_data = ticker_data.clone();
        let thread = std::thread::spawn(move || {
            let open_trade_mutex: Arc<Mutex<Option<Trade>>> = Arc::new(Mutex::new(None));
            let mut backtest = Backtest::new(capital);

            // iterate time series
            for candle in ticker_data.get_candles().iter() {
                let date = candle.date;
                // find confluent PFS event on this candle date
                let pfs_event = corr.events.iter().find(|&x| x.date == date);
                let mut open_trade = open_trade_mutex
                    .lock()
                    .expect("Failed to lock open trade mutex");

                // confluent PFS direction on this date
                match pfs_event {
                    Some(pfs_event) => {
                        if let Some(direction) = &pfs_event.direction {
                            match direction {
                                // exit short, enter long
                                Direction::Up => {
                                    // exit short
                                    if let Some(trade) = &*open_trade {
                                        let mut trade = trade.clone();
                                        if trade.order == Order::Short
                                            || stop_triggered(
                                                &trade.order,
                                                &trade.trailing_stop,
                                                &trade.stop_loss,
                                                candle,
                                            )
                                        {
                                            trade.exit(date, candle.close);
                                            backtest.add_trade(trade);
                                            *open_trade = None;
                                        }
                                    }
                                    // enter long
                                    let qty = Trade::trade_quantity(capital, candle.close);
                                    let trailing_stop = Trade::calc_trailing_stop(
                                        Order::Long,
                                        candle.close,
                                        trailing_stop_type,
                                        trailing_stop,
                                    );
                                    let stop_loss = Trade::calc_stop_loss(
                                        Order::Long,
                                        candle.close,
                                        stop_loss_pct,
                                    );
                                    *open_trade = Some(Trade::new(
                                        date,
                                        Order::Long,
                                        qty,
                                        candle.close,
                                        capital,
                                        Some(trailing_stop),
                                        Some(stop_loss),
                                    ));
                                }
                                // exit long, enter short
                                Direction::Down => {
                                    // exit long
                                    if let Some(trade) = &*open_trade {
                                        // clone is ok because value is overwritten after this block
                                        let mut trade = trade.clone();
                                        if trade.order == Order::Long
                                            || stop_triggered(
                                                &trade.order,
                                                &trade.trailing_stop,
                                                &trade.stop_loss,
                                                candle,
                                            )
                                        {
                                            trade.exit(date, candle.close);
                                            backtest.add_trade(trade);
                                            *open_trade = None;
                                        }
                                    }
                                    // enter short
                                    let qty = Trade::trade_quantity(capital, candle.close);
                                    let trailing_stop = Trade::calc_trailing_stop(
                                        Order::Short,
                                        candle.close,
                                        trailing_stop_type,
                                        trailing_stop,
                                    );
                                    let stop_loss = Trade::calc_stop_loss(
                                        Order::Short,
                                        candle.close,
                                        stop_loss_pct,
                                    );
                                    *open_trade = Some(Trade::new(
                                        date,
                                        Order::Short,
                                        qty,
                                        candle.close,
                                        capital,
                                        Some(trailing_stop),
                                        Some(stop_loss),
                                    ));
                                }
                            }
                        }
                    }
                    // if no event, check trailing stop
                    // if trailing stop is hit, exit trade
                    // other update trailing stop
                    None => {
                        debug!("No PFS Direction: {}", date.to_string_daily());
                        if let Some(trade) = &*open_trade {
                            match trade.order {
                                Order::Long => {
                                    // Long trailing stop is hit, exit trade
                                    if stop_triggered(
                                        &Order::Long,
                                        &trade.trailing_stop,
                                        &trade.stop_loss,
                                        candle,
                                    ) {
                                        let mut trade = trade.clone();
                                        trade.exit(date, candle.close);
                                        backtest.add_trade(trade);
                                        *open_trade = None;
                                    }
                                    // Long trailing stop is not hit, update trailing stop
                                    else {
                                        let mut trade = trade.clone();
                                        trade.trailing_stop = Some(Trade::calc_trailing_stop(
                                            Order::Long,
                                            candle.close,
                                            trailing_stop_type,
                                            trailing_stop,
                                        ));
                                        *open_trade = Some(trade);
                                    }
                                }
                                Order::Short => {
                                    // Short trailing stop is hit, exit trade
                                    if stop_triggered(
                                        &Order::Short,
                                        &trade.trailing_stop,
                                        &trade.stop_loss,
                                        candle,
                                    ) {
                                        let mut trade = trade.clone();
                                        trade.exit(date, candle.close);
                                        backtest.add_trade(trade);
                                        *open_trade = None;
                                    }
                                    // Short trailing stop is not hit, update trailing stop
                                    else {
                                        let mut trade = trade.clone();
                                        trade.trailing_stop = Some(Trade::calc_trailing_stop(
                                            Order::Short,
                                            candle.close,
                                            trailing_stop_type,
                                            trailing_stop,
                                        ));
                                        *open_trade = Some(trade);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            backtest.summarize();
            (backtest, corr.cycles)
        });
        threads.push(thread);
    }
    for thread in threads {
        let backtest = thread
            .join()
            .expect("Failed to join PFS confluent direction backtest thread");
        backtests.push(backtest);
    }
    backtests.sort_by(|a, b| b.0.pnl.partial_cmp(&a.0.pnl).unwrap());
    write_backtest_csv(backtests.clone(), backtest_file)
        .expect("Failed to write PFS confluent direction backtest to CSV");
    backtests
}

#[allow(dead_code)]
fn btcusd_confluent_reversal_backtest(
    conf_pfs_rev: Vec<ConfluentPFSCorrelation>,
    ticker_data: &TickerData,
    backtest_file: &str,
    trailing_stop_type: TrailingStopType,
    trailing_stop: f64,
    stop_loss_pct: f64,
) -> Vec<(Backtest, Vec<u32>)> {
    let capital = 1000.0;
    let mut backtests = Vec::<(Backtest, Vec<u32>)>::new();
    // iterate through PFS cycle combinations
    let mut threads = vec![];
    for corr in conf_pfs_rev.into_iter() {
        let ticker_data = ticker_data.clone();
        let thread = std::thread::spawn(move || {
            let open_trade_mutex: Arc<Mutex<Option<Trade>>> = Arc::new(Mutex::new(None));
            let mut backtest = Backtest::new(capital);

            // iterate time series
            for candle in ticker_data.get_candles().iter() {
                let date = candle.date;
                // find confluent PFS event on this candle date
                let pfs_event = corr.events.iter().find(|&x| x.date == date);
                let mut open_trade = open_trade_mutex
                    .lock()
                    .expect("Failed to lock open trade mutex");

                // confluent PFS direction on this date
                match pfs_event {
                    Some(pfs_event) => {
                        if let Some(reversal) = &pfs_event.reversal {
                            match reversal {
                                // exit short, enter long
                                ReversalType::Low => {
                                    // exit short
                                    if let Some(trade) = &*open_trade {
                                        let mut trade = trade.clone();
                                        if trade.order == Order::Short
                                            || stop_triggered(
                                                &trade.order,
                                                &trade.trailing_stop,
                                                &trade.stop_loss,
                                                candle,
                                            )
                                        {
                                            trade.exit(date, candle.close);
                                            backtest.add_trade(trade);
                                            *open_trade = None;
                                        }
                                    }
                                    // enter long
                                    let qty = Trade::trade_quantity(capital, candle.close);
                                    let trailing_stop = Trade::calc_trailing_stop(
                                        Order::Long,
                                        candle.close,
                                        trailing_stop_type,
                                        trailing_stop,
                                    );
                                    let stop_loss = Trade::calc_stop_loss(
                                        Order::Long,
                                        candle.close,
                                        stop_loss_pct,
                                    );
                                    *open_trade = Some(Trade::new(
                                        date,
                                        Order::Long,
                                        qty,
                                        candle.close,
                                        capital,
                                        Some(trailing_stop),
                                        Some(stop_loss),
                                    ));
                                }
                                // exit long, enter short
                                ReversalType::High => {
                                    // exit long
                                    if let Some(trade) = &*open_trade {
                                        // clone is ok because value is overwritten after this block
                                        let mut trade = trade.clone();
                                        if trade.order == Order::Long
                                            || stop_triggered(
                                                &trade.order,
                                                &trade.trailing_stop,
                                                &trade.stop_loss,
                                                candle,
                                            )
                                        {
                                            trade.exit(date, candle.close);
                                            backtest.add_trade(trade);
                                            *open_trade = None;
                                        }
                                    }
                                    // enter short
                                    let qty = Trade::trade_quantity(capital, candle.close);
                                    let trailing_stop = Trade::calc_trailing_stop(
                                        Order::Short,
                                        candle.close,
                                        trailing_stop_type,
                                        trailing_stop,
                                    );
                                    let stop_loss = Trade::calc_stop_loss(
                                        Order::Short,
                                        candle.close,
                                        stop_loss_pct,
                                    );
                                    *open_trade = Some(Trade::new(
                                        date,
                                        Order::Short,
                                        qty,
                                        candle.close,
                                        capital,
                                        Some(trailing_stop),
                                        Some(stop_loss),
                                    ));
                                }
                            }
                        }
                    }
                    // if no event, check trailing stop
                    // if trailing stop is hit, exit trade
                    // other update trailing stop
                    None => {
                        debug!("No PFS Direction: {}", date.to_string_daily());
                        if let Some(trade) = &*open_trade {
                            match trade.order {
                                Order::Long => {
                                    // Long trailing stop is hit, exit trade
                                    if stop_triggered(
                                        &Order::Long,
                                        &trade.trailing_stop,
                                        &trade.stop_loss,
                                        candle,
                                    ) {
                                        let mut trade = trade.clone();
                                        trade.exit(date, candle.close);
                                        backtest.add_trade(trade);
                                        *open_trade = None;
                                    }
                                    // Long trailing stop is not hit, update trailing stop
                                    else {
                                        let mut trade = trade.clone();
                                        trade.trailing_stop = Some(Trade::calc_trailing_stop(
                                            Order::Long,
                                            candle.close,
                                            trailing_stop_type,
                                            trailing_stop,
                                        ));
                                        *open_trade = Some(trade);
                                    }
                                }
                                Order::Short => {
                                    // Short trailing stop is hit, exit trade
                                    if stop_triggered(
                                        &Order::Short,
                                        &trade.trailing_stop,
                                        &trade.stop_loss,
                                        candle,
                                    ) {
                                        let mut trade = trade.clone();
                                        trade.exit(date, candle.close);
                                        backtest.add_trade(trade);
                                        *open_trade = None;
                                    }
                                    // Short trailing stop is not hit, update trailing stop
                                    else {
                                        let mut trade = trade.clone();
                                        trade.trailing_stop = Some(Trade::calc_trailing_stop(
                                            Order::Short,
                                            candle.close,
                                            trailing_stop_type,
                                            trailing_stop,
                                        ));
                                        *open_trade = Some(trade);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            backtest.summarize();
            (backtest, corr.cycles)
        });
        threads.push(thread);
    }
    for thread in threads {
        let backtest = thread
            .join()
            .expect("Failed to join PFS confluent direction backtest thread");
        backtests.push(backtest);
    }
    backtests.sort_by(|a, b| b.0.pnl.partial_cmp(&a.0.pnl).unwrap());
    write_backtest_csv(backtests.clone(), backtest_file)
        .expect("Failed to write PFS confluent direction backtest to CSV");
    backtests
}
