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
use builder::trade::BinanceTrade;
use response::*;
use account::Account;
use config::Config;
use websocket::{WebSockets, WebSocketEvent};

use actix_web::{error, post, web, App, HttpResponse, HttpServer, Responder, Error, Result, get};
use regex::Regex;
use std::sync::Mutex;
use log::*;
use simplelog::{
    ColorChoice, Config as SimpleLogConfig,
    TermLogger, TerminalMode,
};
use std::sync::atomic::AtomicBool;
use time_series::{Candle, Time, Trade, Order};
use ephemeris::*;
use futures::StreamExt;

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
          .service(plpl)
          .route("/", web::get().to(test))
    })
      .bind(bind_address)?
      .run()
      .await
}

async fn test() -> impl Responder {
    HttpResponse::Ok().body("Server is running...")
}

fn init_logger() {
    TermLogger::init(
        LevelFilter::Info,
        SimpleLogConfig::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ).expect("Failed to initialize logger");
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
            AlertOrder::Enter => {
                // get account token balances
                let account_info = account.account_info().expect("failed to get account info");
                let busd_balance = &account_info.balances.iter().find(|&x| x.asset == account.quote_asset).unwrap().free;
                #[allow(unused_variables)]
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
                        let trade = BinanceTrade::new(
                            account.ticker.clone(),
                            alert.side,
                            OrderType::Market,
                            qty,
                            None,
                            None,
                            None
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
                        let trade = BinanceTrade::new(
                            account.ticker.clone(),
                            alert.side,
                            OrderType::Market,
                            qty,
                            None,
                            None,
                            None
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
            AlertOrder::Exit => {
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
                                let trade = BinanceTrade::new(
                                    account.ticker.clone(),
                                    Side::Short,
                                    OrderType::Market,
                                    qty,
                                    None,
                                    None,
                                    None
                                );
                                let res = account.trade::<OrderResponse>(trade)
                                  .expect("failed to exit long");
                                debug!("{:?}", res);
                                account.set_active_order(None);
                                res
                            },
                            Side::Short => {
                                info!("Exit Short");
                                let trade = BinanceTrade::new(
                                    account.ticker.clone(),
                                    Side::Long,
                                    OrderType::Market,
                                    qty,
                                    None,
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

#[get("/plpl")]
async fn plpl() -> Result<HttpResponse, Error> {
    info!("Starting Binance PLPL!");
    let config = Config::testnet();
    let keep_running = AtomicBool::new(true);

    let prev_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let curr_candle: Mutex<Option<Candle>> = Mutex::new(None);
    let mut account = ACCOUNT.lock().unwrap();

    let trailing_stop = 0.95;
    #[allow(unused_variables)]
    let stop_loss_pct = 0.001;

    let mut ws = WebSockets::new(|event: WebSocketEvent| {
        if let WebSocketEvent::Kline(kline_event) = event {
            let date = Time::from_unix_msec(kline_event.event_time as i64);

            // initialize PLPL
            let plpl_system =
              PLPLSystem::new(PLPLSystemConfig {
                planet: Planet::from("Jupiter"),
                origin: Origin::Heliocentric,
                date,
                plpl_scale: 0.5,
                plpl_price: 20000.0,
                num_plpls: 2000,
                cross_margin_pct: 55.0
            }).expect("Failed to create PLPLSystem");
            debug!("PLPLSystem initialized");

            let candle = Candle {
                date,
                open: kline_event.kline.open.parse::<f64>().expect("Failed to parse Kline open to f64"),
                high: kline_event.kline.high.parse::<f64>().expect("Failed to parse Kline high to f64"),
                low: kline_event.kline.low.parse::<f64>().expect("Failed to parse Kline low to f64"),
                close: kline_event.kline.close.parse::<f64>().expect("Failed to parse Kline close to f64"),
                volume: None
            };

            // cache previous and current candle to assess PLPL trade conditions
            let mut prev = prev_candle.lock().expect("Failed to lock previous candle");
            let mut curr = curr_candle.lock().expect("Failed to lock current candle");
            let plpl = plpl_system.closest_plpl(&candle).expect("Failed to get closest plpl");

            // get account balance for BTC and BUSD
            // get account token balances
            let account_info = account.account_info().expect("failed to get account info");
            let busd_balance = account_info.balances.iter().find(|&x| x.asset == account.quote_asset)
              .unwrap().free
              .parse::<f64>().unwrap();
            info!("BUSD balance: {}", busd_balance);
            let btc_balance = account_info.balances.iter().find(|&x| x.asset == account.base_asset)
              .unwrap().free
              .parse::<f64>().unwrap();
            info!("BTC balance: {}", btc_balance);
            // get current price of symbol
            info!("Current price: {}", candle.close);

            // calculate quantity of base asset to trade
            // Trade with $1000 or as close as the account can get
            let mut long_qty = 0.0;
            if btc_balance * candle.close < 1000.0 {
               long_qty = btc_balance;
            } else {
                long_qty = BinanceTrade::round_quantity(1000.0/candle.close);
            }
            let mut short_qty = 0.0;
            if busd_balance < 1000.0 {
                short_qty = busd_balance;
            } else {
                short_qty = BinanceTrade::round_quantity(1000.0/candle.close);
            }

            match (&*prev, &*curr) {
                (None, None) => {
                    debug!("prev none & curr none");
                    *prev = Some(candle);
                },
                (Some(prev_candle), None) => {
                    debug!("prev some & curr none");
                    *curr = Some(candle.clone());
                    if plpl_system.long_signal(prev_candle, &candle, plpl) {
                        // if position is Long, ignore
                        // if position is Short, close short and open Long
                        // if position is None, enter Long
                        match account.get_active_order() {
                            None => {
                                info!("No active order, enter Long");
                                match account.cancel_all_active_orders() {
                                    Err(_) => {
                                        warn!("No active orders to cancel with error");
                                    },
                                    Ok(_) => {
                                        info!("All active orders canceled");
                                    }
                                }
                                let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop);
                                #[allow(unused_variables)]
                                let stop_loss = Trade::calc_stop_loss(Order::Long, candle.close, stop_loss_pct);
                                #[allow(unused_variables)]
                                let limit = BinanceTrade::round_quantity(candle.close);
                                let trade = BinanceTrade::new(
                                    account.ticker.clone(),
                                    Side::Long,
                                    OrderType::StopLossLimit,
                                    long_qty,
                                    Some(limit),
                                    None,
                                    Some(trailing_stop),
                                );
                                let res = account.trade::<OrderResponse>(trade).expect("Failed to enter Long");
                                debug!("{:?}", res);
                                let active_order = res;
                                account.set_active_order(Some(active_order));
                            },
                            Some(active_order) => {
                                match active_order.side() {
                                    Side::Long => {
                                        info!("Already Long, ignoring");
                                    },
                                    Side::Short => {
                                        info!("Close Short, enter Long");
                                        match account.cancel_all_active_orders() {
                                            Err(_) => {
                                                warn!("No active orders to cancel");
                                            },
                                            Ok(_) => {
                                                info!("All active orders canceled");
                                            }
                                        }
                                        let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop);
                                        #[allow(unused_variables)]
                                        let stop_loss = Trade::calc_stop_loss(Order::Long, candle.close, stop_loss_pct);
                                        #[allow(unused_variables)]
                                        let limit = BinanceTrade::round_quantity(candle.close);
                                        let trade = BinanceTrade::new(
                                            account.ticker.clone(),
                                            Side::Long,
                                            OrderType::StopLossLimit,
                                            long_qty,
                                            Some(limit),
                                            None,
                                            Some(trailing_stop),
                                        );
                                        let res = account.trade::<OrderResponse>(trade).expect("Failed to enter Long");
                                        debug!("{:?}", res);
                                        let active_order = res;
                                        account.set_active_order(Some(active_order));
                                    }
                                }
                            }
                        }

                        info!(
                            "Long {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                            kline_event.kline.symbol, date.to_string(), prev_candle.close, candle.close, plpl
                        );
                    } else if plpl_system.short_signal(prev_candle, &candle, plpl) {
                        // if position is Short, ignore
                        // if position is Long, close long and open Short
                        // if position is None, enter Short
                        match account.get_active_order() {
                            None => {
                                match account.cancel_all_active_orders() {
                                    Err(_) => {
                                        warn!("No active orders to cancel");
                                    },
                                    Ok(_) => {
                                        info!("All active orders canceled");
                                    }
                                }
                                info!("No active order, enter Short");
                                let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop);
                                #[allow(unused_variables)]
                                let stop_loss = Trade::calc_stop_loss(Order::Short, candle.close, stop_loss_pct);
                                #[allow(unused_variables)]
                                let limit = BinanceTrade::round_quantity(candle.close);
                                println!("limit: {}", limit);
                                let trade = BinanceTrade::new(
                                    account.ticker.clone(),
                                    Side::Short,
                                    OrderType::StopLossLimit,
                                    short_qty,
                                    Some(limit),
                                    None,
                                    Some(trailing_stop),
                                );
                                let res = account.trade::<OrderResponse>(trade).expect("Failed to enter Short");
                                debug!("{:?}", res);
                                let active_order = res;
                                account.set_active_order(Some(active_order));
                            },
                            Some(active_order) => {
                                match active_order.side() {
                                    Side::Long => {
                                        match account.cancel_all_active_orders() {
                                            Err(_) => {
                                                warn!("No active orders to cancel");
                                            },
                                            Ok(_) => {
                                                info!("All active orders canceled");
                                            }
                                        }
                                        info!("Close Long, enter Short");
                                        let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop);
                                        #[allow(unused_variables)]
                                        let stop_loss = Trade::calc_stop_loss(Order::Short, candle.close, stop_loss_pct);
                                        #[allow(unused_variables)]
                                        let limit = BinanceTrade::round_quantity(candle.close);
                                        let trade = BinanceTrade::new(
                                            account.ticker.clone(),
                                            Side::Short,
                                            OrderType::StopLossLimit,
                                            short_qty,
                                            Some(limit),
                                            None,
                                            Some(trailing_stop),
                                        );
                                        let res = account.trade::<OrderResponse>(trade).expect("Failed to enter Short");
                                        debug!("{:?}", res);
                                        let active_order = res;
                                        account.set_active_order(Some(active_order));
                                    },
                                    Side::Short => {
                                        info!("Already Short, ignoring");
                                    }
                                }
                            }
                        }

                        info!(
                            "Short {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                            kline_event.kline.symbol, date.to_string(), prev_candle.close, candle.close, plpl
                        );
                    }
                },
                (None, Some(_)) => {
                    error!("Previous candle is None and current candle is Some. Should never occur!");
                    unreachable!()
                },
                (Some(_prev_candle), Some(curr_candle)) => {
                    debug!("prev some & curr some");
                    if plpl_system.long_signal(curr_candle, &candle, plpl) {
                        // if position is Long, ignore
                        // if position is Short, close short and enter Long
                        // if position is None, enter Long
                        match account.get_active_order() {
                            None => {
                                info!("No active order, enter Long");
                                match account.cancel_all_active_orders() {
                                    Err(_) => {
                                        info!("No active orders to cancel");
                                    },
                                    Ok(_) => {
                                        info!("All active orders canceled");
                                    }
                                }
                                let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop);
                                #[allow(unused_variables)]
                                let stop_loss = Trade::calc_stop_loss(Order::Long, candle.close, stop_loss_pct);
                                #[allow(unused_variables)]
                                let limit = BinanceTrade::round_quantity(candle.close);
                                let trade = BinanceTrade::new(
                                    account.ticker.clone(),
                                    Side::Long,
                                    OrderType::StopLossLimit,
                                    long_qty,
                                    Some(limit),
                                    None,
                                    Some(trailing_stop),
                                );
                                let res = account.trade::<OrderResponse>(trade).expect("Failed to enter Long");
                                debug!("{:?}", res);
                                let active_order = res;
                                account.set_active_order(Some(active_order));

                            },
                            Some(active_order) => {
                                match active_order.side() {
                                    Side::Long => {
                                        info!("Already Long, ignoring");
                                    },
                                    Side::Short => {
                                        info!("Close Short, enter Long");
                                        match account.cancel_all_active_orders() {
                                            Err(_) => {
                                                info!("No active orders to cancel");
                                            },
                                            Ok(_) => {
                                                info!("All active orders canceled");
                                            }
                                        }
                                        let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop);
                                        #[allow(unused_variables)]
                                        let stop_loss = Trade::calc_stop_loss(Order::Long, candle.close, stop_loss_pct);
                                        #[allow(unused_variables)]
                                        let limit = BinanceTrade::round_quantity(candle.close);
                                        let trade = BinanceTrade::new(
                                            account.ticker.clone(),
                                            Side::Long,
                                            OrderType::StopLossLimit,
                                            long_qty,
                                            Some(limit),
                                            None,
                                            Some(trailing_stop),
                                        );
                                        let res = account.trade::<OrderResponse>(trade).expect("Failed to enter Long");
                                        debug!("{:?}", res);
                                        let active_order = res;
                                        account.set_active_order(Some(active_order));
                                    }
                                }
                            }
                        }

                        info!(
                            "Long {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                            kline_event.kline.symbol, date.to_string(), curr_candle.close, candle.close, plpl
                        );
                    } else if plpl_system.short_signal(curr_candle, &candle, plpl) {
                        // if position is Short, ignore
                        // if position is Long, close long and enter Short
                        // if position is None, enter Short
                        match account.get_active_order() {
                            None => {
                                match account.cancel_all_active_orders() {
                                    Err(_) => {
                                        info!("No active orders to cancel");
                                    },
                                    Ok(_) => {
                                        info!("All active orders canceled");
                                    }
                                }
                                info!("No active order, enter Short");
                                let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop);
                                #[allow(unused_variables)]
                                let stop_loss = Trade::calc_stop_loss(Order::Short, candle.close, stop_loss_pct);
                                #[allow(unused_variables)]
                                let limit = BinanceTrade::round_quantity(candle.close);
                                let trade = BinanceTrade::new(
                                    account.ticker.clone(),
                                    Side::Short,
                                    OrderType::StopLossLimit,
                                    short_qty,
                                    Some(limit),
                                    None,
                                    Some(trailing_stop),
                                );
                                let res = account.trade::<OrderResponse>(trade).expect("Failed to enter Short");
                                debug!("{:?}", res);
                                let active_order = res;
                                account.set_active_order(Some(active_order));
                            },
                            Some(active_order) => {
                                match active_order.side() {
                                    Side::Long => {
                                        match account.cancel_all_active_orders() {
                                            Err(_) => {
                                                info!("No active orders to cancel");
                                            },
                                            Ok(_) => {
                                                info!("All active orders canceled");
                                            }
                                        }
                                        info!("Close Long, enter Short");
                                        let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop);
                                        #[allow(unused_variables)]
                                        let stop_loss = Trade::calc_stop_loss(Order::Short, candle.close, stop_loss_pct);
                                        #[allow(unused_variables)]
                                        let limit = BinanceTrade::round_quantity(candle.close);
                                        let trade = BinanceTrade::new(
                                            account.ticker.clone(),
                                            Side::Short,
                                            OrderType::StopLossLimit,
                                            short_qty,
                                            Some(limit),
                                            None,
                                            Some(trailing_stop),
                                        );
                                        let res = account.trade::<OrderResponse>(trade).expect("Failed to enter Short");
                                        debug!("{:?}", res);
                                        let active_order = res;
                                        account.set_active_order(Some(active_order));
                                    },
                                    Side::Short => {
                                        info!("Already Short, ignoring");
                                    }
                                }
                            }
                        }

                        info!(
                            "Short {} @ {}, Prev: {}, Curr: {}, PLPL: {}",
                            kline_event.kline.symbol, date.to_string(), curr_candle.close, candle.close, plpl
                        );
                    }
                    *prev = Some(curr_candle.clone());
                    *curr = Some(candle);
                }
            }
        }
        Ok(())
    });
    let sub = String::from("btcbusd@kline_5m");
    ws.connect_with_config(&sub, &config).expect("failed to connect to binance");
    if let Err(e) = ws.event_loop(&keep_running) {
        println!("Binance websocket error: {}", e);
    }
    ws.disconnect().unwrap();
    println!("Binance websocket disconnected");

    Ok(HttpResponse::Ok().body("Ok"))
}