use std::fmt::{Display, Formatter, Result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AlpacaError {
    TimeError(time_series::TimeError),
    EnvReadError(std::env::VarError),
    AlpacaError(apca::Error),
}

impl Display for AlpacaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            AlpacaError::TimeError(e) => write!(f, "TimeError: {}", e),
            AlpacaError::EnvReadError(e) => write!(f, "EnvReadError: {}", e),
            AlpacaError::AlpacaError(e) => write!(f, "AlpacaError: {}", e),
        }
    }
}

pub type AlpacaResult<T> = std::result::Result<T, AlpacaError>;
