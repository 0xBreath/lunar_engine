use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OrderResponse {
  pub symbol: String,
  pub orderId: u64,
  pub orderListId: i64,
  pub clientOrderId: String,
  pub transactTime: u64,
  pub price: String,
  pub origQty: String,
  pub executedQty: String,
  pub cummulativeQuoteQty: String,
  pub status: String,
  pub timeInForce: String,
  #[serde(rename = "type")]
  pub type_: String,
  pub side: String,
  pub workingTime: u64,
  pub selfTradePreventionMode: String,
  pub fills: Vec<Fill>
}

#[derive(Debug, Clone, Deserialize)]
pub struct Fill {
  pub price: String,
  pub qty: String,
  pub commission: String,
  pub commissionAsset: String,
  pub tradeId: u64
}