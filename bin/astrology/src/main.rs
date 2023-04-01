use log::*;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
// use time_series::*;
// use ephemeris::*;

#[tokio::main]
async fn main() {
    init_logger();
    // TODO
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
