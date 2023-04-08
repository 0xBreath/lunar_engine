use serde::de::DeserializeOwned;
use crate::api::*;
use crate::client::Client;
use crate::response::{AccountInfoResponse, OrderCanceled, OrderResponse, PriceResponse};
use crate::errors::Result;
use crate::builder::*;
// use crate::model::Transaction;

#[derive(Clone)]
pub struct Account {
  pub client: Client,
  pub recv_window: u64,
  pub base_asset: String,
  pub quote_asset: String,
  pub ticker: String,
  pub active_order: Option<OrderResponse>
}

impl Account {
  pub fn new(client: Client, recv_window: u64, base_asset: String, quote_asset: String, ticker: String) -> Self {
    Self {
      client,
      recv_window,
      base_asset,
      quote_asset,
      ticker,
      active_order: None
    }
  }

  /// Place a trade
  pub fn trade<T: DeserializeOwned>(&self, trade: Trade) -> Result<T> {
    let req = trade.request();
    self.client.post_signed::<T>(API::Spot(Spot::Order), req)
  }

  /// Get account info which includes token balances
  pub fn account_info(&self) -> Result<AccountInfoResponse> {
    let req= AccountInfo::request();
    self.client.get_signed::<AccountInfoResponse>(API::Spot(Spot::Account), Some(req))
  }

  /// Get price of a single symbol
  pub fn get_price(&self, symbol: String) -> Result<f64> {
    let req = Price::request(symbol);
    let res = self.client.get::<PriceResponse>(API::Spot(Spot::Price), Some(req))?;
    let price = res.price.parse::<f64>().expect("failed to parse price");
    Ok(price)
  }

  #[allow(dead_code)]
  /// Cancel all open orders for a single symbol
  pub fn cancel_all_active_orders(&self, symbol: String) -> Result<Vec<OrderCanceled>> {
    let req = CancelOrders::request(symbol);
    self.client
      .delete_signed(API::Spot(Spot::OpenOrders), Some(req))
  }
  
  #[allow(dead_code)]
  /// Set current active trade to track quantity to exit trade
  pub fn set_active_order(&mut self, trade: Option<OrderResponse>) {
    self.active_order = trade;
  }
  
  #[allow(dead_code)]
  /// Get current active trade
  pub fn get_active_order(&self) -> Option<OrderResponse> {
    self.active_order.clone()
  }
}