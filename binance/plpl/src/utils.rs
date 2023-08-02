use library::*;
use log::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;
use time_series::{Candle, Time};

pub fn init_logger(log_file: &PathBuf) -> Result<()> {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            SimpleLogConfig::default(),
            TerminalMode::Mixed,
            ColorChoice::Always,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            ConfigBuilder::new()
                .set_time_format_custom(simplelog::format_description!(
                    "[hour]:[minute]:[second].[subsecond]"
                ))
                .build(),
            File::create(log_file).map_err(|_| {
                BinanceError::Custom("Failed to create PLPL Binance log file".to_string())
            })?,
        ),
    ])
    .map_err(|_| BinanceError::Custom("Failed to initialize PLPL Binance logger".to_string()))
}

pub fn kline_to_candle(kline_event: &KlineEvent) -> Result<Candle> {
    let date = Time::from_unix_msec(kline_event.event_time as i64);
    Ok(Candle {
        date,
        open: kline_event
            .kline
            .open
            .parse::<f64>()
            .map_err(|_| BinanceError::Custom("Failed to parse Kline open to f64".to_string()))?,
        high: kline_event
            .kline
            .high
            .parse::<f64>()
            .map_err(|_| BinanceError::Custom("Failed to parse Kline high to f64".to_string()))?,
        low: kline_event
            .kline
            .low
            .parse::<f64>()
            .map_err(|_| BinanceError::Custom("Failed to parse Kline low to f64".to_string()))?,
        close: kline_event
            .kline
            .close
            .parse::<f64>()
            .map_err(|_| BinanceError::Custom("Failed to parse Kline close to f64".to_string()))?,
        volume: None,
    })
}

pub fn free_asset(account_info: &AccountInfoResponse, asset: &str) -> Result<f64> {
    let balances = account_info.balances.iter().find(|&x| x.asset == asset);
    match balances {
        Some(balances) => balances
            .free
            .parse::<f64>()
            .map_err(|e| BinanceError::Custom(format!("Failed to parse free asset {}", asset))),
        None => Err(BinanceError::Custom(format!(
            "Failed to find asset {}",
            asset
        ))),
    }
}

pub fn locked_asset(account_info: &AccountInfoResponse, asset: &str) -> Result<f64> {
    let balances = account_info.balances.iter().find(|&x| x.asset == asset);
    match balances {
        Some(balances) => balances
            .locked
            .parse::<f64>()
            .map_err(|e| BinanceError::Custom(format!("Failed to parse locked asset {}", asset))),
        None => Err(BinanceError::Custom(format!(
            "Failed to find asset {}",
            asset
        ))),
    }
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
) -> Result<Assets> {
    let free_quote = free_asset(account, quote_asset)?;
    let locked_quote = locked_asset(account, quote_asset)?;
    let free_base = free_asset(account, base_asset)?;
    let locked_base = locked_asset(account, base_asset)?;
    Ok(Assets {
        free_quote,
        locked_quote,
        free_base,
        locked_base,
    })
}

pub fn trade_qty(
    account_info: &AccountInfoResponse,
    quote_asset: &str,
    base_asset: &str,
    side: Side,
    candle: &Candle,
) -> Result<f64> {
    let assets = account_assets(account_info, quote_asset, base_asset)?;
    info!(
        "{}, Free: {}, Locked: {}",
        quote_asset, assets.free_quote, assets.locked_quote,
    );
    info!(
        "{}, Free: {}, Locked: {}",
        base_asset, assets.free_base, assets.locked_base,
    );
    Ok(match side {
        Side::Long => {
            let qty = assets.free_quote / candle.close * 0.99 * 0.33;
            BinanceTrade::round_quantity(qty, 5)
        }
        Side::Short => {
            let qty = assets.free_base * 0.99 * 0.33;
            BinanceTrade::round_quantity(qty, 5)
        }
    })
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
) -> Result<Vec<BinanceTrade>> {
    // each order gets 1/3 of 99% of account balance
    // 99% is to account for fees
    // 1/3 is to account for 3 orders
    let long_qty = trade_qty(account_info, quote_asset, base_asset, Side::Long, candle)?;
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
    let stop_price = BinanceTrade::round_price(stop_loss - (stop_loss - limit) / 2.0);
    let loss = BinanceTrade::new(
        ticker.to_string(),
        Side::Short,
        OrderType::StopLossLimit,
        long_qty,
        client_order_id.to_string(),
        Some(stop_loss),  // price in this context is the actual exit (stop loss)
        Some(stop_price), // stopPrice is the trigger to place the stop loss order, in this case the entry price
        None,
        Some(5000),
    );
    Ok(vec![entry, profit, loss])
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
) -> Result<Vec<BinanceTrade>> {
    let short_qty = trade_qty(account_info, quote_asset, base_asset, Side::Short, candle)?;
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
    let stop_price = BinanceTrade::round_price(stop_loss + (limit - stop_loss) / 2.0);
    let loss = BinanceTrade::new(
        ticker.to_string(),
        Side::Long,
        OrderType::StopLossLimit,
        short_qty,
        client_order_id.to_string(),
        Some(stop_loss),  // price is this context is the actual exit (stop loss)
        Some(stop_price), // stopPrice is the trigger to place the stop loss order
        None,
        Some(5000),
    );
    Ok(vec![entry, profit, loss])
}
