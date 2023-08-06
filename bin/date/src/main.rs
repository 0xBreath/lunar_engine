use log::*;
use simplelog::{ColorChoice, Config as SimpleLogConfig, TermLogger, TerminalMode};
use time_series::Time;

fn init_logger() {
    TermLogger::init(
        LevelFilter::Info,
        SimpleLogConfig::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");
}
fn main() {
    init_logger();

    let create_timestamp = 1691275643358;
    let update_timestamp = 1691301557606;

    let create_date = Time::from_unix_msec(create_timestamp);
    let update_date = Time::from_unix_msec(update_timestamp);
    info!("create_date: {}", create_date.to_string());
    info!("update_date: {}", update_date.to_string());
}
