use log::error;
use serde::Deserialize;

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
    ParseFloat(std::num::ParseFloatError),
    UrlParser(url::ParseError),
    Json(serde_json::Error),
    Tungstenite(tungstenite::Error),
    Time(std::time::SystemTimeError),
    OrderStatusParseError(String),
    Custom(String),
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
            BinanceError::Custom(e) => {
                error!("Custom error: {:?}", e);
                write!(f, "Custom error: {:?}", e)
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, BinanceError>;
