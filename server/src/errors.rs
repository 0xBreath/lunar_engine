use std::error::Error;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BinanceContentError {
  pub code: i16,
  pub msg: String,
}

#[derive(Debug)]
pub enum BinanceError {
  Reqwest(reqwest::Error),
  Serde(serde_json::Error),
  Content(BinanceContentError),
  Other(String),
}

impl Error for BinanceError {}

impl std::fmt::Display for BinanceError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BinanceError::Reqwest(e) => write!(f, "BinanceError: {}", e),
      BinanceError::Serde(e) => write!(f, "BinanceError: {}", e),
      BinanceError::Content(e) => write!(f, "BinanceError: {}", e.msg),
      BinanceError::Other(e) => write!(f, "BinanceError: {}", e),
    }
  }
}

pub type Result<T> = std::result::Result<T, BinanceError>;