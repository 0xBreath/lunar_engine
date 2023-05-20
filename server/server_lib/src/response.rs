use crate::alert::Side;
use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitOrderResponse {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64,
    pub client_order_id: String,
    pub transact_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64,
    pub client_order_id: String,
    pub transact_time: u64,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    pub cummulative_quote_qty: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub type_: String,
    // BUY or SELL
    pub side: String,
    pub working_time: u64,
    pub self_trade_prevention_mode: String,
    pub fills: Vec<Fill>,
}

impl OrderResponse {
    #[allow(dead_code)]
    pub fn side(&self) -> Side {
        match self.side.as_str() {
            "BUY" => Side::Long,
            "SELL" => Side::Short,
            _ => panic!("Invalid side {}", self.side),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub client_order_id: String,
    #[serde(with = "string_or_float")]
    pub cum_qty: f64,
    #[serde(with = "string_or_float")]
    pub cum_quote: f64,
    #[serde(with = "string_or_float")]
    pub executed_qty: f64,
    pub order_id: u64,
    #[serde(with = "string_or_float")]
    pub avg_price: f64,
    #[serde(with = "string_or_float")]
    pub orig_qty: f64,
    pub reduce_only: bool,
    pub side: String,
    pub position_side: String,
    pub status: String,
    #[serde(with = "string_or_float")]
    pub stop_price: f64,
    pub close_position: bool,
    pub symbol: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub orig_type: String,
    #[serde(default)]
    #[serde(with = "string_or_float_opt")]
    pub activate_price: Option<f64>,
    #[serde(default)]
    #[serde(with = "string_or_float_opt")]
    pub price_rate: Option<f64>,
    pub update_time: u64,
    pub working_type: String,
    price_protect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fill {
    pub price: String,
    pub qty: String,
    pub commission: String,
    pub commission_asset: String,
    pub trade_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfoResponse {
    pub maker_commission: u32,
    pub taker_commission: u32,
    pub buyer_commission: u32,
    pub seller_commission: u32,
    pub commission_rates: CommissionRates,
    pub can_trade: bool,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    pub brokered: bool,
    pub require_self_trade_prevention: bool,
    pub update_time: u64,
    pub account_type: String,
    pub balances: Vec<Balance>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionRates {
    pub maker: String,
    pub taker: String,
    pub buyer: String,
    pub seller: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceResponse {
    pub symbol: String,
    pub price: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderCanceled {
    pub symbol: String,
    pub orig_client_order_id: Option<String>,
    pub order_id: Option<u64>,
    pub client_order_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalOrder {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64, //Unless OCO, the value will always be -1
    pub client_order_id: String,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    pub cummulative_quote_qty: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub _type: String,
    pub side: String,
    pub stop_price: Option<String>,
    pub iceberg_qty: Option<String>,
    pub time: i64,
    pub update_time: i64,
    pub is_working: bool,
    pub orig_quote_order_qty: String,
    pub working_time: i64,
    pub self_trade_prevention_mode: String,
}
