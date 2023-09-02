use crate::config::Config;
use crate::errors::Result;
use crate::model::{
    AccountUpdateEvent, BalanceUpdateEvent, KlineEvent, OrderTradeEvent, TradeEvent,
};
use crate::BinanceError;
use log::*;
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use tungstenite::handshake::client::Response;
use tungstenite::protocol::WebSocket;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message};
use url::Url;

#[allow(clippy::all)]
enum WebSocketAPI {
    Default,
    MultiStream,
    Custom(String),
}

impl WebSocketAPI {
    fn params(self, subscription: &str, testnet: bool) -> String {
        match testnet {
            true => match self {
                WebSocketAPI::Default => {
                    format!("wss://testnet.binance.vision/ws/{}", subscription)
                }
                WebSocketAPI::MultiStream => format!(
                    "wss://testnet.binance.vision/stream?streams={}",
                    subscription
                ),
                WebSocketAPI::Custom(url) => format!("{}/{}", url, subscription),
            },
            false => match self {
                WebSocketAPI::Default => {
                    format!("wss://stream.binance.us:9443/ws/{}", subscription)
                }
                WebSocketAPI::MultiStream => format!(
                    "wss://stream.binance.us:9443/stream?streams={}",
                    subscription
                ),
                WebSocketAPI::Custom(url) => format!("{}/{}", url, subscription),
            },
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
    Kline(KlineEvent),
}

pub struct WebSockets<'a> {
    pub socket: Option<(WebSocket<MaybeTlsStream<TcpStream>>, Response)>,
    handler: Box<dyn FnMut(WebSocketEvent) -> Result<()> + 'a>,
    testnet: bool,
}

impl<'a> Drop for WebSockets<'a> {
    fn drop(&mut self) {
        if let Some(ref mut socket) = self.socket {
            socket.0.close(None).unwrap();
        }
        self.disconnect().unwrap();
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Events {
    BalanceUpdateEvent(BalanceUpdateEvent),
    AccountUpdateEvent(AccountUpdateEvent),
    OrderTradeEvent(OrderTradeEvent),
    TradeEvent(TradeEvent),
    KlineEvent(KlineEvent),
}

impl<'a> WebSockets<'a> {
    pub fn new<Callback>(testnet: bool, handler: Callback) -> WebSockets<'a>
    where
        Callback: FnMut(WebSocketEvent) -> Result<()> + 'a,
    {
        WebSockets {
            socket: None,
            handler: Box::new(handler),
            testnet,
        }
    }

    #[allow(dead_code)]
    pub fn connect(&mut self, subscription: &str) -> Result<()> {
        self.connect_wss(&WebSocketAPI::Default.params(subscription, self.testnet))
    }

    pub fn connect_with_config(&mut self, subscription: &str, config: &Config) -> Result<()> {
        self.connect_wss(
            &WebSocketAPI::Custom(config.ws_endpoint.clone()).params(subscription, self.testnet),
        )
    }

    pub fn connect_multiple_streams(&mut self, endpoints: &[String], testnet: bool) -> Result<()> {
        self.connect_wss(&WebSocketAPI::MultiStream.params(&endpoints.join("/"), testnet))
    }

    fn connect_wss(&mut self, wss: &str) -> Result<()> {
        let url = Url::parse(wss)?;
        match connect(url) {
            Ok(answer) => {
                self.socket = Some(answer);
                Ok(())
            }
            Err(e) => Err(BinanceError::Tungstenite(e)),
        }
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(ref mut socket) = self.socket {
            socket.0.close(None)?;
            return Ok(());
        }
        Err(BinanceError::WebSocketDisconnected)
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
                    Message::Text(msg) => match self.handle_msg(&msg) {
                        Ok(_) => {}
                        Err(e) => {
                            if let BinanceError::WebSocketDisconnected = e {
                                return Err(e);
                            }
                        }
                    },
                    Message::Ping(_) => {
                        debug!("Received websocket ping");
                        match socket.0.write_message(Message::Pong(vec![])) {
                            Ok(_) => {}
                            Err(e) => return Err(BinanceError::Tungstenite(e)),
                        }
                    }
                    Message::Pong(_) | Message::Binary(_) | Message::Frame(_) => return Ok(()),
                    Message::Close(e) => {
                        return match e {
                            Some(e) => Err(BinanceError::Custom(e.to_string())),
                            None => Err(BinanceError::WebSocketDisconnected),
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
