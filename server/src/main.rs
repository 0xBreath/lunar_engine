#[macro_use]
extern crate lazy_static;

mod builder;
mod alert;
mod client;
mod api;
mod errors;
mod account;
mod response;
mod model;
mod websocket;
mod config;

use alert::*;
use client::Client;
use builder::trade::Trade;
use response::*;
use account::Account;
use config::Config;
use websocket::{WebSockets, WebSocketEvent};

use actix_web::{error, post, web, App, HttpResponse, HttpServer, Responder, Error, Result, get};
use futures::StreamExt;
use regex::Regex;
use std::sync::Mutex;
use log::*;
use simplelog::{
    ColorChoice, Config as SimpleLogConfig,
    TermLogger, TerminalMode,
};
use std::sync::atomic::AtomicBool;

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

lazy_static! {
    static ref ACCOUNT: Mutex<Account> = Mutex::new(Account {
        client: Client::new(
            Some(BINANCE_TEST_API_KEY.to_string()),
            Some(BINANCE_TEST_API_SECRET.to_string()),
            BINANCE_TEST_API.to_string()
        ),
        recv_window: 5000,
        base_asset: "BTC".to_string(),
        quote_asset: "BUSD".to_string(),
        ticker: "BTCBUSD".to_string(),
        active_order: None
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    HttpServer::new(|| {
        App::new()
          .service(post_alert)
          .service(get_assets)
          .service(cancel_orders)
          .service(get_price)
          .service(ticker)
          .route("/", web::get().to(test))
    })
      .bind(bind_address)?
      .run()
      .await
}

fn init_logger() {
    TermLogger::init(
        LevelFilter::Info,
        SimpleLogConfig::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ).expect("Failed to initialize logger");
}

async fn test() -> impl Responder {
    HttpResponse::Ok().body("Server is running...")
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
    let re = Regex::new(r"\{side: (\w+), order: (\w+), timestamp: (\d+)}").unwrap();
    if let Some(captures) = re.captures(&msg) {
        let side = captures.get(1).unwrap().as_str();
        let order = captures.get(2).unwrap().as_str();
        let timestamp = captures.get(3).unwrap().as_str().parse::<i64>().expect("invalid timestamp");
        debug!("Receive latency: {}ms", chrono::Utc::now().timestamp_millis() - timestamp);
        let alert = Alert {
            side: side.parse().expect("invalid side"),
            order: order.parse().expect("invalid order"),
            timestamp
        };
        info!("{:?}", alert);

        let mut account = ACCOUNT.lock().unwrap();

        let pre_trade_time = chrono::Utc::now().timestamp_millis();
        let res = match alert.order {
            Order::Enter => {
                // get account token balances
                let account_info = account.account_info().expect("failed to get account info");
                let busd_balance = &account_info.balances.iter().find(|&x| x.asset == account.quote_asset).unwrap().free;
                let btc_balance = &account_info.balances.iter().find(|&x| x.asset == account.base_asset).unwrap().free;
                info!("BUSD balance: {}", busd_balance);
                // balance is busd_balance parsed to f64
                let balance = busd_balance.parse::<f64>().unwrap();
                // get current price of symbol
                let ticker_price = account.get_price(account.ticker.clone()).expect("failed to get price");
                info!("Current price: {}", ticker_price);
                // calculate quantity of base asset to trade
                let qty = quantity(ticker_price, balance, 25.0);
                info!("Buy BTC quantity: {}", qty);

                match alert.side {
                    Side::Long => {
                        info!("Enter Long");
                        let trade = Trade::new(
                            account.ticker.clone(),
                            alert.side,
                            OrderType::Market,
                            qty,
                            None,
                            None,
                        );
                        let res = account.trade::<OrderResponse>(trade)
                          .expect("failed to enter long");
                        debug!("{:?}", res);
                        let active_order = res.clone();
                        account.set_active_order(Some(active_order));
                        res
                    },
                    Side::Short => {
                        info!("Enter Short");
                        let trade = Trade::new(
                            account.ticker.clone(),
                            alert.side,
                            OrderType::Market,
                            qty,
                            None,
                            None,
                        );
                        let res = account.trade::<OrderResponse>(trade)
                          .expect("failed to enter short");
                        debug!("{:?}", res);
                        let active_order = res.clone();
                        account.set_active_order(Some(active_order));
                        res
                    },
                }
            },
            Order::Exit => {
                // trade balance is to exit account.open_trade.quantity
                match account.get_active_order() {
                    None => {
                        error!("No active order to exit");
                        return Ok(HttpResponse::Ok().body("No active order to exit"));
                    },
                    Some(res) => {
                        let qty = res.executed_qty
                          .parse::<f64>()
                          .expect("failed to parse executed quantity to f64");
                        info!("Exit Quantity: {}", qty);

                        match alert.side {
                            Side::Long => {
                                info!("Exit Long");
                                let trade = Trade::new(
                                    account.ticker.clone(),
                                    Side::Short,
                                    OrderType::Market,
                                    qty,
                                    None,
                                    None,
                                );
                                let res = account.trade::<OrderResponse>(trade)
                                  .expect("failed to exit long");
                                debug!("{:?}", res);
                                account.set_active_order(None);
                                res
                            },
                            Side::Short => {
                                info!("Exit Short");
                                let trade = Trade::new(
                                    account.ticker.clone(),
                                    Side::Long,
                                    OrderType::Market,
                                    qty,
                                    None,
                                    None,
                                );
                                let res = account.trade::<OrderResponse>(trade)
                                  .expect("failed to exit short");
                                debug!("{:?}", res);
                                account.set_active_order(None);
                                res
                            },
                        }
                    }
                }
            },
        };
        debug!("Binance latency: {}ms", chrono::Utc::now().timestamp_millis() - pre_trade_time);
        Ok(HttpResponse::Ok().json(res))
    } else {
        Err(error::ErrorBadRequest("invalid json"))
    }
}

fn quantity(price: f64, balance: f64, pct_equity: f64) -> f64 {
    let quote_qty = ((balance * (pct_equity/100.0)) * 100.0).round() / 100.0;
    ((quote_qty / price) * 1000000.0).round() / 1000000.0
}

#[get("/assets")]
async fn get_assets() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().unwrap();

    let res = account.account_info().expect("failed to get account info");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/cancel")]
async fn cancel_orders() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().unwrap();
    let res = account.cancel_all_active_orders().expect("failed to cancel orders");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/price")]
async fn get_price() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().unwrap();
    let res = account.get_price(account.ticker.clone()).expect("failed to get price");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/ticker")]
async fn ticker() -> Result<HttpResponse, Error> {
    let config = Config::testnet();
    let keep_running = AtomicBool::new(true);
    let mut ws = WebSockets::new(|event: WebSocketEvent| {
        if let WebSocketEvent::Kline(kline_event) = event {
            println!(
                "Symbol: {}, high: {}, low: {}",
                kline_event.kline.symbol, kline_event.kline.low, kline_event.kline.high
            );
        }
        Ok(())
    });
    let sub = String::from("btcbusd@kline_1m");
    ws.connect_with_config(&sub, &config).expect("failed to connect to binance");
    if let Err(e) = ws.event_loop(&keep_running) {
        println!("Error: {}", e);
    }
    ws.disconnect().unwrap();
    println!("disconnected");

    Ok(HttpResponse::Ok().body("Dummy response..."))
}