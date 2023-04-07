use std::collections::BTreeMap;
use crate::api::*;
use crate::client::Client;
use crate::response::{AccountInfoResponse, OrderCanceled, OrderResponse, PriceResponse};
use crate::errors::Result;
use crate::builder::*;
use crate::model::Transaction;

#[derive(Clone)]
pub struct Account {
  pub client: Client,
  pub recv_window: u64,
}

impl Account {
  pub fn new(client: Client, recv_window: u64) -> Self {
    Self {
      client,
      recv_window,
    }
  }

  pub fn trade(&self, trade: Trade) -> Result<Transaction> {
    let req = trade.request();
    self.client.post_signed::<Transaction>(API::Spot(Spot::Order), req)
  }

  pub fn account_info(&self) -> Result<AccountInfoResponse> {
    let req= AccountInfo::request();
    self.client.get_signed::<AccountInfoResponse>(API::Spot(Spot::Account), Some(req))
  }

  pub fn get_price(&self, symbol: String) -> Result<f64> {
    let req = Price::request(symbol);
    let res = self.client.get::<PriceResponse>(API::Spot(Spot::Price), Some(req))?;
    let price = res.price.parse::<f64>().expect("failed to parse price");
    Ok(price)
  }

  // Cancel all open orders for a single symbol
  pub fn cancel_all_open_orders(&self, symbol: String) -> Result<Vec<OrderCanceled>> {
    let req = CancelOrders::request(symbol);
    self.client
      .delete_signed(API::Spot(Spot::OpenOrders), Some(req))
  }
}