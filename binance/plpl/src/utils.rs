use crossbeam::channel::Sender;
use library::*;
use log::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use time_series::{Candle, Time};

pub fn init_logger(log_file: &PathBuf) {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            SimpleLogConfig::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            ConfigBuilder::new()
                .set_time_format_custom(simplelog::format_description!(
                    "[hour]:[minute]:[second].[subsecond]"
                ))
                .build(),
            File::create(log_file).expect("Failed to create PLPL Binance log file"),
        ),
    ])
    .expect("Failed to initialize logger");
}

pub fn kline_to_candle(kline_event: &KlineEvent) -> Candle {
    let date = Time::from_unix_msec(kline_event.event_time as i64);
    Candle {
        date,
        open: kline_event
            .kline
            .open
            .parse::<f64>()
            .expect("Failed to parse Kline open to f64"),
        high: kline_event
            .kline
            .high
            .parse::<f64>()
            .expect("Failed to parse Kline high to f64"),
        low: kline_event
            .kline
            .low
            .parse::<f64>()
            .expect("Failed to parse Kline low to f64"),
        close: kline_event
            .kline
            .close
            .parse::<f64>()
            .expect("Failed to parse Kline close to f64"),
        volume: None,
    }
}

pub fn free_asset(account_info: &AccountInfoResponse, asset: &str) -> f64 {
    account_info
        .balances
        .iter()
        .find(|&x| x.asset == asset)
        .unwrap()
        .free
        .parse::<f64>()
        .unwrap()
}

pub fn locked_asset(account_info: &AccountInfoResponse, asset: &str) -> f64 {
    account_info
        .balances
        .iter()
        .find(|&x| x.asset == asset)
        .unwrap()
        .locked
        .parse::<f64>()
        .unwrap()
}

pub struct Assets {
    free_quote: f64,
    locked_quote: f64,
    free_base: f64,
    locked_base: f64,
}

pub fn account_assets(
    account: &AccountInfoResponse,
    quote_asset: &str,
    base_asset: &str,
) -> Assets {
    let free_quote = free_asset(account, quote_asset);
    let locked_quote = locked_asset(account, quote_asset);
    let free_base = free_asset(account, base_asset);
    let locked_base = locked_asset(account, base_asset);
    Assets {
        free_quote,
        locked_quote,
        free_base,
        locked_base,
    }
}

pub fn trade_qty(
    account_info: &AccountInfoResponse,
    quote_asset: &str,
    base_asset: &str,
    side: Side,
    candle: &Candle,
) -> f64 {
    let assets = account_assets(account_info, quote_asset, base_asset);
    info!(
        "{}: {}, {}: {}",
        quote_asset,
        assets.free_quote + assets.locked_quote,
        base_asset,
        assets.free_base + assets.locked_base,
    );
    match side {
        Side::Long => {
            let qty = assets.free_quote / candle.close * 0.99;
            BinanceTrade::round_quantity(qty, 5)
        }
        Side::Short => {
            let qty = assets.free_base * 0.99;
            BinanceTrade::round_quantity(qty, 5)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn plpl_long(
    account_info: &AccountInfoResponse,
    client_order_id: &str,
    candle: &Candle,
    trailing_stop_pct: f64,
    stop_loss_pct: f64,
    ticker: &str,
    quote_asset: &str,
    base_asset: &str,
) -> Vec<BinanceTrade> {
    let long_qty = trade_qty(account_info, quote_asset, base_asset, Side::Long, candle);
    let limit = BinanceTrade::round_price(candle.close);
    let entry = BinanceTrade::new(
        ticker.to_string(),
        Side::Long,
        OrderType::Limit,
        long_qty,
        client_order_id.to_string(),
        Some(limit),
        None,
        None,
        Some(5000),
    );
    let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop_pct);
    let profit = BinanceTrade::new(
        ticker.to_string(),
        Side::Short,
        OrderType::TakeProfitLimit,
        long_qty,
        client_order_id.to_string(),
        Some(limit),
        None,
        Some(trailing_stop),
        Some(5000),
    );
    let stop_loss = BinanceTrade::calc_stop_loss(Side::Long, candle.close, stop_loss_pct);
    let loss = BinanceTrade::new(
        ticker.to_string(),
        Side::Short,
        OrderType::StopLossLimit,
        long_qty,
        client_order_id.to_string(),
        Some(stop_loss), // price in this context is the actual exit (stop loss)
        Some(limit), // stopPrice is the trigger to place the stop loss order, in this case the entry price
        None,
        Some(5000),
    );
    vec![entry, profit, loss]
}

#[allow(clippy::too_many_arguments)]
pub fn plpl_short(
    account_info: &AccountInfoResponse,
    client_order_id: &str,
    candle: &Candle,
    trailing_stop_pct: f64,
    stop_loss_pct: f64,
    ticker: &str,
    quote_asset: &str,
    base_asset: &str,
) -> Vec<BinanceTrade> {
    let short_qty = trade_qty(account_info, quote_asset, base_asset, Side::Short, candle);
    let limit = BinanceTrade::round_price(candle.close);
    let entry = BinanceTrade::new(
        ticker.to_string(),
        Side::Short,
        OrderType::Limit,
        short_qty,
        client_order_id.to_string(),
        Some(limit),
        None,
        None,
        Some(5000),
    );
    let trailing_stop = BinanceTrade::bips_trailing_stop(trailing_stop_pct);
    let profit = BinanceTrade::new(
        ticker.to_string(),
        Side::Long,
        OrderType::TakeProfitLimit,
        short_qty,
        client_order_id.to_string(),
        Some(limit),
        None,
        Some(trailing_stop),
        Some(5000),
    );
    let stop_loss = BinanceTrade::calc_stop_loss(Side::Short, candle.close, stop_loss_pct);
    let loss = BinanceTrade::new(
        ticker.to_string(),
        Side::Long,
        OrderType::StopLossLimit,
        short_qty,
        client_order_id.to_string(),
        Some(stop_loss), // price is this content is the actual exit (stop loss)
        Some(limit), // stopPrice is the trigger to place the stop loss order, in this case the entry price
        None,
        Some(5000),
    );
    vec![entry, profit, loss]
}

/// Kline stream and User Stream of account and order updates
pub async fn handle_streams(
    user_stream: &UserStream,
    kline_sub: String,
    keep_running: AtomicBool,
    queue_tx: Sender<WebSocketEvent>,
) -> Result<()> {
    match user_stream.start().await {
        Ok(answer) => {
            let listen_key = answer.listen_key;
            let mut ws = WebSockets::new(|event: WebSocketEvent| {
                match &event {
                    WebSocketEvent::Kline(kline_event) => {
                        let res = queue_tx.send(event);
                        if let Err(e) = res {
                            error!("Failed to send Kline event to queue: {:?}", e);
                        }
                    }
                    WebSocketEvent::AccountUpdate(account_update) => {
                        for balance in &account_update.data.balances {
                            info!(
                                "Asset: {}, wallet_balance: {}, cross_wallet_balance: {}, balance: {}",
                                balance.asset,
                                balance.wallet_balance,
                                balance.cross_wallet_balance,
                                balance.balance_change
                            );
                        }
                    }
                    WebSocketEvent::OrderTrade(trade) => {
                        info!(
                            "Symbol: {}, Side: {}, Price: {}, Execution Type: {}",
                            trade.symbol, trade.side, trade.price, trade.execution_type
                        );
                        let res = queue_tx.send(event);
                        if let Err(e) = res {
                            error!("Failed to send OrderTrade event to queue: {:?}", e);
                        }
                    }
                    _ => (),
                };
                Ok(())
            });

            let subs = vec![kline_sub, listen_key.clone()];
            match ws.connect_multiple_streams(&subs) {
                Err(e) => {
                    error!("Failed to connect to Binance websocket: {}", e);
                    return Err(e);
                }
                Ok(_) => info!("Binance websocket connected"),
            }

            if let Err(e) = ws.event_loop(&keep_running) {
                error!("Binance websocket error: {}", e);
            }

            user_stream.close(&listen_key).await?;

            return match ws.disconnect() {
                Err(e) => {
                    error!("Failed to disconnect from Binance websocket: {}", e);
                    match ws.connect_multiple_streams(&subs) {
                        Err(e) => {
                            error!("Failed to connect to Binance websocket: {}", e);
                            Err(e)
                        }
                        Ok(_) => {
                            info!("Binance websocket connected");
                            Ok(())
                        }
                    }
                }
                Ok(_) => {
                    info!("Binance websocket disconnected");
                    Ok(())
                }
            };
        }
        Err(e) => {
            error!("Error starting websocket {:?}", e);
            Err(e)
        }
    }
}
