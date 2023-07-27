#[macro_use]
extern crate lazy_static;

use actix_web::{get, web, App, Error, HttpResponse, HttpServer, Responder, Result};
use binance_lib::*;
use log::*;
use simplelog::{ColorChoice, Config as SimpleLogConfig, TermLogger, TerminalMode};
use tokio::sync::Mutex;

// Binance Spot Test Network API credentials
const BINANCE_TEST_API: &str = "https://testnet.binance.vision";
const BINANCE_TEST_API_KEY: &str =
    "AekFIdmCDmPkaeQjCjaPtEE9IvYtpoceePvvelkthAh7tEtvMAm7oHzcxkhbmxl0";
const BINANCE_TEST_API_SECRET: &str =
    "epU83XZHBcHuvznmccDQCbCcxbGeVq6sl4AspOyALCTqWkeG1CVlJx6BzXIC2wXK";
// Binance Spot Live Network API credentials
const BINANCE_LIVE_API: &str = "https://api.binance.us";
const BINANCE_LIVE_API_KEY: &str =
    "WeGpjrcMfU4Yndtb8tOqy2MQouEWsGuQbCwNHOwCSKtnxm5MUhqB6EOyQ3u7rBFY";
const BINANCE_LIVE_API_SECRET: &str =
    "aLfkivKBnH31bhfcOc1P7qdg7HxLRcjCRBMDdiViVXMfO64TFEYe6V1OKr0MjyJS";

lazy_static! {
    static ref ACCOUNT: Mutex<Account> = Mutex::new(Account {
        client: Client::new(
            Some(BINANCE_LIVE_API_KEY.to_string()),
            Some(BINANCE_LIVE_API_SECRET.to_string()),
            BINANCE_LIVE_API.to_string()
        ),
        recv_window: 5000,
        base_asset: "BTC".to_string(),
        quote_asset: "USDT".to_string(),
        ticker: "BTCUSDT".to_string(),
        active_order: None,
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
            .service(account_info)
            .service(get_assets)
            .service(cancel_orders)
            .service(get_price)
            .service(exchange_info)
            .service(all_orders)
            .service(open_order)
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
async fn account_info() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let res = account
        .account_info()
        .await
        .expect("failed to get account info");
    Ok(HttpResponse::Ok().json(res))
}

#[get("/assets")]
async fn get_assets() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let res = account.all_assets().await.expect("failed to get assets");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/cancel")]
async fn cancel_orders() -> Result<HttpResponse, Error> {
    info!("Cancel all active orders");
    let account = ACCOUNT.lock().await;
    let res = account
        .cancel_all_active_orders()
        .await
        .expect("failed to cancel orders");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/price")]
async fn get_price() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let res = account
        .get_price(account.ticker.clone())
        .await
        .expect("failed to get price");
    debug!("{:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/allOrders")]
async fn all_orders() -> Result<HttpResponse, Error> {
    info!("Fetching all historical orders...");
    let account = ACCOUNT.lock().await;
    let res = account
        .all_orders(account.ticker.clone())
        .await
        .expect("failed to get historical orders");
    let last = res.last().unwrap();
    info!(
        "Last order -- ID: {:?}, Status: {}",
        last.client_order_id, last.status
    );
    Ok(HttpResponse::Ok().json(res))
}

#[get("/openOrder")]
async fn open_order() -> Result<HttpResponse, Error> {
    info!("Fetching last open order...");
    let account = ACCOUNT.lock().await;
    let res = account
        .open_order(account.ticker.clone())
        .await
        .expect("failed to get last order");
    info!("Last open order: {:?}", res);
    Ok(HttpResponse::Ok().json(res))
}

#[get("/info")]
async fn exchange_info() -> Result<HttpResponse, Error> {
    let account = ACCOUNT.lock().await;
    let info = account
        .exchange_info(account.ticker.clone())
        .await
        .expect("Failed to get exchange info");
    Ok(HttpResponse::Ok().json(info))
}
