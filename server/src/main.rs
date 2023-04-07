mod builder;
mod alert;
mod client;
mod api;
mod errors;
mod account;
mod response;
mod model;

use alert::*;
use client::Client;
use builder::trade::Trade;

use actix_web::{error, post, web, App, HttpResponse, HttpServer, Responder, Error, Result, get};
use futures::StreamExt;
use regex::Regex;

// Message buffer max size is 256k bytes
const MAX_SIZE: usize = 262_144;

// Binance US API endpoint
// Data returned in ascending order, oldest first
// Timestamps are in milliseconds
#[allow(dead_code)]
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
          .service(get_assets)
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
    let re = Regex::new(r"\{side: (\w+), order: (\w+), price: (\d+\.\d+), stop_loss: (\d+\.\d+), timestamp: (\d+)}").unwrap();
    if let Some(captures) = re.captures(&msg) {
        let side = captures.get(1).unwrap().as_str();
        let order = captures.get(2).unwrap().as_str();
        let entry_price = captures.get(3).unwrap().as_str().parse::<f64>().expect("invalid price");
        let stop_loss = captures.get(4).unwrap().as_str().parse::<f64>().expect("invalid stop loss");
        let timestamp = captures.get(5).unwrap().as_str().parse::<i64>().expect("invalid timestamp");
        println!("Tradingview latency: {}ms", chrono::Utc::now().timestamp_millis() - timestamp);
        let alert = Alert {
            side: side.parse().expect("invalid side"),
            order: order.parse().expect("invalid order"),
            price: entry_price,
            stop_loss,
            timestamp
        };
        println!("{:?}", alert);

        // init Binance client
        let client = Client::new(
            Some(BINANCE_TEST_API_KEY.to_string()),
            Some(BINANCE_TEST_API_SECRET.to_string()),
            BINANCE_TEST_API.to_string()
        );
        let account = account::Account::new(client, 5000);
        let symbol = "BTCBUSD".to_string();
        let quote_asset_symbol = "BUSD".to_string();

        // let cancel = account.cancel_all_open_orders(symbol.clone()).expect("failed to cancel orders");

        // get account token balances
        let account_info = account.account_info().expect("failed to get account info");
        let busd_balance = &account_info.balances.iter().find(|&x| x.asset == quote_asset_symbol).unwrap().free;
        println!("BUSD balance: {}", busd_balance);
        // balance is busd_balance parsed to f64
        let balance = busd_balance.parse::<f64>().unwrap();

        // get current price of symbol
        let ticker_price = account.get_price(symbol.clone()).expect("failed to get price");
        println!("Current price: {}", ticker_price);

        // calculate quantity of base asset to trade
        let qty = quantity(ticker_price, balance, 25.0);
        println!("Quantity: {}", qty);

        let pre_trade_time = chrono::Utc::now().timestamp_millis();
        let res = match alert.order {
            Order::Enter => {
                match alert.side {
                    Side::Long => {
                        println!("Enter Long");
                        let trade = Trade::new(
                            symbol,
                            alert.side,
                            OrderType::StopLossLimit,
                            qty,
                            Some(alert.price),
                            Some(alert.stop_loss),
                        );
                        let res = account.trade(trade);
                        println!("{:?}", res);
                        res
                    },
                    Side::Short => {
                        println!("Enter Short");
                        let trade = Trade::new(
                            symbol,
                            alert.side,
                            OrderType::StopLossLimit,
                            qty,
                            Some(alert.price),
                            Some(alert.stop_loss),
                        );
                        let res = account.trade(trade);
                        println!("{:?}", res);
                        res
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
                            qty,
                            None,
                            None,
                        );
                        let res = account.trade(trade);
                        println!("{:?}", res);
                        res
                    },
                    Side::Short => {
                        println!("Exit Short");
                        let trade = Trade::new(
                            symbol,
                            Side::Long,
                            OrderType::Market,
                            qty,
                            None,
                            None,
                        );
                        let res = account.trade(trade);
                        println!("{:?}", res);
                        res
                    },
                }
            },
        };
        println!("Binance latency: {}ms", chrono::Utc::now().timestamp_millis() - pre_trade_time);

        Ok(HttpResponse::Ok().json(res.expect("failed to trade")))
    } else {
        Err(error::ErrorBadRequest("invalid json"))
    }
}

fn quantity(price: f64, balance: f64, pct_equity: f64) -> f64 {
    let busd_qty = ((balance * (pct_equity/100.0)) * 100.0).round() / 100.0;
    ((busd_qty / price) * 1000000.0).round() / 1000000.0
}

#[get("/assets")]
async fn get_assets() -> Result<HttpResponse, Error> {
    println!("get_assets");
    let client = Client::new(
        Some(BINANCE_TEST_API_KEY.to_string()),
        Some(BINANCE_TEST_API_SECRET.to_string()),
        BINANCE_TEST_API.to_string()
    );
    let account = account::Account::new(client, 5000);
    let res = account.account_info().expect("failed to get account info");
    println!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

async fn test() -> impl Responder {
    HttpResponse::Ok().body("Server is running...")
}