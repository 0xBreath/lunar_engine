use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Deserialize)]
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
    ParseFloat(std::num::ParseFloatError),
    UrlParser(url::ParseError),
    Json(serde_json::Error),
    Tungstenite(tungstenite::Error),
    Time(std::time::SystemTimeError),
    OrderStatusParseError(String),
    Custom(String)
}

impl std::fmt::Display for BinanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinanceError::Binance(e) => write!(f, "Binance error: {:?}", e.msg),
            BinanceError::KlineMissing => write!(f, "Kline missing"),
            BinanceError::NoActiveOrder => write!(f, "No active order"),
            BinanceError::SideInvalid => write!(f, "Order Side invalid"),
            BinanceError::OrderTypeInvalid => write!(f, "OrderType invalid"),
            BinanceError::WebSocketDisconnected => write!(f, "WebSocket disconnected"),
            BinanceError::Reqwest(e) => write!(f, "Reqwest error: {:?}", e),
            BinanceError::InvalidHeader(e) => write!(f, "Invalid header: {:?}", e),
            BinanceError::Io(e) => write!(f, "IO error: {:?}", e),
            BinanceError::ParseFloat(e) => write!(f, "Parse float error: {:?}", e),
            BinanceError::UrlParser(e) => write!(f, "URL parser error: {:?}", e),
            BinanceError::Json(e) => write!(f, "JSON error: {:?}", e),
            BinanceError::Tungstenite(e) => write!(f, "Tungstenite error: {:?}", e),
            BinanceError::Time(e) => write!(f, "Time error: {:?}", e),
            BinanceError::OrderStatusParseError(e) => {
                write!(f, "Order status parse error: {:?}", e)
            }
            BinanceError::Custom(e) => write!(f, "Custom error: {:?}", e),
        }
    }
}

pub type Result<T> = std::result::Result<T, BinanceError>;
