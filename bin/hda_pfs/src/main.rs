use log::*;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::env;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use time_series::*;
use std::io::Write;

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

    // SPX
    let spx_daily = path_to_dir.clone() + "/data/SPX/input/SPX_daily.csv";
    #[allow(unused_variables)]
    let spx_history = path_to_dir.clone() + "/data/SPX/output/SPX_history.csv";
    let spx_confluent_direction_file = path_to_dir.clone() + "/data/SPX/output/SPX_PFS_confluent_direction.csv";
    let spx_hda_pfs_backtest_file = path_to_dir.clone() + "/data/SPX/output/SPX_HDA_PFS_confluent_direction_backtest.csv";

    let start_date = Time::new(start_year, &Month::from_num(start_month), &Day::from_num(start_day), None, None);
    let end_date = Time::new(end_year, &Month::from_num(end_month), &Day::from_num(end_day), None, None);

    // SPX ticker data
    let mut spx_ticker_data = TickerData::new();
    spx_ticker_data.build_series(
        "SPX",
        Interval::Daily,
        &PathBuf::from(spx_daily),
    ).await.expect("Failed to add SPX CSV series");

    // SPX HDA
    let hda = spx_hda(
        start_date,
        end_date,
        left_bars,
        right_bars,
        hda_margin,
        &spx_ticker_data,
    ).await;

    // TODO: take from ENV
    let timeframe = PFSTimeframe::Day;
    // SPX confluent PFS direction
    let conf_pfs_dir = spx_pfs_confluent_direction(
        start_date,
        end_date,
        &pfs_confluent_years,
        timeframe,
        &spx_ticker_data,
        spx_confluent_direction_file
    ).await;

    let capital = 1000.0;
    let mut backtests = Vec::<Backtest>::new();
    // iterate through PFS cycle combinations
    for corr in conf_pfs_dir.into_iter().take(5) {
        let mut open_trade: Option<Trade> = None;
        let mut backtest = Backtest::new(capital);

        // iterate time series
        for candle in spx_ticker_data.get_candles().iter() {
            let date = candle.date;
            // find confluent PFS event with this candle date
            let pfs_event = corr.events.iter().find(|&x| x.date == date);
            // find HDA on this candle date
            let hda = hda.iter().find(|&x| x.date == date);

            // confluent PFS direction and HDA on this date
            if let (Some(pfs_event), Some(hda)) = (pfs_event, hda) {
                if let Some(direction) = &pfs_event.direction {
                    if hda.mode > 0 {
                        match direction {
                            // exit short, enter long
                            Direction::Up => {
                                // exit short
                                if let Some(mut trade) = open_trade {
                                    if trade.order == Order::Short {
                                        trade.exit(date, candle.close);
                                        backtest.add_trade(trade);
                                    }
                                }
                                // enter long
                                let qty = Trade::trade_quantity(capital, candle.close);
                                open_trade = Some(Trade::new(
                                    date,
                                    Order::Long,
                                    qty,
                                    candle.close,
                                    capital,
                                    // TODO: trailing stop, stop loss
                                    None,
                                    None
                                ));
                            }
                            // exit long, enter short
                            Direction::Down => {
                                // exit long
                                if let Some(mut trade) = open_trade {
                                    if trade.order == Order::Long {
                                        trade.exit(date, candle.close);
                                        backtest.add_trade(trade);
                                    }
                                }
                                // enter short
                                let qty = Trade::trade_quantity(capital, candle.close);
                                open_trade = Some(Trade::new(
                                    date,
                                    Order::Short,
                                    qty,
                                    candle.close,
                                    capital,
                                    // TODO: trailing stop, stop loss
                                    None,
                                    None
                                ));
                            }
                        }
                    }
                }
            }
        }
        backtest.summarize();
        backtests.push(backtest);
    }
    backtests.sort_by(|a, b| b.pnl.partial_cmp(&a.pnl).unwrap());
    write_hda_pfs_backtest_csv(backtests, &spx_hda_pfs_backtest_file).expect("Failed to write PFS confluence backtest CSV");
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
async fn spx_hda(
    hda_start_date: Time,
    hda_end_date: Time,
    pivot_left_bars: usize,
    pivot_right_bars: usize,
    hda_margin: usize,
    ticker_data: &TickerData,
) -> Vec<HDA> {
    // ======================== Historical Date Analysis ============================
    let hda = PlotHDA::new(
        hda_start_date,
        hda_end_date,
        pivot_left_bars,
        pivot_right_bars,
        hda_margin,
    );
    // TODO: choose timeframe for intra-day trading
    hda.hda(ticker_data)
}

/// Expects SPX PFS to be run first to generate the SPX ticker history
#[allow(dead_code)]
async fn spx_pfs_confluent_direction(
    start_date: Time,
    end_date: Time,
    pfs_confluent_years: &[u32],
    timeframe: PFSTimeframe,
    ticker_data: &TickerData,
    pfs_confluence_file: String,
) -> Vec<ConfluentPFSCorrelation> {
    // ======================== Polarity Factor System ============================
    let pfs = PlotPFS::new(start_date, end_date);
    pfs.confluent_pfs_direction(ticker_data, pfs_confluent_years, timeframe, &pfs_confluence_file)
}

fn write_hda_pfs_backtest_csv(backtests: Vec<Backtest>, out_file: &str) -> Result<(), Box<dyn Error>> {
    if backtests.is_empty() {
        return Err("No backtests found".into())
    }
    let mut file = File::create(out_file)?;

    writeln!(file, "start_date,end_date,pnl,avg_trade,avg_win,avg_loss,win_trades,loss_trades,trades")?;
    for backtest in backtests.iter() {
        if backtest.trades.is_empty() {
            continue;
        }
        let start_date = backtest.start_date.expect("No start date found").to_string_daily();
        let end_date = backtest.end_date.expect("No end date found").to_string_daily();
        let pnl = backtest.pnl.unwrap_or(0.0);
        let avg_trade = backtest.avg_trade_pnl.unwrap_or(0.0);
        let avg_win = backtest.avg_win_trade_pnl.unwrap_or(0.0);
        let avg_loss = backtest.avg_loss_trade_pnl.unwrap_or(0.0);
        let win_trades = backtest.num_win_trades();
        let loss_trades = backtest.num_loss_trades();
        let trades = backtest.trades.len();
        writeln!(
            file, "{},{},{},{},{},{},{},{},{}",
            start_date, end_date, pnl, avg_trade, avg_win, avg_loss, win_trades, loss_trades, trades
        )?;
    }
    Ok(())
}


