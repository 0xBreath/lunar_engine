use log::*;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use time_series::*;

#[tokio::main]
async fn main() -> CycleResult<()> {
    init_logger();

    let path_to_dir = env::var("PATH_TO_DIR").expect("PATH_TO_DIR not set");

    // BTCUSD
    #[allow(unused_variables)]
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/input/BTC_daily.csv";
    #[allow(unused_variables)]
    let btc_1h = path_to_dir.clone() + "/data/BTCUSD/input/BTC_1h.csv";
    #[allow(unused_variables)]
    let btc_5min = path_to_dir.clone() + "/data/BTCUSD/input/BTC_5min.csv";

    // let out_file = PathBuf::from(path_to_dir + "/data/BTCUSD/output/BTC_cycle_results.txt");

    // let cycle_timeframe = CycleTimeframe::Month;
    // let cycles_to_test = (1..=50).collect::<Vec<u64>>();
    // let harmonic_cycles: Vec<u64> = vec![1, 2, 3, 4, 5, 6, 8, 12, 16];
    // let best_composites: Vec<(CompositeCycle, CycleCorrelation)> = single_sine_harmonics_composite(
    //     &cycles_to_test,
    //     &harmonic_cycles,
    //     cycle_timeframe.clone(),
    //     &PathBuf::from(btc_daily.clone()),
    //     &out_file,
    // )?;

    // let cycles_to_test = vec![11, 23, 33, 37, 14, 38, 21];
    // multi_sine_composite(
    //     &cycles_to_test,
    //     cycle_timeframe,
    //     &PathBuf::from(btc_daily),
    //     &out_file,
    // )?;

    /*
    TODO:
       return 5 best single_sine_harmonics_composite
       compute composite based on those 5 combinations
    */

    Ok(())
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
fn single_sine_harmonics_composite(
    cycles_to_test: &[u64],
    harmonic_cycles: &[u64],
    cycle_timeframe: CycleTimeframe,
    btc_daily: &PathBuf,
    out_file: &PathBuf,
) -> CycleResult<Vec<(CompositeCycle, CycleCorrelation)>> {
    // BTC daily
    let mut ticker_data_daily = TickerData::new();
    ticker_data_daily
        .add_csv_series(&PathBuf::from(btc_daily))
        .expect("Failed to add BTC daily CSV to TickerData");

    // start sine cycles at first major low in time series (candle range should be high to get high timeframe low)
    let market_structure = MarketStructure::new(&ticker_data_daily, 200);
    let lows: Vec<&Reversal> = market_structure
        .reversals
        .iter()
        .filter(|r| r.reversal_type == ReversalType::Low)
        .collect();
    // get second elements in lows vector
    let first_low = match lows.get(0) {
        Some(low) => low,
        None => return Err(CycleError::EmptySeries),
    };

    info!("Major Low: {}", first_low.candle.date.to_string());

    let start_date = first_low.candle.date;
    let end_date = match ticker_data_daily.candles.last() {
        Some(candle) => candle.date,
        None => return Err(CycleError::EmptySeries),
    };
    if end_date < start_date {
        return Err(CycleError::EndDateBeforeStartDate);
    }

    // build sine cycles ranging from 1-60 months
    let cycles = cycles_to_test
        .iter()
        .map(|x| {
            let period = cycle_timeframe.timeframe_to_period_minutes(*x as f32);
            SineCycle::new(start_date, end_date, period, cycle_timeframe.clone())
        })
        .collect::<Vec<SineCycle>>();

    // single sine wave correlation to time series
    let mut correlations = Vec::new();
    for cycle in cycles.into_iter() {
        let correlation = cycle.series_correlation(&market_structure)?;
        correlations.push((cycle, correlation));
    }
    // sort by highest correlation
    correlations.sort_by(|a, b| b.1.correlation.partial_cmp(&a.1.correlation).unwrap());

    let mut best_composites: Vec<(CompositeCycle, CycleCorrelation)> = Vec::new();
    // for each highly correlated sine wave, build a composite based on its harmonics (1/2, 1/3, etc)
    // test each combination of harmonic cycles summed the correlated sine wave
    // to discover a more correlated composite wave
    let mut file = File::create(out_file).map_err(CycleError::CustomError)?;
    for (cycle, _) in correlations.iter().take(3) {
        // get all combinations of harmonic cycles
        // compute composite wave for each combination
        let composites = CompositeCycle::cycle_combinations(
            &start_date,
            &end_date,
            harmonic_cycles,
            cycle_timeframe.clone(),
        )?;

        // for each composite wave compute correlation to time series
        let mut composite_correlations: Vec<(CompositeCycle, CycleCorrelation)> = Vec::new();
        for composite in composites {
            let correlation = composite.series_correlation(&market_structure)?;
            composite_correlations.push((composite, correlation));
        }
        // sort by highest correlation to time series
        composite_correlations
            .sort_by(|a, b| b.1.correlation.partial_cmp(&a.1.correlation).unwrap());

        let best_harmonics_composite = match composite_correlations.first() {
            Some(composite) => composite,
            None => return Err(CycleError::EmptySeries),
        };
        // save best composite for this sine cycle and harmonics
        best_composites.push(best_harmonics_composite.clone());

        let period = cycle_timeframe.period_minutes_to_timeframe(cycle.cycle_period_minutes);
        let period_name = cycle_timeframe.to_str();

        // log 3 highest composite correlations
        info!("{} Period: {}", period_name, period);
        writeln!(file, "{} Period: {}", period_name, period).map_err(CycleError::CustomError)?;
        for (cycle, composite) in composite_correlations.iter().take(3) {
            let cycles = cycle
                .sine_cycles
                .iter()
                .map(|x| {
                    x.timeframe
                        .period_minutes_to_timeframe(x.cycle_period_minutes)
                })
                .collect::<Vec<f32>>();
            info!(
                "Composite: {:?}, Correlation: {}, Hits: {}, Total: {}",
                cycles, composite.correlation, composite.correlated, composite.total
            );
            writeln!(
                file,
                "Composite: {:?}, Correlation: {}, Hits: {}, Total: {}",
                cycles, composite.correlation, composite.correlated, composite.total
            )
            .map_err(CycleError::CustomError)?;
        }
        info!("==========================================================");
        writeln!(
            file,
            "=========================================================="
        )
        .map_err(CycleError::CustomError)?;
    }

    Ok(best_composites)
}

#[allow(dead_code)]
fn multi_sine_composite(
    cycles_to_test: &[u64],
    cycle_timeframe: CycleTimeframe,
    btc_daily: &PathBuf,
    out_file: &PathBuf,
) -> CycleResult<()> {
    // BTC daily
    let mut ticker_data_daily = TickerData::new();
    ticker_data_daily
        .add_csv_series(&PathBuf::from(btc_daily))
        .expect("Failed to add BTC daily CSV to TickerData");

    // start sine cycles at first major low in time series (candle range should be high to get high timeframe low)
    let market_structure = MarketStructure::new(&ticker_data_daily, 200);
    let lows: Vec<&Reversal> = market_structure
        .reversals
        .iter()
        .filter(|r| r.reversal_type == ReversalType::Low)
        .collect();
    // get second elements in lows vector
    let first_low = match lows.get(0) {
        Some(low) => low,
        None => return Err(CycleError::EmptySeries),
    };

    info!("Major Low: {}", first_low.candle.date.to_string());

    let start_date = first_low.candle.date;
    let end_date = match ticker_data_daily.candles.last() {
        Some(candle) => candle.date,
        None => return Err(CycleError::EmptySeries),
    };
    if end_date < start_date {
        return Err(CycleError::EndDateBeforeStartDate);
    }

    // get all combinations of sine cycles
    // compute composite wave for each combination
    let combinations = CompositeCycle::cycle_combinations(
        &start_date,
        &end_date,
        cycles_to_test,
        cycle_timeframe,
    )?;

    // for each composite wave compute correlation to time series
    let mut composite_correlations = Vec::new();
    for comb in combinations {
        let correlation = comb.series_correlation(&market_structure)?;
        let periods = comb
            .sine_cycles
            .iter()
            .map(|x| {
                x.timeframe
                    .period_minutes_to_timeframe(x.cycle_period_minutes)
            })
            .collect::<Vec<f32>>();
        debug!(
            "Composite: {:?}, Correlation: {}, Hits: {}, Total: {}",
            periods, correlation.correlation, correlation.correlated, correlation.total
        );
        composite_correlations.push((comb, correlation));
    }
    info!("-----------------------------------");

    // sort by highest correlation to time series
    composite_correlations.sort_by(|a, b| b.1.correlation.partial_cmp(&a.1.correlation).unwrap());

    // find all combinations of sine waves
    // build a composite based on each sine wave combination
    // test correlation of each composite wave to time series
    let mut file = File::create(out_file).map_err(CycleError::CustomError)?;
    // log 5 highest composite correlations
    for (cycle, composite) in composite_correlations.into_iter().take(5) {
        let period = cycle
            .sine_cycles
            .iter()
            .map(|x| {
                x.timeframe
                    .period_minutes_to_timeframe(x.cycle_period_minutes)
            })
            .collect::<Vec<f32>>();
        info!(
            "Composite: {:?}, Correlation: {}, Hits: {}, Total: {}",
            period, composite.correlation, composite.correlated, composite.total
        );
        writeln!(
            file,
            "Composite: {:?}, Correlation: {}, Hits: {}, Total: {}",
            period, composite.correlation, composite.correlated, composite.total
        )
        .map_err(CycleError::CustomError)?;
    }
    info!("==========================================================");
    writeln!(
        file,
        "=========================================================="
    )
    .map_err(CycleError::CustomError)?;

    Ok(())
}
