#[macro_use]
extern crate lazy_static;

use actix_web::{get, web, App, Error, HttpResponse, HttpServer, Responder, Result};
use binance_lib::*;
use dotenv::dotenv;
use log::*;
use simplelog::{ColorChoice, Config as SimpleLogConfig, TermLogger, TerminalMode};
use tokio::sync::Mutex;

// Binance Spot Test Network API credentials
#[allow(dead_code)]
const BINANCE_TEST_API: &str = "https://testnet.binance.vision";
// Binance Spot Live Network API credentials
#[allow(dead_code)]
const BINANCE_LIVE_API: &str = "https://api.binance.us";
const BASE_ASSET: &str = "BTC";
const QUOTE_ASSET: &str = "USDT";
const TICKER: &str = "BTCUSDT";

lazy_static! {
    static ref ACCOUNT: Mutex<Account> = match std::env::var("TESTNET")
        .expect(
            "ACCOUNT init failed. TESTNET environment variable must be set to either true or false"
        )
        .parse::<bool>()
        .expect("Failed to parse env TESTNET to boolean")
    {
        true => {
            Mutex::new(Account {
                client: Client::new(
                    Some(
                        std::env::var("BINANCE_TEST_API_KEY")
                            .expect("Failed to parse BINANCE_TEST_API_KEY from env"),
                    ),
                    Some(
                        std::env::var("BINANCE_TEST_API_SECRET")
                            .expect("Failed to parse BINANCE_TEST_API_SECRET from env"),
                    ),
                    BINANCE_TEST_API.to_string(),
                ),
                recv_window: 5000,
                base_asset: BASE_ASSET.to_string(),
                quote_asset: QUOTE_ASSET.to_string(),
                ticker: TICKER.to_string(),
            })
        }
        false => {
            Mutex::new(Account {
                client: Client::new(
                    Some(
                        std::env::var("BINANCE_LIVE_API_KEY")
                            .expect("Failed to parse BINANCE_LIVE_API_KEY from env"),
                    ),
                    Some(
                        std::env::var("BINANCE_LIVE_API_SECRET")
                            .expect("Failed to parse BINANCE_LIVE_API_SECRET from env"),
                    ),
                    BINANCE_LIVE_API.to_string(),
                ),
                recv_window: 5000,
                base_asset: BASE_ASSET.to_string(),
                quote_asset: QUOTE_ASSET.to_string(),
                ticker: TICKER.to_string(),
            })
        }
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    init_logger();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let bind_address = format!("0.0.0.0:{}", port);

    info!("Starting Server...");
    HttpServer::new(|| {
        App::new()
            .service(get_account_info)
            .service(get_assets)
            .service(cancel_orders)
            .service(get_price)
            .service(exchange_info)
            .service(all_orders)
            .service(open_orders)
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

#[get("/account")]
async fn get_account_info() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let res = account.account_info().expect("failed to get account info");
    Ok(HttpResponse::Ok().json(res))
}

#[get("/assets")]
async fn get_assets() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let res = account.all_assets().expect("failed to get assets");
    trace!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/cancel")]
async fn cancel_orders() -> Result<HttpResponse, Error> {
    info!("Cancel all active orders");
    let account = ACCOUNT.lock().await;
    let res = account
        .cancel_all_open_orders()
        .expect("failed to cancel orders");
    let ids = res
        .iter()
        .map(|order| order.orig_client_order_id.clone().unwrap())
        .collect::<Vec<String>>();
    info!("All active orders canceled {:?}", ids);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/price")]
async fn get_price() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let res = account.price().expect("failed to get price");
    trace!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/allOrders")]
async fn all_orders() -> Result<HttpResponse, Error> {
    info!("Fetching all historical orders...");
    let account = ACCOUNT.lock().await;
    let res = account
        .all_orders(account.ticker.clone())
        .expect("failed to get all orders");
    let last = res.last().unwrap();
    info!(
        "Last order ID: {:?}, Status: {}",
        last.client_order_id, last.status
    );
    Ok(HttpResponse::Ok().json(res))
}

#[get("/openOrders")]
async fn open_orders() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let res = account
        .open_orders(account.ticker.clone())
        .expect("failed to get open orders");
    info!("Open orders: {:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/info")]
async fn exchange_info() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let info = account
        .exchange_info(account.ticker.clone())
        .expect("Failed to get exchange info");
    Ok(HttpResponse::Ok().json(info))
}
