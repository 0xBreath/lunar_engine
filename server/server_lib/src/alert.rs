use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Side {
    Long,
    Short,
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
pub enum AlertOrder {
    Enter,
    Exit,
}
impl FromStr for AlertOrder {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Enter" => Ok(AlertOrder::Enter),
            "Exit" => Ok(AlertOrder::Exit),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub side: Side,
    pub order: AlertOrder,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum OrderType {
    Limit,
    Market,
    StopLossLimit,
    StopLoss,
    TakeProfitLimit,
    TakeProfit,
}
impl OrderType {
    pub fn fmt_binance(&self) -> &str {
        match self {
            OrderType::Limit => "LIMIT",
            OrderType::Market => "MARKET",
            OrderType::StopLossLimit => "STOP_LOSS_LIMIT",
            OrderType::StopLoss => "STOP_LOSS",
            OrderType::TakeProfitLimit => "TAKE_PROFIT_LIMIT",
            OrderType::TakeProfit => "TAKE_PROFIT",
        }
    }
}
