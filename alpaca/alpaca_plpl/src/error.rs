use apca::api::v2::{account, order};
use apca::RequestError;
use log::error;
use std::fmt::Debug;
use std::sync::PoisonError;

#[derive(Debug)]
pub enum AlpacaError {
    PLPL(ephemeris::PLPLError),
    Apca(apca::Error),
    Logger(log::SetLoggerError),
    Io(std::io::Error),
    ParseFloat(std::num::ParseFloatError),
    ParseBool(std::str::ParseBoolError),
    Json(serde_json::Error),
    Time(std::time::SystemTimeError),
    Custom(String),
    EnvMissing(std::env::VarError),
    WebSocket(tungstenite::Error),
    ApcaPostOrder(RequestError<order::PostError>),
    ApcaGetOrder(RequestError<order::GetError>),
    ApcaDeleteOrder(RequestError<order::DeleteError>),
    ApcaGetAccount(RequestError<account::GetError>),
    NumUnwrap,
    NoEntryOrder,
    NoExitOrder,
    BothExitsFilled,
    ExitNotCanceled,
    InvalidActiveOrder,
    ParseLevelError(log::ParseLevelError),
}

impl std::fmt::Display for AlpacaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlpacaError::PLPL(e) => {
                error!("PLPL error: {:?}", e);
                write!(f, "PLPL error: {:?}", e)
            }
            AlpacaError::Apca(e) => {
                error!("Apca error: {:?}", e);
                write!(f, "Apca error: {:?}", e)
            }
            AlpacaError::Logger(e) => {
                error!("Logger error: {:?}", e);
                write!(f, "Logger error: {:?}", e)
            }
            AlpacaError::Io(e) => {
                error!("IO error: {:?}", e);
                write!(f, "IO error: {:?}", e)
            }
            AlpacaError::ParseFloat(e) => {
                error!("ParseFloat error: {:?}", e);
                write!(f, "ParseFloat error: {:?}", e)
            }
            AlpacaError::ParseBool(e) => {
                error!("ParseBool error: {:?}", e);
                write!(f, "ParseBool error: {:?}", e)
            }
            AlpacaError::Json(e) => {
                error!("Json error: {:?}", e);
                write!(f, "Json error: {:?}", e)
            }
            AlpacaError::Time(e) => {
                error!("Time error: {:?}", e);
                write!(f, "Time error: {:?}", e)
            }
            AlpacaError::Custom(e) => {
                error!("Custom error: {:?}", e);
                write!(f, "Custom error: {:?}", e)
            }
            AlpacaError::EnvMissing(e) => {
                error!("EnvMissing error: {:?}", e);
                write!(f, "EnvMissing error: {:?}", e)
            }
            AlpacaError::WebSocket(e) => {
                error!("WebSocket error: {:?}", e);
                write!(f, "WebSocket error: {:?}", e)
            }
            AlpacaError::ApcaPostOrder(e) => {
                error!("Apca post order error: {:?}", e);
                write!(f, "Apca post order error: {:?}", e)
            }
            AlpacaError::ApcaGetOrder(e) => {
                error!("Apca get order error: {:?}", e);
                write!(f, "Apca get order error: {:?}", e)
            }
            AlpacaError::ApcaDeleteOrder(e) => {
                error!("Apca delete order error: {:?}", e);
                write!(f, "Apca delete order error: {:?}", e)
            }
            AlpacaError::ApcaGetAccount(e) => {
                error!("Apca get account error: {:?}", e);
                write!(f, "Apca get account error: {:?}", e)
            }
            AlpacaError::NumUnwrap => {
                error!("Failed to unwrap f64 to Num");
                write!(f, "Failed to unwrap f64 to Num")
            }
            AlpacaError::NoEntryOrder => {
                error!("No entry order found");
                write!(f, "No entry order found")
            }
            AlpacaError::NoExitOrder => {
                error!("Both exits should exist or not exist");
                write!(f, "Both exits should exist or not exist")
            }
            AlpacaError::BothExitsFilled => {
                error!("Both exit orders filled");
                write!(f, "Both exit orders filled")
            }
            AlpacaError::ExitNotCanceled => {
                error!("Exit order not canceled");
                write!(f, "Exit order not canceled")
            }
            AlpacaError::InvalidActiveOrder => {
                error!("Invalid active order");
                write!(f, "Invalid active order")
            }
            AlpacaError::ParseLevelError(e) => {
                error!("ParseLevelError: {:?}", e);
                write!(f, "ParseLevelError: {:?}", e)
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, AlpacaError>;

impl From<ephemeris::PLPLError> for AlpacaError {
    fn from(e: ephemeris::PLPLError) -> Self {
        AlpacaError::PLPL(e)
    }
}

impl From<apca::Error> for AlpacaError {
    fn from(e: apca::Error) -> Self {
        AlpacaError::Apca(e)
    }
}

impl From<log::SetLoggerError> for AlpacaError {
    fn from(e: log::SetLoggerError) -> Self {
        AlpacaError::Logger(e)
    }
}

impl From<std::io::Error> for AlpacaError {
    fn from(e: std::io::Error) -> Self {
        AlpacaError::Io(e)
    }
}

impl From<std::num::ParseFloatError> for AlpacaError {
    fn from(e: std::num::ParseFloatError) -> Self {
        AlpacaError::ParseFloat(e)
    }
}

impl From<std::str::ParseBoolError> for AlpacaError {
    fn from(e: std::str::ParseBoolError) -> Self {
        AlpacaError::ParseBool(e)
    }
}

impl From<serde_json::Error> for AlpacaError {
    fn from(e: serde_json::Error) -> Self {
        AlpacaError::Json(e)
    }
}

impl From<std::time::SystemTimeError> for AlpacaError {
    fn from(e: std::time::SystemTimeError) -> Self {
        AlpacaError::Time(e)
    }
}

impl From<std::env::VarError> for AlpacaError {
    fn from(e: std::env::VarError) -> Self {
        AlpacaError::EnvMissing(e)
    }
}

impl<T> From<PoisonError<T>> for AlpacaError {
    fn from(e: PoisonError<T>) -> Self {
        AlpacaError::Custom(format!("Poison error: {:?}", e))
    }
}

impl<T: Debug, E: Debug> From<std::result::Result<T, E>> for AlpacaError {
    fn from(e: std::result::Result<T, E>) -> Self {
        AlpacaError::Custom(format!("Result error: {:?}", e))
    }
}

impl From<tungstenite::Error> for AlpacaError {
    fn from(e: tungstenite::Error) -> Self {
        AlpacaError::WebSocket(e)
    }
}

impl From<log::ParseLevelError> for AlpacaError {
    fn from(e: log::ParseLevelError) -> Self {
        AlpacaError::ParseLevelError(e)
    }
}

impl From<RequestError<order::PostError>> for AlpacaError {
    fn from(e: RequestError<order::PostError>) -> Self {
        AlpacaError::ApcaPostOrder(e)
    }
}

impl From<RequestError<order::GetError>> for AlpacaError {
    fn from(e: RequestError<order::GetError>) -> Self {
        AlpacaError::ApcaGetOrder(e)
    }
}

impl From<RequestError<order::DeleteError>> for AlpacaError {
    fn from(e: RequestError<order::DeleteError>) -> Self {
        AlpacaError::ApcaDeleteOrder(e)
    }
}

impl From<RequestError<account::GetError>> for AlpacaError {
    fn from(e: RequestError<account::GetError>) -> Self {
        AlpacaError::ApcaGetAccount(e)
    }
}
