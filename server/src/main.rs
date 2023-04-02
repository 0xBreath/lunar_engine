mod trade;
mod alert;
mod client;
mod api;
mod errors;
mod account;
mod response;

use alert::*;
use client::Client;
use trade::Trade;

use actix_web::{error, post, web, App, HttpResponse, HttpServer, Responder, Error, Result};
use futures::StreamExt;
use regex::Regex;

// Message buffer max size is 256k bytes
const MAX_SIZE: usize = 262_144;

// Binance US API endpoint
// Data returned in ascending order, oldest first
// Timestamps are in milliseconds
const BINANCE_API: &str = "https://api.binance.us";

// Binance Spot Test Network API credentials
const BINANCE_TEST_API: &str = "https://testnet.binance.vision";
const BINANCE_TEST_API_KEY: &str = "hrCcYjjRCW6jCCOVGiOOXve1UVLK8jbYd08WyKQjuUI63VNmcuR0EDBtDsrW9KBJ";
const BINANCE_TEST_API_SECRET: &str = "XGKu8AelLejzC6R5ZBWvbNzy4NC7d78ckU0sOJk3VeFRsWnJTajCfcFsArnPFEjP";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    HttpServer::new(|| {
        App::new()
          .service(post_alert)
          .route("/", web::get().to(test))
    })
      .bind(bind_address)?
      .run()
      .await
}

#[post("/alert")]
async fn post_alert(mut payload: web::Payload) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }
    let msg = String::from_utf8(body.to_vec()).unwrap();
    let re = Regex::new(r"\{side: (\w+), order: (\w+), timestamp: (\d+)\}").unwrap();
    if let Some(captures) = re.captures(&msg) {
        let side = captures.get(1).unwrap().as_str();
        let order = captures.get(2).unwrap().as_str();
        let timestamp = captures.get(3).unwrap().as_str().parse::<i64>().expect("invalid timestamp");
        println!("From Tradingview latency: {}ms", chrono::Utc::now().timestamp_millis() - timestamp);
        let alert = Alert {
            side: side.parse().expect("invalid side"),
            order: order.parse().expect("invalid order"),
            timestamp,
        };
        println!("{:?}", alert);

        let client = Client::new(
            Some(BINANCE_TEST_API_KEY.to_string()),
            Some(BINANCE_TEST_API_SECRET.to_string()),
            BINANCE_TEST_API.to_string()
        );
        let account = account::Account::new(client, 5000);
        let symbol = "BTCBUSD".to_string();
        let qty = 1000.0;

        match alert.order {
            Order::Enter => {
                match alert.side {
                    Side::Long => {
                        println!("Enter Long");
                        let trade = Trade::new(
                            symbol,
                            alert.side.clone(),
                            OrderType::Market,
                            qty
                        );
                        let res = account.test_market_buy(trade);
                        println!("{:?}", res);
                    },
                    Side::Short => {
                        println!("Enter Short");
                        let trade = Trade::new(
                            symbol,
                            alert.side.clone(),
                            OrderType::Market,
                            qty
                        );
                        let res = account.test_market_sell(trade);
                        println!("{:?}", res);
                    },
                }
            },
            Order::Exit => {
                match alert.side {
                    Side::Long => {
                        println!("Exit Long");
                        let trade = Trade::new(
                            symbol,
                            Side::Short,
                            OrderType::Market,
                            qty
                        );
                        let res = account.test_market_sell(trade);
                        println!("{:?}", res);
                    },
                    Side::Short => {
                        println!("Exit Short");
                        let trade = Trade::new(
                            symbol,
                            Side::Long,
                            OrderType::Market,
                            qty
                        );
                        let res = account.test_market_buy(trade);
                        println!("{:?}", res);
                    },
                }
            },
        }

        Ok(HttpResponse::Ok().json(alert))
    } else {
        Err(error::ErrorBadRequest("invalid json"))
    }
}

async fn test() -> impl Responder {
    HttpResponse::Ok().body("Server is running...")
}
