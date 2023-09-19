use crate::utils::*;
use binance_lib::*;
use ephemeris::PLPLSystem;
use log::*;
use serde::de::DeserializeOwned;
use std::time::SystemTime;
use time_series::{precise_round, Candle};

#[derive(Clone)]
pub struct Engine {
    pub client: Client,
    pub plpl_system: PLPLSystem,
    pub recv_window: u64,
    pub base_asset: String,
    pub quote_asset: String,
    pub ticker: String,
    pub active_order: ActiveOrder,
    pub assets: Assets,
    pub prev_candle: Option<Candle>,
    pub candle: Option<Candle>,
}

impl Engine {
    #[allow(dead_code)]
    pub fn new(
        client: Client,
        plpl_system: PLPLSystem,
        recv_window: u64,
        base_asset: String,
        quote_asset: String,
        ticker: String,
        trailing_take_profit: ExitType,
        stop_loss: ExitType,
    ) -> Self {
        let take_profit_handler = TakeProfitHandler::new(trailing_take_profit.clone());
        let stop_loss_handler = StopLossHandler::new(stop_loss.clone());
        let active_order = ActiveOrder::new(take_profit_handler, stop_loss_handler);
        let prev_candle: Option<Candle> = None;
        let candle: Option<Candle> = None;
        Self {
            client,
            plpl_system,
            recv_window,
            base_asset,
            quote_asset,
            ticker,
            active_order,
            assets: Assets::default(),
            prev_candle,
            candle,
        }
    }

    #[allow(dead_code)]
    pub fn exchange_info(&self, symbol: String) -> Result<ExchangeInformation> {
        let req = ExchangeInfo::request(symbol);
        self.client
            .get::<ExchangeInformation>(API::Spot(Spot::ExchangeInfo), Some(req))
    }

    /// Place a trade
    pub fn trade<T: DeserializeOwned>(&self, trade: BinanceTrade) -> Result<T> {
        let req = trade.request();
        self.client.post_signed::<T>(API::Spot(Spot::Order), req)
    }

    pub fn trade_or_reset<T: DeserializeOwned>(&mut self, trade: BinanceTrade) -> Result<T> {
        let res = self.trade::<T>(trade.clone());
        match res {
            Ok(res) => Ok(res),
            Err(e) => {
                let order_type = ActiveOrder::client_order_id_suffix(&trade.client_order_id);
                error!(
                    "🛑 Error entering {} for {}: {:?}",
                    trade.side.fmt_binance(),
                    order_type,
                    e
                );
                self.reset_active_order()?;
                Err(e)
            }
        }
    }

    fn trade_qty(&self, side: Side, candle: &Candle) -> Result<f64> {
        let assets = self.assets();
        info!(
            "{}, Free: {}, Locked: {}  |  {}, Free: {}, Locked: {}",
            self.quote_asset,
            assets.free_quote,
            assets.locked_quote,
            self.base_asset,
            assets.free_base,
            assets.locked_base
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
                precise_round!(qty, 5)
            }
            Side::Short => {
                let qty = match short_qty > long_qty / 2.0 {
                    true => long_qty / 2.0,
                    false => short_qty,
                };
                precise_round!(qty, 5)
            }
        })
    }

    fn long_orders(&mut self, candle: &Candle, timestamp: String) -> Result<OrderBuilder> {
        match (
            &self.active_order.take_profit_handler.state,
            &self.active_order.stop_loss_handler.state,
        ) {
            (Some(_), Some(_)) => {
                error!("🛑 Active order exit handlers are initialized before order placement");
                Err(BinanceError::ExitHandlersInitializedEarly)
            }
            (None, None) => {
                info!(
                    "No active order, enter Long @ {} | {}",
                    candle.close,
                    candle.date.to_string()
                );

                // each order gets 1/3 of 99% of account balance
                // 99% is to account for fees
                // 1/3 is to account for 3 orders
                let long_qty = self.trade_qty(Side::Long, candle)?;
                let limit = precise_round!(candle.close, 2);
                let entry = BinanceTrade::new(
                    self.ticker.to_string(),
                    format!("{}-{}", timestamp, "ENTRY"),
                    Side::Long,
                    OrderType::Limit,
                    long_qty,
                    Some(limit),
                    None,
                    None,
                    Some(10000),
                );
                let tp_state = self
                    .active_order
                    .take_profit_handler
                    .init(candle.close, Side::Long)?;
                let take_profit = BinanceTrade::new(
                    self.ticker.to_string(),
                    format!("{}-{}", timestamp, "TAKE_PROFIT"),
                    Side::Short,
                    OrderType::TakeProfitLimit,
                    long_qty,
                    Some(tp_state.exit),
                    Some(tp_state.exit_trigger),
                    None,
                    Some(10000),
                );
                let sl_state = self
                    .active_order
                    .stop_loss_handler
                    .init(candle.close, Side::Long)?;
                let stop_loss = BinanceTrade::new(
                    self.ticker.to_string(),
                    format!("{}-{}", timestamp, "STOP_LOSS"),
                    Side::Short,
                    OrderType::StopLossLimit,
                    long_qty,
                    Some(sl_state.exit),
                    Some(sl_state.exit_trigger),
                    None,
                    Some(10000),
                );
                Ok(OrderBuilder {
                    entry,
                    take_profit,
                    stop_loss,
                })
            }
            _ => Err(BinanceError::ExitHandlersNotBothInitialized),
        }
    }

    fn short_orders(&mut self, candle: &Candle, timestamp: String) -> Result<OrderBuilder> {
        match (
            &self.active_order.take_profit_handler.state,
            &self.active_order.stop_loss_handler.state,
        ) {
            (Some(_), Some(_)) => {
                error!("🛑 Active order exit handlers are initialized before order placement");
                Err(BinanceError::ExitHandlersInitializedEarly)
            }
            (None, None) => {
                let short_qty = self.trade_qty(Side::Short, candle)?;
                let limit = precise_round!(candle.close, 2);
                let entry = BinanceTrade::new(
                    self.ticker.to_string(),
                    format!("{}-{}", timestamp, "ENTRY"),
                    Side::Short,
                    OrderType::Limit,
                    short_qty,
                    Some(limit),
                    None,
                    None,
                    Some(10000),
                );
                let tp_state = self
                    .active_order
                    .take_profit_handler
                    .init(candle.close, Side::Long)?;
                let take_profit = BinanceTrade::new(
                    self.ticker.to_string(),
                    format!("{}-{}", timestamp, "TAKE_PROFIT"),
                    Side::Long,
                    OrderType::TakeProfitLimit,
                    short_qty,
                    Some(tp_state.exit),
                    Some(tp_state.exit_trigger),
                    None,
                    Some(10000),
                );
                let sl_state = self
                    .active_order
                    .stop_loss_handler
                    .init(candle.close, Side::Long)?;
                let stop_loss = BinanceTrade::new(
                    self.ticker.to_string(),
                    format!("{}-{}", timestamp, "STOP_LOSS"),
                    Side::Long,
                    OrderType::StopLossLimit,
                    short_qty,
                    Some(sl_state.exit),
                    Some(sl_state.exit_trigger),
                    None,
                    Some(10000),
                );
                Ok(OrderBuilder {
                    entry,
                    take_profit,
                    stop_loss,
                })
            }
            _ => Err(BinanceError::ExitHandlersNotBothInitialized),
        }
    }

    pub fn handle_signal(&mut self, candle: &Candle, timestamp: String, side: Side) -> Result<()> {
        let order_builder = match side {
            Side::Long => self.long_orders(candle, timestamp)?,
            Side::Short => self.short_orders(candle, timestamp)?,
        };
        self.active_order.add_entry(order_builder.entry.clone());
        self.active_order
            .add_exits(order_builder.take_profit, order_builder.stop_loss);
        self.log_active_order();
        self.trade_or_reset::<LimitOrderResponse>(order_builder.entry)?;
        Ok(())
    }

    pub fn process_candle(&mut self, prev_candle: &Candle, candle: &Candle) -> Result<()> {
        let timestamp = candle.date.to_unix_ms().to_string();
        if self.active_order.entry.is_none() {
            let plpl = self.plpl_system.closest_plpl(candle)?;
            if self.plpl_system.long_signal(prev_candle, candle, plpl) {
                // if position is None, enter Long
                // else ignore signal and let active trade play out
                self.handle_signal(candle, timestamp, Side::Long)?;
            } else if self.plpl_system.short_signal(prev_candle, candle, plpl) {
                // if position is None, enter Short
                // else ignore signal and let active trade play out
                self.handle_signal(candle, timestamp, Side::Short)?;
            }
        }
        Ok(())
    }

    pub fn reset_active_order(&mut self) -> Result<Vec<OrderCanceled>> {
        self.active_order.reset();
        self.cancel_all_open_orders()
    }

    /// Get account info which includes token balances
    pub fn account_info(&self) -> Result<AccountInfoResponse> {
        let builder = AccountInfo::request(None);
        let req = builder.request;
        let pre = SystemTime::now();
        let res = self
            .client
            .get_signed::<AccountInfoResponse>(API::Spot(Spot::Account), Some(req));
        let dur = SystemTime::now().duration_since(pre).unwrap().as_millis();
        info!("Request time: {:?}ms", dur);
        if let Err(e) = res {
            let now = AccountInfo::get_timestamp()?;
            let req_time = builder
                .btree
                .get("timestamp")
                .unwrap()
                .parse::<u64>()
                .unwrap();
            // difference between now and req_time
            let diff = now - req_time;
            error!("🛑 Failed to get account info in {}ms: {:?}", diff, e);
            return Err(e);
        }
        res
    }

    pub fn update_assets(&mut self) -> Result<()> {
        let account_info = self.account_info()?;
        self.assets = account_info.account_assets(&self.quote_asset, &self.base_asset)?;
        Ok(())
    }

    /// Get all assets
    /// Not available on testnet
    #[allow(dead_code)]
    pub fn all_assets(&self) -> Result<Vec<CoinInfo>> {
        let req = AllAssets::request(Some(5000));
        self.client
            .get_signed::<Vec<CoinInfo>>(API::Savings(Sapi::AllCoins), Some(req))
    }

    /// Get price of a single symbol
    pub fn price(&self) -> Result<f64> {
        let req = Price::request(self.ticker.to_string());
        let res = self
            .client
            .get::<PriceResponse>(API::Spot(Spot::Price), Some(req))?;
        res.price.parse::<f64>().map_err(BinanceError::ParseFloat)
    }

    /// Get historical orders for a single symbol
    #[allow(dead_code)]
    pub fn all_orders(&self, symbol: String) -> Result<Vec<HistoricalOrder>> {
        let req = AllOrders::request(symbol, Some(5000));
        let mut orders = self
            .client
            .get_signed::<Vec<HistoricalOrder>>(API::Spot(Spot::AllOrders), Some(req))?;
        // order by time
        orders.sort_by(|a, b| a.update_time.partial_cmp(&b.update_time).unwrap());
        Ok(orders)
    }

    /// Get last open trade for a single symbol
    /// Returns Some if there is an open trade, None otherwise
    #[allow(dead_code)]
    pub fn open_orders(&self, symbol: String) -> Result<Vec<HistoricalOrder>> {
        let req = AllOrders::request(symbol, Some(5000));
        let orders = self
            .client
            .get_signed::<Vec<HistoricalOrder>>(API::Spot(Spot::AllOrders), Some(req))?;
        // filter out orders that are not filled or canceled
        let open_orders = orders
            .into_iter()
            .filter(|order| order.status == "NEW")
            .collect::<Vec<HistoricalOrder>>();
        Ok(open_orders)
    }

    /// Cancel all open orders for a single symbol
    pub fn cancel_all_open_orders(&self) -> Result<Vec<OrderCanceled>> {
        info!("Canceling all active orders");
        let req = CancelOrders::request(self.ticker.clone(), Some(10000));
        let res = self
            .client
            .delete_signed::<Vec<OrderCanceled>>(API::Spot(Spot::OpenOrders), Some(req));
        if let Err(e) = &res {
            if let BinanceError::Binance(err) = &e {
                return if err.code != -2011 {
                    error!("🛑 Failed to cancel all active orders: {:?}", e);
                    Err(BinanceError::Binance(err.clone()))
                } else {
                    debug!("No open orders to cancel");
                    Ok(vec![])
                };
            }
        }
        res
    }

    pub fn cancel_order(&self, order_id: u64) -> Result<OrderCanceled> {
        debug!("Canceling order {}", order_id);
        let req = CancelOrder::request(order_id, self.ticker.to_string(), Some(10000));
        let res = self
            .client
            .delete_signed::<OrderCanceled>(API::Spot(Spot::Order), Some(req));
        if let Err(e) = &res {
            if let BinanceError::Binance(err) = &e {
                if err.code != -2011 {
                    error!("🛑 Failed to cancel order: {:?}", e);
                    return Err(BinanceError::Binance(err.clone()));
                } else {
                    debug!("No order to cancel");
                }
            }
        }
        res
    }

    pub fn update_active_order(&mut self, event: OrderTradeEvent) -> Result<()> {
        let id = ActiveOrder::client_order_id_suffix(&event.new_client_order_id);
        match &*id {
            "ENTRY" => {
                self.active_order.entry = Some(PendingOrActiveOrder::Active(
                    TradeInfo::from_order_trade_event(&event)?,
                ));
            }
            "TAKE_PROFIT" => {
                self.active_order.take_profit = Some(PendingOrActiveOrder::Active(
                    TradeInfo::from_order_trade_event(&event)?,
                ));
            }
            "STOP_LOSS" => {
                self.active_order.stop_loss = Some(PendingOrActiveOrder::Active(
                    TradeInfo::from_order_trade_event(&event)?,
                ));
            }
            _ => debug!("Unknown order id: {}", id),
        }
        self.log_active_order();
        Ok(())
    }

    fn take_profit_pnl(&self, entry: &TradeInfo, take_profit: &TradeInfo) -> Result<f64> {
        let pnl = precise_round!(
            match entry.side {
                Side::Long => {
                    (take_profit.price - entry.price) / entry.price * 100_f64
                }
                Side::Short => {
                    (entry.price - take_profit.price) / entry.price * 100_f64
                }
            },
            5
        );
        Ok(pnl)
    }

    fn stop_loss_pnl(&self, entry: &TradeInfo, stop_loss: &TradeInfo) -> Result<f64> {
        let pnl = precise_round!(
            match entry.side {
                Side::Long => {
                    (stop_loss.price - entry.price) / entry.price * 100_f64
                }
                Side::Short => {
                    (entry.price - stop_loss.price) / entry.price * 100_f64
                }
            },
            5
        );
        Ok(pnl)
    }

    pub fn check_active_order(&mut self) -> Result<()> {
        let copy = self.active_order.clone();
        if let (Some(entry), Some(take_profit), Some(stop_loss)) =
            (&copy.entry, &copy.take_profit, &copy.stop_loss)
        {
            match (entry, take_profit, stop_loss) {
                (
                    PendingOrActiveOrder::Active(entry),
                    PendingOrActiveOrder::Pending(tp),
                    PendingOrActiveOrder::Pending(sl),
                ) => {
                    // do nothing, order is active
                    if entry.status == OrderStatus::Filled {
                        self.trade_or_reset::<LimitOrderResponse>(tp.clone())?;
                        self.trade_or_reset::<LimitOrderResponse>(sl.clone())?;
                    }
                }
                (
                    PendingOrActiveOrder::Active(entry),
                    PendingOrActiveOrder::Active(tp),
                    PendingOrActiveOrder::Active(sl),
                ) => {
                    if tp.status == OrderStatus::Filled && sl.status != OrderStatus::Filled {
                        self.cancel_all_open_orders()?;
                        info!("✅ Take profit filled, canceled stop loss");
                        let pnl = self.take_profit_pnl(&entry, &tp)?;
                        info!("📈 PNL: {}%", pnl);
                        self.active_order.reset();
                    }
                    if sl.status == OrderStatus::Filled && tp.status != OrderStatus::Filled {
                        self.cancel_all_open_orders()?;
                        info!("❌ Stop loss filled, canceled take profit");
                        let pnl = self.stop_loss_pnl(&entry, &sl)?;
                        info!("📈 PNL: {}%", pnl);
                        self.active_order.reset();
                    }
                    if sl.status == OrderStatus::Filled && tp.status == OrderStatus::Filled {
                        self.cancel_all_open_orders()?;
                        self.active_order.reset();
                        error!(
                            "Take profit and stop loss both filled: {}",
                            entry.client_order_id
                        );
                    }
                }
                _ => debug!("Unknown active order state"),
            }
        }
        Ok(())
    }

    pub fn log_active_order(&self) {
        let take_profit_status = match &self.active_order.take_profit {
            None => "None".to_string(),
            Some(option) => match option {
                PendingOrActiveOrder::Active(take_profit) => {
                    format!("{:?}", take_profit.status)
                }
                PendingOrActiveOrder::Pending(_) => "Pending".to_string(),
            },
        };
        let tp_price = match &self.active_order.take_profit_handler.state {
            None => "None".to_string(),
            Some(state) => state.exit.to_string(),
        };
        let stop_loss_status = match &self.active_order.stop_loss {
            None => "None".to_string(),
            Some(option) => match option {
                PendingOrActiveOrder::Active(stop_loss) => {
                    format!("{:?}", stop_loss.status)
                }
                PendingOrActiveOrder::Pending(_) => "Pending".to_string(),
            },
        };
        let sl_price = match &self.active_order.stop_loss_handler.state {
            None => "None".to_string(),
            Some(state) => state.exit.to_string(),
        };
        let entry_status = match &self.active_order.entry {
            None => "None".to_string(),
            Some(option) => match option {
                PendingOrActiveOrder::Active(entry) => {
                    format!("{:?}", entry.status)
                }
                PendingOrActiveOrder::Pending(_) => "Pending".to_string(),
            },
        };
        let entry_price = match &self.active_order.entry {
            None => "None".to_string(),
            Some(option) => match option {
                PendingOrActiveOrder::Active(entry) => {
                    format!("{:?}", entry.price)
                }
                PendingOrActiveOrder::Pending(_) => "Pending".to_string(),
            },
        };
        let entry_side = match &self.active_order.entry {
            None => "None".to_string(),
            Some(option) => match option {
                PendingOrActiveOrder::Active(entry) => {
                    format!("{:?}", entry.side)
                }
                PendingOrActiveOrder::Pending(_) => "Pending".to_string(),
            },
        };
        let entry_id = match &self.active_order.entry {
            None => "None".to_string(),
            Some(option) => match option {
                PendingOrActiveOrder::Active(entry) => {
                    format!("{:?}", entry.side)
                }
                PendingOrActiveOrder::Pending(order) => {
                    ActiveOrder::client_order_id_prefix(&order.client_order_id)
                }
            },
        };
        info!(
            "Active Order, {}, {}, Entry: {} @ {}, TP: {} @ {}, SL: {} @ {}",
            entry_id,
            entry_side,
            entry_price,
            entry_status,
            take_profit_status,
            tp_price,
            stop_loss_status,
            sl_price
        );
    }

    pub fn equalize_assets(&self) -> Result<()> {
        info!("Equalizing assets");
        let account_info = self.account_info()?;
        let assets = account_info.account_assets(&self.quote_asset, &self.base_asset)?;
        let price = self.price()?;

        // USDT
        let quote_balance = assets.free_quote / price;
        // BTC
        let base_balance = assets.free_base;

        let sum = quote_balance + base_balance;
        let equal = precise_round!(sum / 2_f64, 5);
        let quote_diff = precise_round!(quote_balance - equal, 5);
        let base_diff = precise_round!(base_balance - equal, 5);
        let min_notional = 0.001;

        // buy BTC
        if quote_diff > 0_f64 && quote_diff > min_notional {
            let timestamp = BinanceTrade::get_timestamp()?;
            let client_order_id = format!("{}-{}", timestamp, "EQUALIZE_QUOTE");
            let long_qty = precise_round!(quote_diff, 5);
            info!(
                "Quote asset too high = {} {}, 50/50 = {} {}, buy base asset = {} {}",
                quote_balance * price,
                self.quote_asset,
                equal * price,
                self.quote_asset,
                long_qty,
                self.base_asset
            );
            let buy_base = BinanceTrade::new(
                self.ticker.to_string(),
                client_order_id,
                Side::Long,
                OrderType::Limit,
                long_qty,
                Some(price),
                None,
                None,
                None,
            );
            if let Err(e) = self.trade::<LimitOrderResponse>(buy_base) {
                error!("🛑 Error equalizing quote asset with error: {:?}", e);
                return Err(e);
            }
        }

        // sell BTC
        if base_diff > 0_f64 && base_diff > min_notional {
            let timestamp = BinanceTrade::get_timestamp()?;
            let client_order_id = format!("{}-{}", timestamp, "EQUALIZE_BASE");
            let short_qty = precise_round!(base_diff, 5);
            info!(
                "Base asset too high = {} {}, 50/50 = {} {}, sell base asset = {} {}",
                base_balance, self.base_asset, equal, self.base_asset, short_qty, self.base_asset
            );
            let sell_base = BinanceTrade::new(
                self.ticker.to_string(),
                client_order_id,
                Side::Short,
                OrderType::Limit,
                short_qty,
                Some(price),
                None,
                None,
                None,
            );
            if let Err(e) = self.trade::<LimitOrderResponse>(sell_base) {
                error!("🛑 Error equalizing base asset with error: {:?}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    pub fn assets(&self) -> Assets {
        self.assets.clone()
    }

    pub fn log_assets(&self) {
        let assets = &self.assets;
        info!(
            "Account Assets  |  {}, Free: {}, Locked: {}  |  {}, Free: {}, Locked: {}",
            self.quote_asset,
            assets.free_quote,
            assets.locked_quote,
            self.base_asset,
            assets.free_base,
            assets.locked_base
        );
    }

    pub fn check_trailing_take_profit(&mut self) -> Result<ActiveOrder> {
        let copy = self.active_order.clone();
        if let (Some(tp_state), Some(candle)) = (&copy.take_profit_handler.state, &self.candle) {
            let update_action_info = &self
                .active_order
                .take_profit_handler
                .check(tp_state.exit_side.clone(), candle)?;
            match update_action_info.action {
                UpdateAction::None => debug!("Take profit checked, no update"),
                UpdateAction::CancelAndUpdate => {
                    // cancel take profit order and place new one
                    match &self.active_order.take_profit {
                        None => error!("No take profit order to cancel and update"),
                        Some(take_profit) => {
                            match take_profit {
                                PendingOrActiveOrder::Active(tp) => {
                                    // cancel existing trailing take profit order
                                    let res = self.cancel_order(tp.order_id)?;
                                    let orig_client_order_id =
                                        res.orig_client_order_id.ok_or(BinanceError::Custom(
                                            "OrderCanceled orig client order id is none"
                                                .to_string(),
                                        ))?;
                                    info!(
                                        "Cancel and update take profit: {:?}",
                                        orig_client_order_id
                                    );
                                    // place new take profit order with updated trigger price
                                    let exit_side = tp_state.exit_side.clone();

                                    info!(
                                        "Old take profit price: {}, new price: {}",
                                        tp.price, update_action_info.exit
                                    );
                                    let old_exit = tp.price;
                                    let new_exit = update_action_info.exit;
                                    if old_exit != new_exit {
                                        let trade = BinanceTrade::new(
                                            res.symbol,
                                            orig_client_order_id,
                                            exit_side,
                                            OrderType::TakeProfitLimit,
                                            tp.quantity,
                                            Some(update_action_info.exit),
                                            Some(update_action_info.exit_trigger),
                                            None,
                                            Some(10000),
                                        );
                                        self.trade_or_reset::<LimitOrderResponse>(trade)?;
                                    } else {
                                        debug!("Take profit price is the same, no update");
                                    }
                                }
                                PendingOrActiveOrder::Pending(_) => {
                                    debug!(
                                        "Take profit order is pending, ignore cancel and update"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Ok(self.active_order.clone())
        } else {
            error!("No take profit state to check");
            Ok(self.active_order.clone())
        }
    }
}
