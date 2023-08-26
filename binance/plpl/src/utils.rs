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

pub fn trade_qty(
    account_info: &AccountInfoResponse,
    quote_asset: &str,
    base_asset: &str,
    side: Side,
    candle: &Candle,
) -> Result<f64> {
    let assets = account_info.account_assets(quote_asset, base_asset)?;
    info!(
        "{}, Free: {}, Locked: {}",
        quote_asset, assets.free_quote, assets.locked_quote,
    );
    info!(
        "{}, Free: {}, Locked: {}",
        base_asset, assets.free_base, assets.locked_base,
    );
    // if long, check short has 2x balance for exit order
    // if short, check long has 2x balance for exit order
    // if not, error
    let long_qty = assets.free_quote / candle.close * 1.0 / 3.0;
    let short_qty = assets.free_base * 0.33;

    Ok(match side {
        Side::Long => {
            let qty = match long_qty > short_qty / 2.0 {
                true => short_qty / 2.0,
                false => long_qty,
            };
            BinanceTrade::round(qty, 5)
        }
        Side::Short => {
            let qty = match short_qty > long_qty / 2.0 {
                true => long_qty / 2.0,
                false => short_qty,
            };
            BinanceTrade::round(qty, 5)
        }
    })
}

pub struct OrderBuilder {
    pub entry: BinanceTrade,
    pub take_profit: BinanceTrade,
    pub stop_loss: BinanceTrade,
    pub take_profit_tracker: TrailingTakeProfitTracker,
    pub stop_loss_tracker: StopLossTracker,
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
) -> Result<OrderBuilder> {
    // each order gets 1/3 of 99% of account balance
    // 99% is to account for fees
    // 1/3 is to account for 3 orders
    let long_qty = trade_qty(account_info, quote_asset, base_asset, Side::Long, candle)?;
    let limit = BinanceTrade::round_price(candle.close);
    let entry = BinanceTrade::new(
        ticker.to_string(),
        format!("{}-{}", timestamp, "ENTRY"),
        Side::Long,
        OrderType::Limit,
        long_qty,
        Some(limit),
        None,
        None,
        Some(10000),
    );
    let trailing_take_profit_tracker =
        TrailingTakeProfitTracker::new(limit, trailing_take_profit, Side::Short);
    let profit = BinanceTrade::new(
        ticker.to_string(),
        format!("{}-{}", timestamp, "TAKE_PROFIT"),
        Side::Short,
        OrderType::Limit,
        long_qty,
        Some(trailing_take_profit_tracker.trigger),
        None,
        None,
        Some(10000),
    );
    let stop_loss_tracker = StopLossTracker::new(limit, stop_loss, Side::Short);
    let loss = BinanceTrade::new(
        ticker.to_string(),
        format!("{}-{}", timestamp, "STOP_LOSS"),
        Side::Short,
        OrderType::Limit,
        long_qty,
        Some(stop_loss_tracker.trigger),
        None,
        None,
        Some(10000),
    );
    Ok(OrderBuilder {
        entry,
        take_profit: profit,
        stop_loss: loss,
        take_profit_tracker: trailing_take_profit_tracker,
        stop_loss_tracker,
    })
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
) -> Result<OrderBuilder> {
    let short_qty = trade_qty(account_info, quote_asset, base_asset, Side::Short, candle)?;
    let limit = BinanceTrade::round_price(candle.close);
    let entry = BinanceTrade::new(
        ticker.to_string(),
        format!("{}-{}", timestamp, "ENTRY"),
        Side::Short,
        OrderType::Limit,
        short_qty,
        Some(limit),
        None,
        None,
        Some(10000),
    );
    let trailing_take_profit_tracker =
        TrailingTakeProfitTracker::new(limit, trailing_take_profit, Side::Long);
    let profit = BinanceTrade::new(
        ticker.to_string(),
        format!("{}-{}", timestamp, "TAKE_PROFIT"),
        Side::Long,
        OrderType::Limit,
        short_qty,
        Some(trailing_take_profit_tracker.trigger),
        None,
        None,
        Some(10000),
    );
    let stop_loss_tracker = StopLossTracker::new(limit, stop_loss, Side::Long);
    let loss = BinanceTrade::new(
        ticker.to_string(),
        format!("{}-{}", timestamp, "STOP_LOSS"),
        Side::Long,
        OrderType::Limit,
        short_qty,
        Some(stop_loss_tracker.trigger),
        None,
        None,
        Some(10000),
    );
    Ok(OrderBuilder {
        entry,
        take_profit: profit,
        stop_loss: loss,
        take_profit_tracker: trailing_take_profit_tracker,
        stop_loss_tracker,
    })
}

fn check_trailing_take_profit(
    candle: &Candle,
    date: &Time,
    account: &mut MutexGuard<Account>,
    mut active_order: OrderBundle,
) -> Result<Option<OrderBundle>> {
    let take_profit_action = active_order.take_profit_tracker.check(candle);
    match take_profit_action {
        UpdateAction::None => debug!("Take profit updated"),
        UpdateAction::Close => {
            info!(
                "Take profit triggered @ {} | {}",
                candle.close,
                date.to_string()
            );
            account.cancel_all_open_orders()?;
            account.active_order = None;
        }
        UpdateAction::CancelAndUpdate => {
            // cancel take profit order and place new one
            match active_order.take_profit {
                PendingOrActiveOrder::Empty => {
                    error!("No take profit order to cancel");
                    return Err(BinanceError::Custom(
                        "No take profit order to cancel".to_string(),
                    ));
                }
                PendingOrActiveOrder::Active(tp) => {
                    // cancel exiting trailing take profit order
                    let res = account.cancel_order(tp.order_id)?;
                    let orig_client_order_id =
                        res.orig_client_order_id.ok_or(BinanceError::Custom(
                            "OrderCanceled orig client order id is none".to_string(),
                        ))?;
                    info!("Cancel and update take profit: {:?}", orig_client_order_id);
                    // place new take profit order with updated trigger price
                    let exit_side = active_order.take_profit_tracker.exit_side;
                    let trade = BinanceTrade::new(
                        res.symbol,
                        orig_client_order_id,
                        exit_side.clone(),
                        OrderType::Limit,
                        tp.quantity,
                        Some(active_order.take_profit_tracker.trigger),
                        None,
                        None,
                        Some(10000),
                    );
                    if let Err(e) = account.trade::<LimitOrderResponse>(trade) {
                        error!(
                            "Error updating take profit {} with error: {:?}",
                            exit_side.fmt_binance(),
                            e
                        );
                        account.cancel_all_open_orders()?;
                        account.active_order = None;
                        return Err(e);
                    }
                }
                PendingOrActiveOrder::Pending(_) => {
                    debug!("Take profit order is pending, ignore cancel and update");
                }
            }
        }
    }
    Ok(account.active_order.clone())
}

#[allow(clippy::too_many_arguments)]
fn handle_long_signal(
    candle: &Candle,
    date: &Time,
    timestamp: String,
    account: &mut MutexGuard<Account>,
    trailing_take_profit: ExitType,
    stop_loss: ExitType,
) -> Result<()> {
    info!(
        "No active order, enter Long @ {} | {}",
        candle.close,
        date.to_string()
    );
    let account_info = account.account_info()?;
    let order_builder = plpl_long(
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
        PendingOrActiveOrder::Pending(order_builder.take_profit),
        order_builder.take_profit_tracker,
        PendingOrActiveOrder::Pending(order_builder.stop_loss),
        order_builder.stop_loss_tracker,
    ));
    account.log_active_order();

    // place entry order
    let side = order_builder.entry.side.clone();
    let client_order_id = order_builder.entry.client_order_id.clone();
    let order_type = OrderBundle::client_order_id_suffix(&client_order_id);
    if let Err(e) = account.trade::<LimitOrderResponse>(order_builder.entry) {
        error!(
            "Error entering {} for {}: {:?}",
            side.fmt_binance(),
            order_type,
            e
        );
        account.cancel_all_open_orders()?;
        account.active_order = None;
        return Err(e);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_short_signal(
    candle: &Candle,
    date: &Time,
    timestamp: String,
    account: &mut MutexGuard<Account>,
    trailing_take_profit: ExitType,
    stop_loss: ExitType,
) -> Result<()> {
    // if position is None, enter Short
    // else ignore signal and let active trade play out
    info!(
        "No active order, enter Short @ {} | {}",
        candle.close,
        date.to_string()
    );
    let account_info = account.account_info()?;
    let order_builder = plpl_short(
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
        PendingOrActiveOrder::Pending(order_builder.take_profit),
        order_builder.take_profit_tracker,
        PendingOrActiveOrder::Pending(order_builder.stop_loss),
        order_builder.stop_loss_tracker,
    ));
    account.log_active_order();

    // place entry order
    let side = order_builder.entry.side.clone();
    let order_type = OrderBundle::client_order_id_suffix(&order_builder.entry.client_order_id);
    if let Err(e) = account.trade::<LimitOrderResponse>(order_builder.entry) {
        error!(
            "Error entering {} for {}: {:?}",
            side.fmt_binance(),
            order_type,
            e
        );
        account.cancel_all_open_orders()?;
        account.active_order = None;
        return Err(e);
    }
    Ok(())
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
                handle_short_signal(
                    candle,
                    date,
                    timestamp,
                    account,
                    trailing_take_profit,
                    stop_loss,
                )?;
                trade_placed = true;
            } else if plpl_system.short_signal(prev_candle, candle, plpl) {
                // if position is None, enter Short
                // else ignore signal and let active trade play out
                handle_long_signal(
                    candle,
                    date,
                    timestamp,
                    account,
                    trailing_take_profit,
                    stop_loss,
                )?;
                trade_placed = true;
            }
        }
        Some(active_order) => {
            match (&active_order.take_profit, &active_order.stop_loss) {
                // check if trailing take profit should be updated
                (PendingOrActiveOrder::Active(_), PendingOrActiveOrder::Active(_)) => {
                    account.active_order =
                        check_trailing_take_profit(candle, date, account, active_order)?;
                }
                // check if exit orders should be placed
                (
                    PendingOrActiveOrder::Pending(take_profit),
                    PendingOrActiveOrder::Pending(stop_loss),
                ) => {
                    // only place exit orders if entry is filled
                    if let Some(entry) = active_order.entry {
                        if entry.status == OrderStatus::PartiallyFilled
                            || entry.status == OrderStatus::Filled
                        {
                            // place take profit order
                            let side = take_profit.side.clone();
                            let order_type =
                                OrderBundle::client_order_id_suffix(&take_profit.client_order_id);
                            if let Err(e) = account.trade::<LimitOrderResponse>(take_profit.clone())
                            {
                                error!(
                                    "Error entering {} for {}: {:?}",
                                    side.fmt_binance(),
                                    order_type,
                                    e
                                );
                                account.cancel_all_open_orders()?;
                                account.active_order = None;
                                return Err(e);
                            }

                            // place stop loss order
                            let side = stop_loss.side.clone();
                            let order_type =
                                OrderBundle::client_order_id_suffix(&stop_loss.client_order_id);
                            if let Err(e) = account.trade::<LimitOrderResponse>(stop_loss.clone()) {
                                error!(
                                    "Error entering {} for {}: {:?}",
                                    side.fmt_binance(),
                                    order_type,
                                    e
                                );
                                account.cancel_all_open_orders()?;
                                account.active_order = None;
                                return Err(e);
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
    Ok(trade_placed)
}
