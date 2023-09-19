use ephemeris::PLPLError;
use log::error;
use serde::Deserialize;
use std::env::VarError;
use std::num::ParseFloatError;
use std::str::ParseBoolError;
use std::sync::PoisonError;
use std::time::SystemTimeError;

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceContentError {
    pub code: i16,
    pub msg: String,
}

#[derive(Debug)]
pub enum BinanceError {
    Binance(BinanceContentError),
    KlineMissing,
    NoActiveOrder,
    SideInvalid,
    OrderTypeInvalid,
    WebSocketDisconnected,
    Reqwest(reqwest::Error),
    InvalidHeader(reqwest::header::InvalidHeaderValue),
    Io(std::io::Error),
    ParseFloat(ParseFloatError),
    ParseBool(ParseBoolError),
    UrlParser(url::ParseError),
    Json(serde_json::Error),
    Tungstenite(tungstenite::Error),
    Time(std::time::SystemTimeError),
    OrderStatusParseError(String),
    PLPL(PLPLError),
    Custom(String),
    SystemTime(SystemTimeError),
    EnvMissing(VarError),
    ExitHandlersInitializedEarly,
    ExitHandlersNotBothInitialized,
}

impl std::fmt::Display for BinanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinanceError::Binance(e) => {
                error!("Binance error: {:?}", e.msg);
                write!(f, "Binance error: {:?}", e.msg)
            }
            BinanceError::KlineMissing => {
                error!("Kline missing");
                write!(f, "Kline missing")
            }
            BinanceError::NoActiveOrder => {
                error!("No active order");
                write!(f, "No active order")
            }
            BinanceError::SideInvalid => {
                error!("Order Side invalid");
                write!(f, "Order Side invalid")
            }
            BinanceError::OrderTypeInvalid => {
                error!("OrderType invalid");
                write!(f, "OrderType invalid")
            }
            BinanceError::WebSocketDisconnected => {
                error!("WebSocket disconnected");
                write!(f, "WebSocket disconnected")
            }
            BinanceError::Reqwest(e) => {
                error!("Reqwest error: {:?}", e);
                write!(f, "Reqwest error: {:?}", e)
            }
            BinanceError::InvalidHeader(e) => {
                error!("Invalid header: {:?}", e);
                write!(f, "Invalid header: {:?}", e)
            }
            BinanceError::Io(e) => {
                error!("IO error: {:?}", e);
                write!(f, "IO error: {:?}", e)
            }
            BinanceError::ParseFloat(e) => {
                error!("Parse float error: {:?}", e);
                write!(f, "Parse float error: {:?}", e)
            }
            BinanceError::ParseBool(e) => {
                error!("Parse bool error: {:?}", e);
                write!(f, "Parse bool error: {:?}", e)
            }
            BinanceError::UrlParser(e) => {
                error!("URL parser error: {:?}", e);
                write!(f, "URL parser error: {:?}", e)
            }
            BinanceError::Json(e) => {
                error!("JSON error: {:?}", e);
                write!(f, "JSON error: {:?}", e)
            }
            BinanceError::Tungstenite(e) => {
                error!("Tungstenite error: {:?}", e);
                write!(f, "Tungstenite error: {:?}", e)
            }
            BinanceError::Time(e) => {
                error!("Time error: {:?}", e);
                write!(f, "Time error: {:?}", e)
            }
            BinanceError::OrderStatusParseError(e) => {
                error!("Order status parse error: {:?}", e);
                write!(f, "Order status parse error: {:?}", e)
            }
            BinanceError::PLPL(e) => {
                error!("PLPL error: {:?}", e);
                write!(f, "PLPL error: {:?}", e)
            }
            BinanceError::Custom(e) => {
                error!("Custom error: {:?}", e);
                write!(f, "Custom error: {:?}", e)
            }
            BinanceError::SystemTime(e) => {
                error!("System time error: {:?}", e);
                write!(f, "System time error: {:?}", e)
            }
            BinanceError::EnvMissing(e) => {
                error!("Env var missing: {:?}", e);
                write!(f, "Env var missing: {:?}", e)
            }
            BinanceError::ExitHandlersInitializedEarly => {
                error!("Exit handlers initialized before order placement");
                write!(f, "Exit handlers initialized before order placement")
            }
            BinanceError::ExitHandlersNotBothInitialized => {
                error!("Exit handlers not both initialized");
                write!(f, "Exit handlers not both initialized")
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, BinanceError>;

impl From<SystemTimeError> for BinanceError {
    fn from(e: SystemTimeError) -> Self {
        BinanceError::SystemTime(e)
    }
}

impl From<PLPLError> for BinanceError {
    fn from(e: PLPLError) -> Self {
        BinanceError::PLPL(e)
    }
}

impl<T> From<PoisonError<T>> for BinanceError {
    fn from(e: PoisonError<T>) -> Self {
        BinanceError::Custom(format!("Poison error: {:?}", e))
    }
}

// .parse::<f64>() impl From for BinanceError
impl From<ParseFloatError> for BinanceError {
    fn from(e: ParseFloatError) -> Self {
        BinanceError::ParseFloat(e)
    }
}

// .parse::<bool>() impl From for BinanceError
impl From<ParseBoolError> for BinanceError {
    fn from(e: ParseBoolError) -> Self {
        BinanceError::ParseBool(e)
    }
}

impl From<VarError> for BinanceError {
    fn from(e: VarError) -> Self {
        BinanceError::EnvMissing(e)
    }
}

impl From<std::io::Error> for BinanceError {
    fn from(e: std::io::Error) -> Self {
        BinanceError::Io(e)
    }
}

impl From<tungstenite::Error> for BinanceError {
    fn from(e: tungstenite::Error) -> Self {
        BinanceError::Tungstenite(e)
    }
}

impl From<url::ParseError> for BinanceError {
    fn from(e: url::ParseError) -> Self {
        BinanceError::UrlParser(e)
    }
}

impl From<serde_json::Error> for BinanceError {
    fn from(e: serde_json::Error) -> Self {
        BinanceError::Json(e)
    }
}

impl From<reqwest::Error> for BinanceError {
    fn from(e: reqwest::Error) -> Self {
        BinanceError::Reqwest(e)
    }
}

impl From<reqwest::header::InvalidHeaderValue> for BinanceError {
    fn from(e: reqwest::header::InvalidHeaderValue) -> Self {
        BinanceError::InvalidHeader(e)
    }
}
