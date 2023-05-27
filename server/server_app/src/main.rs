#[macro_use]
extern crate lazy_static;

use actix_web::{get, web, App, Error, HttpResponse, HttpServer, Responder, Result};
use log::*;
use server_lib::*;
use simplelog::{ColorChoice, Config as SimpleLogConfig, TermLogger, TerminalMode};
use std::sync::Mutex;

// Binance US API endpoint
// Data returned in ascending order, oldest first
// Timestamps are in milliseconds
#[allow(dead_code)]
const BINANCE_API: &str = "https://api.binance.us";

// Binance Spot Test Network API credentials
const BINANCE_TEST_API: &str = "https://testnet.binance.vision";
const BINANCE_TEST_API_KEY: &str =
    "hrCcYjjRCW6jCCOVGiOOXve1UVLK8jbYd08WyKQjuUI63VNmcuR0EDBtDsrW9KBJ";
const BINANCE_TEST_API_SECRET: &str =
    "XGKu8AelLejzC6R5ZBWvbNzy4NC7d78ckU0sOJk3VeFRsWnJTajCfcFsArnPFEjP";

lazy_static! {
    static ref ACCOUNT: Mutex<Account> = Mutex::new(Account {
        client: Client::new(
            Some(BINANCE_TEST_API_KEY.to_string()),
            Some(BINANCE_TEST_API_SECRET.to_string()),
            BINANCE_TEST_API.to_string()
        ),
        recv_window: 5000,
        base_asset: "BTC".to_string(),
        quote_asset: "USDT".to_string(),
        ticker: "BTCUSDT".to_string(),
        active_order: None
    });
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    init_logger();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    info!("Starting Server...");
    HttpServer::new(|| {
        App::new()
            .service(get_assets)
            .service(cancel_orders)
            .service(get_price)
            .service(exchange_info)
            .service(all_orders)
            .service(last_order)
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
    )
    .expect("Failed to initialize logger");
}

async fn test() -> impl Responder {
    HttpResponse::Ok().body("Server is running...")
}

#[get("/assets")]
async fn get_assets() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().unwrap();
    let res = account.account_info().expect("failed to get account info");
    Ok(HttpResponse::Ok().json(res))
}

#[get("/cancel")]
async fn cancel_orders() -> Result<HttpResponse, Error> {
    info!("Cancel all active orders");
    let account = ACCOUNT.lock().unwrap();
    let res = account
        .cancel_all_active_orders()
        .expect("failed to cancel orders");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/price")]
async fn get_price() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().unwrap();
    let res = account
        .get_price(account.ticker.clone())
        .expect("failed to get price");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/allOrders")]
async fn all_orders() -> Result<HttpResponse, Error> {
    info!("Fetching all historical orders...");
    let account = ACCOUNT.lock().unwrap();
    let res = account
        .all_orders(account.ticker.clone())
        .expect("failed to get historical orders");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/lastOrder")]
async fn last_order() -> Result<HttpResponse, Error> {
    info!("Fetching last open order...");
    let account = ACCOUNT.lock().unwrap();
    let res = account
        .last_order(account.ticker.clone())
        .expect("failed to get last order");
    info!("Last open order: {:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/info")]
async fn exchange_info() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().unwrap();
    let info = account.exchange_info(account.ticker.clone()).unwrap();
    Ok(HttpResponse::Ok().json(info))
}
