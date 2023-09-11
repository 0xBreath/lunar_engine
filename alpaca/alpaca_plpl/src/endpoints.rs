
pub const ALPACA_API_PAPER_URL: &str = "https://paper-api.alpaca.markets";
pub const ALPACA_API_LIVE_URL: &str = "https://api.alpaca.markets";
/// Data API endpoints (paper or live)
pub const DATA_HTTP_URL: &str = "https://data.alpaca.markets";
pub const DATA_WS_URL: &str = "wss://stream.data.alpaca.markets";

#[derive(Default)]
pub struct Crypto;

impl ToString for Crypto {
  fn to_string(&self) -> String {
    format!("{}/v1beta3/crypto/us", DATA_WS_URL)
  }
}