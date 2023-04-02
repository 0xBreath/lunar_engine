use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::alert::*;
use std::io::Result;

pub struct Trade {
  /// Ticker symbol (e.g. BTCUSDC
  pub symbol: String,
  /// Side of the trade (BUY or SELL)
  pub side: Side,
  /// Type of order (LIMIT, MARKET, STOP_LOSS_LIMIT, TAKE_PROFIT_LIMIT, LIMIT_MAKER)
  pub order_type: OrderType,
  /// Quantity in quote asset of the symbol to trade (e.g. BTCUSDC with quantity 10000 would trade 10000 USDC)
  pub quantity: f64,
  /// UNIX timestamp in milliseconds of trade placement
  pub timestamp: i64,
}

impl Trade {
  pub fn new(symbol: String, side: Side, order_type: OrderType, quantity: f64) -> Self {
    Self {
      symbol,
      side,
      order_type,
      quantity,
      timestamp: 0,
    }
  }

  pub fn get_timestamp(&self) -> Result<u64> {
    let system_time = SystemTime::now();
    let since_epoch = system_time.duration_since(UNIX_EPOCH)
      .expect("System time is before UNIX EPOCH");
    Ok(since_epoch.as_secs() * 1000 + u64::from(since_epoch.subsec_nanos()) / 1_000_000)
  }

  fn build_order(&self) -> BTreeMap<String, String> {
    let mut btree = BTreeMap::<String, String>::new();
    btree.insert("symbol".to_string(), self.symbol.clone());
    btree.insert("side".to_string(), self.side.fmt_binance().to_string());
    btree.insert("type".to_string(), self.order_type.fmt_binance().to_string());
    btree.insert("quoteOrderQty".to_string(), self.quantity.to_string());
    let timestamp = self.get_timestamp().expect("Failed to get timestamp");
    btree.insert("timestamp".to_string(), timestamp.to_string());
    btree
  }

  pub fn build_request(&self) -> String {
    let btree = self.build_order();
    let mut request = String::new();
    for (key, value) in btree.iter() {
      request.push_str(&format!("{}={}&", key, value));
    }
    request.pop();
    request
  }
}






















