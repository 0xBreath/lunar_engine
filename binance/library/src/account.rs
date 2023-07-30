use crate::api::*;
use crate::builder::*;
use crate::client::Client;
use crate::errors::Result;
use crate::model::*;
use log::{error, info};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct OrderBundleTrade {
    pub client_order_id: String,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub event_time: u64,
}

impl OrderBundleTrade {
    pub fn from_historical_order(historical_order: &HistoricalOrder) -> Result<Self> {
        Ok(Self {
            client_order_id: historical_order.client_order_id.clone(),
            order_type: OrderType::from_str(historical_order._type.as_str())?,
            status: OrderStatus::from_str(&historical_order.status)?,
            event_time: historical_order.update_time as u64,
        })
    }

    pub fn from_order_trade_event(order_trade_event: &OrderTradeEvent) -> Result<Self> {
        let order_type = OrderType::from_str(order_trade_event.order_type.as_str())?;
        Ok(Self {
            client_order_id: order_trade_event.new_client_order_id.clone(),
            order_type,
            status: OrderStatus::from_str(&order_trade_event.order_status)?,
            event_time: order_trade_event.event_time,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OrderBundle {
    pub id: String,
    pub timestamp: u64,
    pub side: Side,
    pub entry: Option<OrderBundleTrade>,
    pub take_profit: Option<OrderBundleTrade>,
    pub stop_loss: Option<OrderBundleTrade>,
}

impl OrderBundle {
    pub fn new(
        id: String,
        timestamp: u64,
        side: Side,
        entry: Option<OrderBundleTrade>,
        take_profit: Option<OrderBundleTrade>,
        stop_loss: Option<OrderBundleTrade>,
    ) -> Self {
        Self {
            id,
            timestamp,
            side,
            entry,
            take_profit,
            stop_loss,
        }
    }

    pub fn client_order_id_prefix(client_order_id: &str) -> String {
        client_order_id.split('-').next().unwrap().to_string()
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
    pub async fn exchange_info(&self, symbol: String) -> Result<ExchangeInformation> {
        let req = ExchangeInfo::request(symbol);
        self.client
            .get::<ExchangeInformation>(API::Spot(Spot::ExchangeInfo), Some(req))
            .await
    }

    /// Place a trade
    pub async fn trade<T: DeserializeOwned>(&mut self, trade: BinanceTrade) -> Result<T> {
        let req = trade.request();
        let res = self
            .client
            .post_signed::<T>(API::Spot(Spot::Order), req)
            .await;
        if let Err(e) = &res {
            error!("Failed to enter {}: {:?}", trade.side.fmt_binance(), e);
        }
        res
    }

    /// Get account info which includes token balances
    pub async fn account_info(&self) -> Result<AccountInfoResponse> {
        let req = AccountInfo::request(Some(5000));
        let pre = SystemTime::now();
        let res = self
            .client
            .get_signed::<AccountInfoResponse>(API::Spot(Spot::Account), Some(req))
            .await;
        let dur = SystemTime::now().duration_since(pre).unwrap().as_millis();
        info!("Request time: {:?}ms", dur);
        match res {
            Ok(res) => {
                info!("Account info: {:?}", res);
                Ok(res)
            }
            Err(e) => {
                error!("Failed to get account info: {:?}", e);
                Err(e)
            }
        }
    }

    /// Get all assets
    /// Not available on testnet
    pub async fn all_assets(&self) -> Result<Vec<CoinInfo>> {
        let req = AllAssets::request(Some(5000));
        self.client
            .get_signed::<Vec<CoinInfo>>(API::Savings(Sapi::AllCoins), Some(req))
            .await
    }

    /// Get price of a single symbol
    pub async fn get_price(&self, symbol: String) -> Result<f64> {
        let req = Price::request(symbol, Some(5000));
        let res = self
            .client
            .get::<PriceResponse>(API::Spot(Spot::Price), Some(req))
            .await?;
        let price = res.price.parse::<f64>().expect("failed to parse price");
        Ok(price)
    }

    /// Get historical orders for a single symbol
    pub async fn all_orders(&self, symbol: String) -> Result<Vec<HistoricalOrder>> {
        let req = AllOrders::request(symbol, Some(5000));
        let mut orders = self
            .client
            .get_signed::<Vec<HistoricalOrder>>(API::Spot(Spot::AllOrders), Some(req))
            .await?;
        // order by time
        orders.sort_by(|a, b| a.update_time.partial_cmp(&b.update_time).unwrap());
        Ok(orders)
    }

    /// Get last open trade for a single symbol
    /// Returns Some if there is an open trade, None otherwise
    pub async fn open_orders(&self, symbol: String) -> Result<Vec<HistoricalOrder>> {
        let req = AllOrders::request(symbol, Some(5000));
        let orders = self
            .client
            .get_signed::<Vec<HistoricalOrder>>(API::Spot(Spot::AllOrders), Some(req))
            .await?;
        // filter out orders that are not filled or canceled
        let open_orders = orders
            .into_iter()
            .filter(|order| order.status == "NEW")
            .collect::<Vec<HistoricalOrder>>();
        Ok(open_orders)
    }

    /// Cancel all open orders for a single symbol
    pub async fn cancel_all_active_orders(&self) -> Result<Vec<OrderCanceled>> {
        let req = CancelOrders::request(self.ticker.clone(), Some(5000));
        let res = self
            .client
            .delete_signed::<Vec<OrderCanceled>>(API::Spot(Spot::OpenOrders), Some(req))
            .await;
        match &res {
            Err(_) => {
                info!("No active orders to cancel")
            }
            Ok(res) => {
                let ids = res
                    .iter()
                    .map(|order| order.orig_client_order_id.clone().unwrap())
                    .collect::<Vec<String>>();
                info!("All active orders canceled {:?}", ids);
            }
        };
        res
    }

    /// If no active order exists, initialize with either entry, take profit, or stop loss, depending on event order type.
    ///
    /// If active order exists, update either entry, take profit, or stop loss, depending on event order type.
    pub async fn stream_update_active_order(
        &mut self,
        event: OrderTradeEvent,
    ) -> Result<Option<OrderBundle>> {
        match &self.active_order {
            Some(order_bundle) => {
                let active_id = &order_bundle.id;
                let event_id = OrderBundle::client_order_id_prefix(&event.new_client_order_id);
                if &event_id == active_id {
                    let mut updated_order = order_bundle.clone();
                    let order_type = OrderType::from_str(&event.order_type)?;
                    match order_type {
                        OrderType::Limit => {
                            info!("Updating active order entry");
                            updated_order.entry =
                                Some(OrderBundleTrade::from_order_trade_event(&event)?);
                        }
                        OrderType::TakeProfitLimit => {
                            info!("Updating active order take profit");
                            updated_order.take_profit =
                                Some(OrderBundleTrade::from_order_trade_event(&event)?);
                        }
                        OrderType::StopLossLimit => {
                            info!("Updating active order stop loss");
                            updated_order.stop_loss =
                                Some(OrderBundleTrade::from_order_trade_event(&event)?);
                        }
                        _ => {
                            info!("Invalid order event order type to update active order");
                        }
                    }
                    self.active_order = Some(updated_order);
                }
            }
            None => {
                info!("New active order");
                let event_id = OrderBundle::client_order_id_prefix(&event.new_client_order_id);
                let side = Side::from_str(&event.side)?;
                let order_type = OrderType::from_str(&event.order_type)?;
                let mut updated_order = None;
                match order_type {
                    OrderType::Limit => {
                        info!("Creating active order entry");
                        updated_order = Some(OrderBundle::new(
                            event_id,
                            event.event_time,
                            side,
                            Some(OrderBundleTrade::from_order_trade_event(&event)?),
                            None,
                            None,
                        ));
                    }
                    OrderType::TakeProfitLimit => {
                        info!("Creating active order take profit");
                        updated_order = Some(OrderBundle::new(
                            event_id,
                            event.event_time,
                            side,
                            None,
                            Some(OrderBundleTrade::from_order_trade_event(&event)?),
                            None,
                        ));
                    }
                    OrderType::StopLossLimit => {
                        info!("Creating active order stop loss");
                        updated_order = Some(OrderBundle::new(
                            event_id,
                            event.event_time,
                            side,
                            None,
                            None,
                            Some(OrderBundleTrade::from_order_trade_event(&event)?),
                        ));
                    }
                    _ => {
                        info!("Invalid order event order type to create active order");
                    }
                }
                self.active_order = updated_order;
            }
        }
        Ok(self.active_order.clone())
    }

    /// Find and store active trade
    pub async fn update_active_order(&mut self) -> Result<Option<OrderBundle>> {
        let orders = self.all_orders(self.ticker.clone()).await?;
        // all 3 orders in OrderBundle have the same client_order_id prefix
        // filter all orders and insert to a Vec<HistoricalOrder> by the client_order_id prefix
        // e.g. 123456789-TAKE_PROFIT_LIMIT -> prefix = 1234566789, suffix = TAKE_PROFIT_LIMIT
        let mut id_map = HashMap::new();
        for order in orders.iter() {
            let id = OrderBundle::client_order_id_prefix(&order.client_order_id);
            let entry = id_map.entry(id).or_insert_with(Vec::new);
            entry.push(order);
            // remove potential duplicates
            entry.dedup_by(|a, b| a._type == b._type);
        }

        // map each vector of orders with same ID to an OrderBundle
        let mut order_bundles = Vec::new();
        for id in id_map.keys() {
            let orders = id_map.get(id).unwrap();
            // get limit entry from orders
            let entry = orders.iter().find(|order| order._type == "LIMIT");
            // get take profit exit from orders
            let take_profit = orders
                .iter()
                .find(|order| order._type == "TAKE_PROFIT_LIMIT");
            // get stop loss exit from orders
            let stop_loss = orders.iter().find(|order| order._type == "STOP_LOSS_LIMIT");

            // if all 3 orders exists, push to valid order bundles
            if let (Some(entry), Some(take_profit), Some(stop_loss)) =
                (entry, take_profit, stop_loss)
            {
                let entry_order = OrderBundleTrade::from_historical_order(entry)?;
                let take_profit_order = OrderBundleTrade::from_historical_order(take_profit)?;
                let stop_loss_order = OrderBundleTrade::from_historical_order(stop_loss)?;
                // timestamp is latest of all 3 orders
                let timestamp = entry_order
                    .event_time
                    .max(take_profit_order.event_time)
                    .max(stop_loss_order.event_time);
                order_bundles.push(OrderBundle::new(
                    OrderBundle::client_order_id_prefix(&entry.client_order_id),
                    timestamp,
                    Side::from_str(&entry.side)?,
                    Some(entry_order),
                    Some(take_profit_order),
                    Some(stop_loss_order),
                ));
            }
        }
        order_bundles.sort_by(|a, b| a.timestamp.partial_cmp(&b.timestamp).unwrap());
        // get latest OrderBundle (hopefully are most recent trades)
        match order_bundles.last() {
            // no complete bundle found, cancel all orders to start from scratch
            None => {
                self.cancel_all_active_orders().await?;
                Ok(None)
            }
            Some(latest_bundle) => {
                let mut active_order = None;

                if let (Some(entry), Some(take_profit), Some(stop_loss)) = (
                    latest_bundle.entry.clone(),
                    latest_bundle.take_profit.clone(),
                    latest_bundle.stop_loss.clone(),
                ) {
                    match (entry.status, take_profit.status, stop_loss.status) {
                        // If enter is FILLED && take profit is FILLED && stop loss is FILLED
                        //      -> cancel open orders, log error, return None
                        (OrderStatus::Filled, OrderStatus::Filled, OrderStatus::Filled) => {
                            self.cancel_all_active_orders().await?;
                            let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                            error!(
                                "Order bundle {} orders all filled. Should never happen.",
                                id
                            );
                        }
                        // If enter is NEW && take profit is NEW && stop loss is NEW
                        //      -> cancel open orders, return None
                        (OrderStatus::New, OrderStatus::New, OrderStatus::New) => {
                            let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                            info!("Order bundle {} orders all new", id);
                            active_order = Some(latest_bundle.clone());
                        }
                        // If entry is FILLED && take profit is NEW && stop loss is FILLED
                        //      -> trade closed, cancel remaining exit order, return None
                        (OrderStatus::Filled, OrderStatus::New, OrderStatus::Filled) => {
                            self.cancel_all_active_orders().await?;
                            let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                            info!("Order bundle {} exited with stop loss", id);
                        }
                        // If entry is FILLED && take profit is FILLED && stop loss is NEW
                        //      -> trade closed, cancel remaining exit order, return None
                        (OrderStatus::Filled, OrderStatus::Filled, OrderStatus::New) => {
                            self.cancel_all_active_orders().await?;
                            let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                            info!("Order bundle {} exited with trailing take profit", id);
                        }
                        // If enter is FILLED && take profit is NEW && stop loss is NEW
                        //      -> trade is active, no nothing, return Some(active_order)
                        (OrderStatus::Filled, OrderStatus::New, OrderStatus::New) => {
                            let id = OrderBundle::client_order_id_prefix(&entry.client_order_id);
                            info!("Order bundle {} is active", id);
                            active_order = Some(latest_bundle.clone());
                        }
                        _ => {
                            error!("Invalid OrderBundle order status combination. Cancelling all orders to start from scratch.");
                            self.cancel_all_active_orders().await?;
                            return Ok(None);
                        }
                    }

                    self.active_order = active_order.clone();
                    Ok(active_order)
                } else {
                    self.cancel_all_active_orders().await?;
                    Ok(None)
                }
            }
        }
    }

    pub fn get_active_order(&self) -> Option<OrderBundle> {
        self.active_order.clone()
    }
}
