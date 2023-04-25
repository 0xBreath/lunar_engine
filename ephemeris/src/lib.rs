pub mod common;
pub mod toolkit;

pub use common::*;
pub use toolkit::*;

// input
pub const TICKER_DATA_PATH: &str = "BTCUSD.csv";
pub const SOLAR_ECLIPSE_CSV: &str = "./solar_eclipse.csv";
pub const LUNAR_ECLIPSE_CSV: &str = "./lunar_eclipse.csv";
// output
pub const PRICE_PLANET_RESULTS_PATH: &str = "price_planet_results.txt";
pub const PLANET_MATRIX_RESULTS_PATH: &str = "planet_matrix_results.txt";
pub const RETROGRADE_RESULTS_CSV: &str = "retrograde_results.csv";
pub const TICKER_DATAFRAME_CSV: &str = "ticker_dataframe.csv";
pub const ECLIPSES_DATAFRAME_CSV: &str = "./eclipses.csv";
// daily plpl for each planet as a csv
pub const MOON_EPHEMERIS: &str = "./moon_ephemeris.csv";
pub const SUN_EPHEMERIS: &str = "./sun_ephemeris.csv";
pub const MERCURY_EPHEMERIS: &str = "./mercury_ephemeris.csv";
pub const VENUS_EPHEMERIS: &str = "./venus_ephemeris.csv";
pub const MARS_EPHEMERIS: &str = "./mars_ephemeris.csv";
pub const JUPITER_EPHEMERIS: &str = "./jupiter_ephemeris.csv";
pub const SATURN_EPHEMERIS: &str = "./saturn_ephemeris.csv";
pub const URANUS_EPHEMERIS: &str = "./uranus_ephemeris.csv";
pub const NEPTUNE_EPHEMERIS: &str = "./neptune_ephemeris.csv";
pub const PLUTO_EPHEMERIS: &str = "./pluto_ephemeris.csv";



