use crate::errors::Result;
use crate::model::{
  AccountUpdateEvent, BalanceUpdateEvent,
  KlineEvent, OrderBook, OrderTradeEvent, TradeEvent,
};
use error_chain::bail;
use url::Url;
use serde::{Deserialize, Serialize};
use tungstenite::{connect, Message};
use tungstenite::protocol::WebSocket;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::handshake::client::Response;

use std::sync::atomic::{AtomicBool, Ordering};
use std::net::TcpStream;
use crate::config::Config;

#[allow(clippy::all)]
enum WebSocketAPI {
  Default,
  MultiStream,
  Custom(String),
}

impl WebSocketAPI {
  fn params(self, subscription: &str) -> String {
    match self {
      WebSocketAPI::Default => format!("wss://stream.binance.com:9443/ws/{}", subscription),
      WebSocketAPI::MultiStream => format!(
        "wss://stream.binance.com:9443/stream?streams={}",
        subscription
      ),
      WebSocketAPI::Custom(url) => format!("{}/{}", url, subscription),
    }
  }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WebSocketEvent {
  AccountUpdate(AccountUpdateEvent),
  BalanceUpdate(BalanceUpdateEvent),
  OrderTrade(OrderTradeEvent),
  Trade(TradeEvent),
  OrderBook(OrderBook),
  Kline(KlineEvent),
}

pub struct WebSockets<'a> {
  pub socket: Option<(WebSocket<MaybeTlsStream<TcpStream>>, Response)>,
  handler: Box<dyn FnMut(WebSocketEvent) -> Result<()> + 'a>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Events {
  BalanceUpdateEvent(BalanceUpdateEvent),
  AccountUpdateEvent(AccountUpdateEvent),
  OrderTradeEvent(OrderTradeEvent),
  TradeEvent(TradeEvent),
  KlineEvent(KlineEvent),
  OrderBook(OrderBook),
}

impl<'a> WebSockets<'a> {
  pub fn new<Callback>(handler: Callback) -> WebSockets<'a>
    where
      Callback: FnMut(WebSocketEvent) -> Result<()> + 'a,
  {
    WebSockets {
      socket: None,
      handler: Box::new(handler),
    }
  }

  #[allow(dead_code)]
  pub fn connect(&mut self, subscription: &str) -> Result<()> {
    self.connect_wss(&WebSocketAPI::Default.params(subscription))
  }

  pub fn connect_with_config(&mut self, subscription: &str, config: &Config) -> Result<()> {
    self.connect_wss(&WebSocketAPI::Custom(config.ws_endpoint.clone()).params(subscription))
  }

  #[allow(dead_code)]
  pub fn connect_multiple_streams(&mut self, endpoints: &[String]) -> Result<()> {
    self.connect_wss(&WebSocketAPI::MultiStream.params(&endpoints.join("/")))
  }

  fn connect_wss(&mut self, wss: &str) -> Result<()> {
    let url = Url::parse(wss)?;
    match connect(url) {
      Ok(answer) => {
        self.socket = Some(answer);
        Ok(())
      }
      Err(e) => bail!(format!("Error during handshake {}", e)),
    }
  }

  pub fn disconnect(&mut self) -> Result<()> {
    if let Some(ref mut socket) = self.socket {
      socket.0.close(None)?;
      return Ok(());
    }
    bail!("Not able to close the connection");
  }

  #[allow(dead_code)]
  pub fn test_handle_msg(&mut self, msg: &str) -> Result<()> {
    self.handle_msg(msg)
  }

  fn handle_msg(&mut self, msg: &str) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(msg)?;

    if let Some(data) = value.get("data") {
      self.handle_msg(&data.to_string())?;
      return Ok(());
    }

    if let Ok(events) = serde_json::from_value::<Events>(value) {
      let action = match events {
        Events::BalanceUpdateEvent(v) => WebSocketEvent::BalanceUpdate(v),
        Events::AccountUpdateEvent(v) => WebSocketEvent::AccountUpdate(v),
        Events::OrderTradeEvent(v) => WebSocketEvent::OrderTrade(v),
        Events::TradeEvent(v) => WebSocketEvent::Trade(v),
        Events::KlineEvent(v) => WebSocketEvent::Kline(v),
        Events::OrderBook(v) => WebSocketEvent::OrderBook(v),
       };
      (self.handler)(action)?;
    }
    Ok(())
  }

  pub fn event_loop(&mut self, running: &AtomicBool) -> Result<()> {
    while running.load(Ordering::Relaxed) {
      if let Some(ref mut socket) = self.socket {
        let message = socket.0.read_message()?;
        match message {
          Message::Text(msg) => {
            if let Err(e) = self.handle_msg(&msg) {
              bail!(format!("Error on handling stream message: {}", e));
            }
          }
          Message::Ping(_) => {
            socket.0.write_message(Message::Pong(vec![])).unwrap();
          }
          Message::Pong(_) | Message::Binary(_) | Message::Frame(_) => (),
          Message::Close(e) => bail!(format!("Disconnected {:?}", e)),
        }
      }
    }
    Ok(())
  }
}