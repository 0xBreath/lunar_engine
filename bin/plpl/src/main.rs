use log::*;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use time_series::*;
use ephemeris::*;
use std::env;

#[tokio::main]
async fn main() {
    init_logger();

    // planet data settings
    let planet_input = env::var("PLANET").expect("PLANET not set");
    let year = env::var("YEAR")
      .expect("YEAR not set")
      .parse::<i32>()
      .expect("YEAR not a number");
    let month = env::var("MONTH")
      .expect("MONTH not set")
      .parse::<u32>()
      .expect("MONTH not a number");
    let day = env::var("DAY")
      .expect("DAY not set")
      .parse::<u32>()
      .expect("DAY not a number");

    let date = Time::new(year, &Month::from_num(month), &Day::from_num(day), None, None);

    // PLPL trade signal inputs
    let plpl_scale = env::var("PLPL_SCALE").expect("PLPL_SCALE not set")
      .parse::<f32>()
      .expect("Failed to parse PLPL_SCALE to float");
    let cross_margin_pct = env::var("CROSS_MARGIN_PCT").expect("CROSS_MARGIN_PCT not set")
      .parse::<f32>()
      .expect("Failed to parse CROSS_MARGIN_PCT to float");

    // trade settings
    #[allow(unused_variables)]
    let stop_loss_pct = env::var("STOP_LOSS_PCT")
      .expect("STOP_LOSS_PCT not set")
      .parse::<f64>()
      .expect("failed to parse stop loss pct to float");
    #[allow(unused_variables)]
    let ts_type = env::var("TRAILING_STOP_USE_PCT")
      .expect("TRAILING_STOP_USE_PCT not set")
      .parse::<bool>()
      .expect("failed to parse trailing stop type to bool");
    #[allow(unused_variables)]
    let ts = env::var("TRAILING_STOP")
      .expect("TRAILING_STOP not set")
      .parse::<f64>()
      .expect("failed to parse trailing stop to float");

    // BTCUSD ticker data file paths
    let path_to_dir = env::var("PATH_TO_DIR").expect("PATH_TO_DIR not set");
    #[allow(unused_variables)]
    let btc_daily = path_to_dir.clone() + "/data/BTCUSD/input/BTC_daily.csv";
    #[allow(unused_variables)]
    let btc_1h = path_to_dir.clone() + "/data/BTCUSD/input/BTC_1h.csv";
    #[allow(unused_variables)]
    let btc_5min = path_to_dir.clone() + "/data/BTCUSD/input/BTC_5min.csv";

    // get planet longitude
    let config = PLPLSystemConfig {
        planet: Planet::from(&*planet_input),
        origin: Origin::Heliocentric,
        date,
        plpl_scale,
        plpl_price: 20000.0,
        num_plpls: 2000,
        cross_margin_pct
    };
    let plpl_system = PLPLSystem::new(config).expect("Failed to init PLPL");

    let candle = Candle {
        date,
        open: 29628.0,
        high: 29643.0,
        low: 29579.0,
        close: 29599.0,
        volume: None,
    };
    let closest_plpl = plpl_system.closest_plpl(&candle).expect("Failed to get closest PLPL");
    println!("Closest PLPL: {}, Close: {}", closest_plpl, candle.close);

    // compute stop loss by percentage

    // compute trailing stop by pips or percentage



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
