use crate::{AlpacaError, Result};
use log::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;

pub fn init_logger(log_file: &PathBuf) -> Result<()> {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            SimpleLogConfig::default(),
            TerminalMode::Mixed,
            ColorChoice::Always,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            ConfigBuilder::new().set_time_format_rfc3339().build(),
            File::create(log_file)?,
        ),
    ])
    .map_err(AlpacaError::Logger)
}
