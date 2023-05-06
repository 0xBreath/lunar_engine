use serde::Deserialize;
use error_chain::error_chain;

#[derive(Debug, Deserialize)]
pub struct BinanceContentError {
  pub code: i16,
  pub msg: String,
}

error_chain! {
    errors {
        BinanceError(response: BinanceContentError)

        KlineValueMissingError(index: usize, name: &'static str) {
            description("invalid Vec for Kline"),
            display("{} at {} is missing", name, index),
        }
     }

    foreign_links {
        ReqError(reqwest::Error);
        InvalidHeaderError(reqwest::header::InvalidHeaderValue);
        IoError(std::io::Error);
        ParseFloatError(std::num::ParseFloatError);
        UrlParserError(url::ParseError);
        Json(serde_json::Error);
        Tungstenite(tungstenite::Error);
        TimestampError(std::time::SystemTimeError);
    }
}




// use std::error::Error;
// use serde::Deserialize;
//
// #[derive(Debug, Deserialize)]
// pub struct BinanceContentError {
//   pub code: i16,
//   pub msg: String,
// }
//
// #[derive(Debug)]
// pub enum BinanceError {
//   Reqwest(reqwest::Error),
//   Serde(serde_json::Error),
//   Content(BinanceContentError),
//   Other(String),
// }
//
// impl Error for BinanceError {}
//
// impl std::fmt::Display for BinanceError {
//   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//     match self {
//       BinanceError::Reqwest(e) => write!(f, "BinanceError: {}", e),
//       BinanceError::Serde(e) => write!(f, "BinanceError: {}", e),
//       BinanceError::Content(e) => write!(f, "BinanceError: {}", e.msg),
//       BinanceError::Other(e) => write!(f, "BinanceError: {}", e),
//     }
//   }
// }
//
// pub type Result<T> = std::result::Result<T, BinanceError>;