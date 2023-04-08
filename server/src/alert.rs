use std::str::FromStr;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Side {
  Long,
  Short
}
impl FromStr for Side {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "Long" => Ok(Side::Long),
      "Short" => Ok(Side::Short),
      _ => Err(()),
    }
  }
}
impl Side {
  pub fn fmt_binance(&self) -> &str {
    match self {
      Side::Long => "BUY",
      Side::Short => "SELL",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Order {
  Enter,
  Exit
}
impl FromStr for Order {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "Enter" => Ok(Order::Enter),
      "Exit" => Ok(Order::Exit),
      _ => Err(()),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
  pub side: Side,
  pub order: Order,
  pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum OrderType {
  Limit,
  Market,
  StopLossLimit,
  StopLoss,
  TakeProfitLimit
}
impl OrderType {
  pub fn fmt_binance(&self) -> &str {
    match self {
      OrderType::Limit => "LIMIT",
      OrderType::Market => "MARKET",
      OrderType::StopLossLimit => "STOP_LOSS_LIMIT",
      OrderType::StopLoss => "STOP_LOSS",
      OrderType::TakeProfitLimit => "TAKE_PROFIT_LIMIT",
    }
  }
}