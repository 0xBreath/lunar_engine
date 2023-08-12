use crate::api::*;
use crate::builder::*;
use crate::client::Client;
use crate::errors::Result;
use crate::model::*;
use crate::{BinanceError, StopLossTracker, TrailingTakeProfitTracker};
use log::*;
use serde::de::DeserializeOwned;
use std::str::FromStr;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct OrderBundleTrade {
    pub client_order_id: String,
    pub order_id: u64,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub event_time: u64,
    pub quantity: f64,
    pub price: f64,
}

impl OrderBundleTrade {
    pub fn from_historical_order(historical_order: &HistoricalOrder) -> Result<Self> {
        Ok(Self {
            client_order_id: historical_order.client_order_id.clone(),
            order_id: historical_order.order_id.clone(),
            order_type: OrderType::from_str(historical_order._type.as_str())?,
            status: OrderStatus::from_str(&historical_order.status)?,
            event_time: historical_order.update_time as u64,
            quantity: historical_order
                .executed_qty
                .parse::<f64>()
                .map_err(BinanceError::ParseFloat)?,
            price: historical_order
                .price
                .parse::<f64>()
                .map_err(BinanceError::ParseFloat)?,
        })
    }

    pub fn from_order_trade_event(order_trade_event: &OrderTradeEvent) -> Result<Self> {
        let order_type = OrderType::from_str(order_trade_event.order_type.as_str())?;
        Ok(Self {
            client_order_id: order_trade_event.new_client_order_id.clone(),
            order_id: order_trade_event.order_id,
            order_type,
            status: OrderStatus::from_str(&order_trade_event.order_status)?,
            event_time: order_trade_event.event_time,
            quantity: order_trade_event
                .qty
                .parse::<f64>()
                .map_err(BinanceError::ParseFloat)?,
            price: order_trade_event
                .price
                .parse::<f64>()
                .map_err(BinanceError::ParseFloat)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OrderBundle {
    pub id: Option<String>,
    pub timestamp: Option<u64>,
    pub side: Side,
    pub entry: Option<OrderBundleTrade>,
    pub take_profit_tracker: TrailingTakeProfitTracker,
    pub take_profit: Option<OrderBundleTrade>,
    pub stop_loss_tracker: StopLossTracker,
    pub stop_loss: Option<OrderBundleTrade>,
}

impl OrderBundle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: Option<String>,
        timestamp: Option<u64>,
        side: Side,
        entry: Option<OrderBundleTrade>,
        take_profit: Option<OrderBundleTrade>,
        take_profit_tracker: TrailingTakeProfitTracker,
        stop_loss: Option<OrderBundleTrade>,
        stop_loss_tracker: StopLossTracker,
    ) -> Self {
        Self {
            id,
            timestamp,
            side,
            entry,
            take_profit,
            take_profit_tracker,
            stop_loss,
            stop_loss_tracker,
        }
    }

    pub fn client_order_id_prefix(client_order_id: &str) -> String {
        client_order_id.split('-').next().unwrap().to_string()
    }

    pub fn client_order_id_suffix(client_order_id: &str) -> String {
        client_order_id.split('-').last().unwrap().to_string()
    }

    pub fn has_all(&self) -> bool {
        self.entry.is_some() && self.take_profit.is_some() && self.stop_loss.is_some()
    }
}

#[derive(Clone)]
pub struct Account {
    pub client: Client,
    pub recv_window: u64,
    pub base_asset: String,
    pub quote_asset: String,
    pub ticker: String,
    pub active_order: Option<OrderBundle>,
}

impl Account {
    #[allow(dead_code)]
    pub fn new(
        client: Client,
        recv_window: u64,
        base_asset: String,
        quote_asset: String,
        ticker: String,
    ) -> Self {
        Self {
            client,
            recv_window,
            base_asset,
            quote_asset,
            ticker,
            active_order: None,
        }
    }

    #[allow(dead_code)]
    pub fn exchange_info(&self, symbol: String) -> Result<ExchangeInformation> {
        let req = ExchangeInfo::request(symbol);
        self.client
            .get::<ExchangeInformation>(API::Spot(Spot::ExchangeInfo), Some(req))
    }

    /// Place a trade
    pub fn trade<T: DeserializeOwned>(&mut self, trade: BinanceTrade) -> Result<T> {
        let req = trade.request();
        self.client.post_signed::<T>(API::Spot(Spot::Order), req)
    }

    /// Get account info which includes token balances
    pub fn account_info(&self) -> Result<AccountInfoResponse> {
        let req = AccountInfo::request(Some(5000));
        let pre = SystemTime::now();
        let res = self
            .client
            .get_signed::<AccountInfoResponse>(API::Spot(Spot::Account), Some(req));
        let dur = SystemTime::now().duration_since(pre).unwrap().as_millis();
        info!("Request time: {:?}ms", dur);
        if let Err(e) = res {
            error!("Failed to get account info: {:?}", e);
            return Err(e);
        }
        res
    }

    /// Get all assets
    /// Not available on testnet
    pub fn all_assets(&self) -> Result<Vec<CoinInfo>> {
        let req = AllAssets::request(Some(5000));
        self.client
            .get_signed::<Vec<CoinInfo>>(API::Savings(Sapi::AllCoins), Some(req))
    }

    /// Get price of a single symbol
    pub fn get_price(&self, symbol: String) -> Result<f64> {
        let req = Price::request(symbol, Some(5000));
        let res = self
            .client
            .get::<PriceResponse>(API::Spot(Spot::Price), Some(req))?;
        res.price
            .parse::<f64>()
            .map_err(|e| BinanceError::Custom(e.to_string()))
    }

    /// Get historical orders for a single symbol
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
    pub fn cancel_all_active_orders(&self) -> Result<Vec<OrderCanceled>> {
        let req = CancelOrders::request(self.ticker.clone(), Some(5000));
        let res = self
            .client
            .delete_signed::<Vec<OrderCanceled>>(API::Spot(Spot::OpenOrders), Some(req));
        if let Err(e) = &res {
            if let BinanceError::Binance(err) = &e {
                if err.code != -2011 {
                    error!("Failed to cancel all active orders: {:?}", e);
                    return Err(BinanceError::Binance(err.clone()));
                } else {
                    trace!("No active orders to cancel");
                }
            }
        }
        res
    }

    pub fn cancel_order(&self, order_id: u64) -> Result<OrderCanceled> {
        let req = CancelOrder::request(order_id, Some(5000));
        self.client
            .delete_signed::<OrderCanceled>(API::Spot(Spot::Order), Some(req))
    }

    /// Update active order via websocket stream of [`OrderTradeEvent`]
    ///
    /// If no active order exists, initialize with either entry, take profit, or stop loss, depending on event order type.
    ///
    /// If active order exists, update either entry, take profit, or stop loss, depending on event order type.
    pub fn stream_update_active_order(
        &mut self,
        event: OrderTradeEvent,
    ) -> Result<Option<OrderBundle>> {
        // update active order with new OrderTradeEvent
        match &self.active_order {
            // existing active order
            // if order ID matches active order ID, update one of the 3 orders based on order type
            Some(order_bundle) => {
                let active_id = &order_bundle.id;
                let event_id = OrderBundle::client_order_id_prefix(&event.new_client_order_id);
                let order_type = OrderBundle::client_order_id_suffix(&event.new_client_order_id);
                let order_status = OrderStatus::from_str(&event.order_status)?;
                let should_update = match active_id {
                    Some(active_id) => active_id == &event_id,
                    None => true,
                };
                if should_update {
                    let mut updated_order = order_bundle.clone();
                    match order_type.as_str() {
                        "ENTRY" => {
                            debug!("Updating active order entry");
                            updated_order.entry =
                                Some(OrderBundleTrade::from_order_trade_event(&event)?);
                        }
                        "TAKE_PROFIT" => {
                            debug!("Updating active order take profit");
                            if order_status == OrderStatus::Canceled {
                                info!("Take profit trigger updated, removing");
                                updated_order.take_profit = None;
                            } else {
                                updated_order.take_profit =
                                    Some(OrderBundleTrade::from_order_trade_event(&event)?);
                            }
                        }
                        "STOP_LOSS" => {
                            debug!("Updating active order stop loss");
                            updated_order.stop_loss =
                                Some(OrderBundleTrade::from_order_trade_event(&event)?);
                        }
                        _ => {
                            error!("Invalid order event order type to update active order");
                        }
                    }
                    self.active_order = Some(updated_order);
                }
            }
            // active order should be set to Some before getting order updates via websocket
            None => {
                error!("Active order should have been created on order placement!");
                return Err(BinanceError::Custom(
                    "Active order should have been created on order placement!".to_string(),
                ));
            }
        }

        // =========================================
        // if all 3 orders exist in active order
        // determine whether to cancel, update, or do nothing based on trade status of each order
        if let Some(order_bundle) = &self.active_order {
            if let (Some(entry), Some(take_profit), Some(stop_loss)) = (
                &order_bundle.entry,
                &order_bundle.take_profit,
                &order_bundle.stop_loss,
            ) {
                let mut updated_order = self.active_order.clone();
                match (&entry.status, &take_profit.status, &stop_loss.status) {
                    // If enter is FILLED && take profit is FILLED && stop loss is FILLED
                    //      -> cancel open orders, log error, return None
                    (
                        OrderStatus::Filled | OrderStatus::PartiallyFilled,
                        OrderStatus::Filled | OrderStatus::PartiallyFilled,
                        OrderStatus::Filled | OrderStatus::PartiallyFilled,
                    ) => {
                        self.cancel_all_active_orders()?;
                        let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                        error!(
                            "Order bundle {} orders all filled. Should never happen.",
                            id
                        );
                    }
                    // If enter is NEW && take profit is NEW && stop loss is NEW
                    //      -> do nothing, order is active, return Some
                    (OrderStatus::New, OrderStatus::New, OrderStatus::New) => {
                        let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                        debug!("Order bundle {} orders all new", id);
                    }
                    // If entry is FILLED && take profit is NEW && stop loss is FILLED
                    //      -> trade closed, cancel remaining exit order, return None
                    (
                        OrderStatus::Filled | OrderStatus::PartiallyFilled,
                        OrderStatus::New,
                        OrderStatus::Filled | OrderStatus::PartiallyFilled,
                    ) => {
                        self.cancel_all_active_orders()?;
                        let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                        info!("LOSS -- Order bundle {} exited with stop loss", id);
                        updated_order = None;
                    }
                    // If entry is FILLED && take profit is FILLED && stop loss is NEW
                    //      -> trade closed, cancel remaining exit order, return None
                    (
                        OrderStatus::Filled | OrderStatus::PartiallyFilled,
                        OrderStatus::Filled | OrderStatus::PartiallyFilled,
                        OrderStatus::New,
                    ) => {
                        self.cancel_all_active_orders()?;
                        let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                        info!(
                            "WIN -- Order bundle {} exited with trailing take profit",
                            id
                        );
                        updated_order = None;
                    }
                    // If enter is FILLED && take profit is NEW && stop loss is NEW
                    //      -> trade is active, no nothing, return Some(active_order)
                    (
                        OrderStatus::Filled | OrderStatus::PartiallyFilled,
                        OrderStatus::New,
                        OrderStatus::New,
                    ) => {
                        let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                        debug!("Order bundle {} is active", id);
                    }
                    _ => {
                        error!("Invalid OrderBundle order status combination. Cancelling all orders to start from scratch.");
                        self.cancel_all_active_orders()?;
                        updated_order = None;
                    }
                }
                self.active_order = updated_order;
            }
        }
        // =========================================

        // return updated active order
        Ok(self.active_order.clone())
    }

    pub fn get_active_order(&self) -> Option<OrderBundle> {
        self.active_order.clone()
    }

    pub fn log_active_order(&self) {
        match &self.active_order {
            None => info!("No active order to log"),
            Some(active_order) => {
                let take_profit = match &active_order.take_profit {
                    None => "None".to_string(),
                    Some(take_profit) => format!("{:?}", take_profit.status),
                };
                let tp_trigger = active_order.take_profit_tracker.trigger;
                let stop_loss = match &active_order.stop_loss {
                    None => "None".to_string(),
                    Some(stop_loss) => format!("{:?}", stop_loss.status),
                };
                let sl_trigger = active_order.stop_loss_tracker.trigger;
                match &active_order.entry {
                    Some(entry) => {
                        let entry_status = match &active_order.entry {
                            None => "None".to_string(),
                            Some(entry) => format!("{:?}", entry.status),
                        };
                        let entry_trigger = entry.price;
                        info!(
                            "Active Order: {:?}, {:?}, entry: {} @ {}, take_profit: {} @ {}, stop_loss: {} @ {}",
                            active_order.id,
                            active_order.side,
                            entry_status,
                            entry_trigger,
                            take_profit,
                            tp_trigger,
                            stop_loss,
                            sl_trigger
                        );
                    }
                    None => {
                        let entry_status = match &active_order.entry {
                            None => "None".to_string(),
                            Some(entry) => format!("{:?}", entry.status),
                        };
                        info!(
                            "Active Order: {:?}, {:?}, entry: {}, take_profit: {} @ {}, stop_loss: {} @ {}",
                            active_order.id,
                            active_order.side,
                            entry_status,
                            take_profit,
                            tp_trigger,
                            stop_loss,
                            sl_trigger
                        );
                    }
                }
            }
        }
    }
}
