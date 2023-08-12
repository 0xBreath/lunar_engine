use crate::{BASE_ASSET, QUOTE_ASSET, TICKER};
use ephemeris::PLPLSystem;
use library::*;
use log::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;
use std::sync::MutexGuard;
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
    account_info
        .balances
        .iter()
        .find(|&x| x.asset == asset)
        .ok_or(BinanceError::Custom(format!(
            "Failed to find asset {}",
            asset
        )))?
        .free
        .parse::<f64>()
        .map_err(|_| BinanceError::Custom(format!("Failed to parse free asset {}", asset)))
}

pub fn locked_asset(account_info: &AccountInfoResponse, asset: &str) -> Result<f64> {
    account_info
        .balances
        .iter()
        .find(|&x| x.asset == asset)
        .ok_or(BinanceError::Custom(format!(
            "Failed to find asset {}",
            asset
        )))?
        .locked
        .parse::<f64>()
        .map_err(|_| BinanceError::Custom(format!("Failed to parse locked asset {}", asset)))
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
            let qty = assets.free_quote / candle.close * 0.98 * 0.33;
            BinanceTrade::round(qty, 5)
        }
        Side::Short => {
            let qty = assets.free_base * 0.98 * 0.33;
            BinanceTrade::round(qty, 5)
        }
    })
}

#[allow(clippy::too_many_arguments)]
pub fn plpl_long(
    account_info: &AccountInfoResponse,
    timestamp: &str,
    candle: &Candle,
    trailing_take_profit: ExitType,
    stop_loss: ExitType,
    ticker: &str,
    quote_asset: &str,
    base_asset: &str,
) -> Result<(
    Vec<BinanceTrade>,
    TrailingTakeProfitTracker,
    StopLossTracker,
)> {
    // each order gets 1/3 of 99% of account balance
    // 99% is to account for fees
    // 1/3 is to account for 3 orders
    let long_qty = trade_qty(account_info, quote_asset, base_asset, Side::Long, candle)?;
    let limit = BinanceTrade::round_price(candle.close);
    let entry = BinanceTrade::new(
        ticker.to_string(),
        timestamp.to_string(),
        "ENTRY".to_string(),
        Side::Long,
        OrderType::Limit,
        long_qty,
        Some(limit),
        None,
        None,
        Some(5000),
    );
    let trailing_take_profit_tracker =
        TrailingTakeProfitTracker::new(limit, trailing_take_profit, Side::Long);
    let profit = BinanceTrade::new(
        ticker.to_string(),
        timestamp.to_string(),
        "TAKE_PROFIT".to_string(),
        Side::Short,
        OrderType::Limit,
        long_qty,
        Some(trailing_take_profit_tracker.trigger),
        None,
        None,
        Some(5000),
    );
    let stop_loss_tracker = StopLossTracker::new(limit, stop_loss, Side::Long);
    let loss = BinanceTrade::new(
        ticker.to_string(),
        timestamp.to_string(),
        "STOP_LOSS".to_string(),
        Side::Short,
        OrderType::Limit,
        long_qty,
        Some(stop_loss_tracker.trigger),
        None,
        None,
        Some(5000),
    );
    Ok((
        vec![entry, profit, loss],
        trailing_take_profit_tracker,
        stop_loss_tracker,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn plpl_short(
    account_info: &AccountInfoResponse,
    timestamp: &str,
    candle: &Candle,
    trailing_take_profit: ExitType,
    stop_loss: ExitType,
    ticker: &str,
    quote_asset: &str,
    base_asset: &str,
) -> Result<(
    Vec<BinanceTrade>,
    TrailingTakeProfitTracker,
    StopLossTracker,
)> {
    let short_qty = trade_qty(account_info, quote_asset, base_asset, Side::Short, candle)?;
    let limit = BinanceTrade::round_price(candle.close);
    let entry = BinanceTrade::new(
        ticker.to_string(),
        timestamp.to_string(),
        "ENTRY".to_string(),
        Side::Short,
        OrderType::Limit,
        short_qty,
        Some(limit),
        None,
        None,
        Some(5000),
    );
    let trailing_take_profit_tracker =
        TrailingTakeProfitTracker::new(limit, trailing_take_profit, Side::Short);
    let profit = BinanceTrade::new(
        ticker.to_string(),
        timestamp.to_string(),
        "TAKE_PROFIT".to_string(),
        Side::Long,
        OrderType::Limit,
        short_qty,
        Some(trailing_take_profit_tracker.trigger),
        None,
        None,
        Some(5000),
    );
    let stop_loss_tracker = StopLossTracker::new(limit, stop_loss, Side::Short);
    let loss = BinanceTrade::new(
        ticker.to_string(),
        timestamp.to_string(),
        "STOP_LOSS".to_string(),
        Side::Long,
        OrderType::Limit,
        short_qty,
        Some(stop_loss_tracker.trigger),
        None,
        None,
        Some(5000),
    );
    Ok((
        vec![entry, profit, loss],
        trailing_take_profit_tracker,
        stop_loss_tracker,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn handle_signal(
    plpl_system: &PLPLSystem,
    plpl: f32,
    prev_candle: &Candle,
    candle: &Candle,
    date: &Time,
    timestamp: String,
    account: &mut MutexGuard<Account>,
    active_order: Option<OrderBundle>,
    trailing_take_profit: ExitType,
    stop_loss: ExitType,
) -> Result<bool> {
    let mut trade_placed = false;
    match active_order {
        None => {
            if plpl_system.long_signal(prev_candle, candle, plpl) {
                // if position is None, enter Long
                // else ignore signal and let active trade play out
                info!(
                    "No active order, enter Long @ {} | {}",
                    candle.close,
                    date.to_string()
                );
                let account_info = account.account_info()?;
                let (trades, tp_tracker, sl_tracker) = plpl_long(
                    &account_info,
                    &timestamp,
                    candle,
                    trailing_take_profit,
                    stop_loss,
                    TICKER,
                    QUOTE_ASSET,
                    BASE_ASSET,
                )?;
                account.active_order = Some(OrderBundle::new(
                    None,
                    None,
                    Side::Long,
                    None,
                    None,
                    tp_tracker,
                    None,
                    sl_tracker,
                ));
                info!("{:?}", account.log_active_order());
                for trade in trades {
                    let side = trade.side.clone();
                    let client_order_id = trade.client_order_id.clone();
                    let order_type = OrderBundle::client_order_id_suffix(&client_order_id);
                    if let Err(e) = account.trade::<LimitOrderResponse>(trade) {
                        error!(
                            "Error entering {} for {}: {:?}",
                            side.fmt_binance(),
                            order_type,
                            e
                        );
                        account.cancel_all_active_orders()?;
                        account.active_order = None;
                        return Err(e);
                    }
                }
                trade_placed = true;
            } else if plpl_system.short_signal(prev_candle, candle, plpl) {
                // if position is None, enter Short
                // else ignore signal and let active trade play out
                info!(
                    "No active order, enter Short @ {} | {}",
                    candle.close,
                    date.to_string()
                );
                let account_info = account.account_info()?;
                let (trades, tp_tracker, sl_tracker) = plpl_short(
                    &account_info,
                    &timestamp,
                    candle,
                    trailing_take_profit,
                    stop_loss,
                    TICKER,
                    QUOTE_ASSET,
                    BASE_ASSET,
                )?;
                account.active_order = Some(OrderBundle::new(
                    None,
                    None,
                    Side::Short,
                    None,
                    None,
                    tp_tracker,
                    None,
                    sl_tracker,
                ));
                info!("{:?}", account.log_active_order());
                for trade in trades {
                    let side = trade.side.clone();
                    let order_type = OrderBundle::client_order_id_suffix(&trade.client_order_id);
                    if let Err(e) = account.trade::<LimitOrderResponse>(trade) {
                        error!(
                            "Error entering {} for {}: {:?}",
                            side.fmt_binance(),
                            order_type,
                            e
                        );
                        account.cancel_all_active_orders()?;
                        account.active_order = None;
                        return Err(e);
                    }
                }
                trade_placed = true;
            }
        }
        Some(mut active_order) => {
            let take_profit_action = active_order.take_profit_tracker.check(candle);
            match take_profit_action {
                UpdateAction::None => debug!("Take profit updated"),
                UpdateAction::Close => info!(
                    "Take profit triggered @ {} | {}",
                    candle.close,
                    date.to_string()
                ),
                UpdateAction::CancelAndUpdate => {
                    // cancel take profit order and place new one
                    match active_order.take_profit {
                        None => {
                            error!("No take profit order to cancel");
                            return Err(BinanceError::Custom(
                                "No take profit order to cancel".to_string(),
                            ));
                        }
                        Some(tp) => {
                            // cancel exiting trailing take profit order
                            let res = account.cancel_order(tp.order_id)?;
                            // place new take profit order with updated trigger price
                            let exit_side = active_order.take_profit_tracker.exit_side;
                            let trade = BinanceTrade::new(
                                res.symbol,
                                timestamp,
                                "TAKE_PROFIT".to_string(),
                                exit_side.clone(),
                                OrderType::Limit,
                                tp.quantity,
                                Some(active_order.take_profit_tracker.trigger),
                                None,
                                None,
                                Some(5000),
                            );
                            if let Err(e) = account.trade::<LimitOrderResponse>(trade) {
                                error!(
                                    "Error updating take profit {} with error: {:?}",
                                    exit_side.fmt_binance(),
                                    e
                                );
                                account.cancel_all_active_orders()?;
                                account.active_order = None;
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(trade_placed)
}
