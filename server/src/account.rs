use crate::api::{API, Spot};
use crate::client::Client;
use crate::trade::Trade;
use crate::response::OrderResponse;
use crate::errors::Result;

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

  pub fn test_market_buy(&self, trade: Trade) -> Result<OrderResponse> {
    let req = trade.build_request();
    self.client.post_signed::<OrderResponse>(API::Spot(Spot::Order), req)
  }

  pub fn test_market_sell(&self, trade: Trade) -> Result<OrderResponse> {
    let req = trade.build_request();
    self.client.post_signed::<OrderResponse>(API::Spot(Spot::Order), req)
  }
}