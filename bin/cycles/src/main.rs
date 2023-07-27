use log::*;
use simplelog::{ColorChoice, Config as SimpleLogConfig, TermLogger, TerminalMode};
use std::path::PathBuf;
use time_series::*;

fn init_logger() {
    TermLogger::init(
        LevelFilter::Info,
        SimpleLogConfig::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");
}

fn median(data: &[f32]) -> CycleResult<f32> {
    let mut data = data.to_vec();
    data.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let count = data.len();
    if count > 0 {
        let mid = count / 2;
        if count % 2 == 0 {
            Ok((data[mid - 1] + data[mid]) / 2.0)
        } else {
            Ok(data[mid])
        }
    } else {
        Err(CycleError::EmptyMedian)
    }
}

fn mean(data: &[f32]) -> CycleResult<f32> {
    let sum = data.iter().sum::<f32>();
    let count = data.len();
    if count > 0 {
        Ok(sum / count as f32)
    } else {
        Err(CycleError::EmptyMedian)
    }
}

/// Mean is replaced by median to avoid outliers
fn std_dev(data: &[f32]) -> CycleResult<f32> {
    let mean = match median(data) {
        Ok(mean) => mean,
        Err(e) => return Err(e),
    };
    let count = data.len();
    if count > 0 {
        let variance = data
            .iter()
            .map(|value| {
                let diff = mean - *value;

                diff * diff
            })
            .sum::<f32>()
            / count as f32;

        Ok(variance.sqrt())
    } else {
        Err(CycleError::EmptyStdDev)
    }
}

/// Use this to compute cycle averages
/// Median is used instead of mean for datasets with outliers
/// Median of a range of cycle deviations is a better metric than mean which is skewed by outliers
fn zscore_median(data: &[f32], index: usize) -> CycleResult<f32> {
    let median = match median(data) {
        Ok(median) => median,
        Err(e) => return Err(e),
    };
    let std_dev = match std_dev(data) {
        Ok(std_deviation) => std_deviation,
        Err(e) => return Err(e),
    };
    let diff = data[index] - median;
    Ok(diff / std_dev)
}

#[derive(Debug, Clone)]
pub struct CycleLow {
    pub first: Candle,
    pub second: Candle,
    pub period_minutes: u64,
    pub timeframe: CycleTimeframe,
    pub period_timeframe: f32,
}

fn cycle_lows(
    ticker: &TickerData,
    candle_range: usize,
    timeframe: CycleTimeframe,
) -> CycleResult<Vec<CycleLow>> {
    let lows = ticker.pivot_lows(candle_range, candle_range);
    // get difference in days between each low
    let low_periods = lows
        .windows(2)
        .map(|window| {
            let low1 = &window[0];
            let low2 = &window[1];
            let period = low1
                .date
                .diff_minutes(&low2.date)
                .map_err(CycleError::TimeError)? as u64;
            let period_timeframe = timeframe.period_minutes_to_timeframe(period);
            let candle1 = low1.clone();
            let candle2 = low2.clone();
            Ok(CycleLow {
                first: candle1,
                second: candle2,
                period_minutes: period,
                timeframe: timeframe.clone(),
                period_timeframe,
            })
        })
        .collect::<CycleResult<Vec<CycleLow>>>();
    low_periods
}

fn filter_cycle_lows(cycle_lows: Vec<CycleLow>) -> CycleResult<Vec<CycleLow>> {
    let mut filtered = Vec::new();
    let periods = cycle_lows
        .iter()
        .map(|cycle_low| cycle_low.period_timeframe)
        .collect::<Vec<f32>>();
    for (index, value) in cycle_lows.iter().enumerate() {
        let zscore = zscore_median(&periods, index)?;
        if zscore.abs() < 1.01 {
            filtered.push(value.clone());
        }
    }
    Ok(filtered)
}

fn cycle_period(
    ticker: &TickerData,
    candle_range: usize,
    timeframe: CycleTimeframe,
) -> CycleResult<f32> {
    let low_periods = cycle_lows(ticker, candle_range, timeframe)?;
    let filtered = filter_cycle_lows(low_periods)?;
    let filtered_periods = filtered
        .iter()
        .map(|cycle_low| cycle_low.period_timeframe)
        .collect::<Vec<f32>>();

    let filtered_mean = mean(&filtered_periods)?;
    Ok(filtered_mean)
}

fn cycle_period_to_sine_cycle(
    cycle_period: f32,
    ticker: &TickerData,
    timeframe: CycleTimeframe,
) -> SineCycle {
    SineCycle {
        start_date: *ticker.earliest_date(),
        end_date: *ticker.latest_date(),
        cycle_period_minutes: timeframe.timeframe_to_period_minutes(cycle_period),
        timeframe,
    }
}

fn main() -> CycleResult<()> {
    init_logger();

    let path_to_dir = std::env::var("PATH_TO_DIR").expect("PATH_TO_DIR not set");

    // let dji_daily = path_to_dir.clone() + "/data/DJI/input/DJI_daily.csv";
    let dji_weekly = path_to_dir + "/data/DJI/input/DJI_weekly.csv";
    // let dji_monthly = path_to_dir + "/data/DJI/input/DJI_monthly.csv";

    // DJI daily cycle
    // let mut dji_daily_ticker = TickerData::new();
    // dji_daily_ticker
    //     .add_csv_series(&PathBuf::from(dji_daily))
    //     .expect("Failed to add DJI daily CSV to TickerData");
    // dji_daily_ticker.candles = dji_daily_ticker
    //     .candles
    //     .into_iter()
    //     .filter(|candle| candle.date.year >= 1965 && candle.date.year <= 1968)
    //     .collect::<Vec<Candle>>();
    // let dji_daily_cycle = cycle_period(&dji_daily_ticker, 35, CycleTimeframe::Week)?;
    // info!("DJI Daily cycle as weeks = {}", dji_daily_cycle);

    // DJI weekly cycle
    let mut dji_weekly_ticker = TickerData::new();
    dji_weekly_ticker
        .add_csv_series(&PathBuf::from(dji_weekly))
        .expect("Failed to add DJI weekly CSV to TickerData");

    // start sine cycles at first major low in time series (candle range should be high to get high timeframe low)
    let market_structure = MarketStructure::new(&dji_weekly_ticker, 100);
    let first_low = market_structure
        .first_low()
        .map_err(CycleError::MarketStructureError)?
        .candle
        .date;

    dji_weekly_ticker.candles = dji_weekly_ticker
        .candles
        .into_iter()
        .filter(|candle| candle.date >= first_low)
        .collect::<Vec<Candle>>();

    let weekly_candle_ranges = vec![3, 5, 8, 13, 16, 21, 25, 30, 36, 40, 49, 55];
    let mut weekly_cycles = Vec::new();
    let weekly_timeframe = CycleTimeframe::Week;
    for range in weekly_candle_ranges {
        let dji_weekly_cycle = cycle_period(&dji_weekly_ticker, range, weekly_timeframe)?;
        info!(
            "(range = {}) DJI weekly cycle = {}",
            range, dji_weekly_cycle
        );
        weekly_cycles.push(dji_weekly_cycle);

        // convert to SineCycle
        let sine_cycle =
            cycle_period_to_sine_cycle(dji_weekly_cycle, &dji_weekly_ticker, weekly_timeframe);
        let correlation = sine_cycle.series_correlation(&market_structure)?;
        info!("Correlation = {}", correlation.correlation);
    }

    // // DJI monthly cycle
    // let mut dji_monthly_ticker = TickerData::new();
    // dji_monthly_ticker
    //     .add_csv_series(&PathBuf::from(dji_monthly))
    //     .expect("Failed to add DJI monthly CSV to TickerData");
    //
    // // start sine cycles at first major low in time series (candle range should be high to get high timeframe low)
    // let market_structure = MarketStructure::new(&dji_weekly_ticker, 100);
    // let lows: Vec<&Reversal> = market_structure
    //   .reversals
    //   .iter()
    //   .filter(|r| r.reversal_type == ReversalType::Low)
    //   .collect();
    // let first_low = match lows.get(0) {
    //     Some(low) => low.candle.date,
    //     None => return Err(CycleError::EmptySeries),
    // };
    //
    // dji_monthly_ticker.candles = dji_monthly_ticker
    //     .candles
    //     .into_iter()
    //     .filter(|candle| candle.date >= first_low)
    //     .collect::<Vec<Candle>>();
    //
    // let monthly_candle_ranges = vec![3, 5, 8, 13, 16, 21, 25, 30, 36, 40, 49, 55];
    // let mut monthly_cycles = Vec::new();
    // let monthly_timeframe = CycleTimeframe::Month;
    // for range in monthly_candle_ranges {
    //     let dji_monthly_cycle = cycle_period(&dji_monthly_ticker, range, monthly_timeframe)?;
    //     info!(
    //         "(range = {}) DJI Monthly cycle = {}",
    //         range, dji_monthly_cycle
    //     );
    //     monthly_cycles.push(dji_monthly_cycle);
    //
    //     // convert to SineCycle
    //     let sine_cycle =
    //         cycle_period_to_sine_cycle(dji_monthly_cycle, &dji_monthly_ticker, monthly_timeframe);
    //     let correlation = sine_cycle.series_correlation(&market_structure)?;
    //     info!("Correlation = {}", correlation.correlation);
    // }

    Ok(())
}
