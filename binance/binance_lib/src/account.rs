use crate::api::*;
use crate::builder::*;
use crate::client::Client;
use crate::errors::Result;
use crate::model::ExchangeInformation;
use crate::response::{AccountInfoResponse, HistoricalOrder, OrderCanceled, PriceResponse};
use crate::CoinInfo;
use log::{debug, info};
use serde::de::DeserializeOwned;
use std::time::SystemTime;

#[derive(Clone)]
pub struct Account {
    pub client: Client,
    pub recv_window: u64,
    pub base_asset: String,
    pub quote_asset: String,
    pub ticker: String,
    pub active_order: Option<HistoricalOrder>,
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
            .await?;
        self.set_active_order().await?;
        Ok(res)
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
        res
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
        orders.sort_by(|a, b| a.working_time.partial_cmp(&b.working_time).unwrap());
        Ok(orders)
    }

    /// Get last open trade for a single symbol
    /// Returns Some if there is an open trade, None otherwise
    pub async fn last_order(&self, symbol: String) -> Result<Option<HistoricalOrder>> {
        let req = AllOrders::request(symbol, Some(5000));
        let orders = self
            .client
            .get_signed::<Vec<HistoricalOrder>>(API::Spot(Spot::AllOrders), Some(req))
            .await?;
        // filter out orders that are not open
        let open_orders = orders
            .into_iter()
            .filter(|order| order.is_working)
            .collect::<Vec<HistoricalOrder>>();
        // filter latest open order by time
        let last_order = open_orders
            .into_iter()
            .max_by(|a, b| a.working_time.cmp(&b.working_time));
        Ok(last_order)
    }

    /// Cancel all open orders for a single symbol
    pub async fn cancel_all_active_orders(&self) -> Result<Vec<OrderCanceled>> {
        let req = CancelOrders::request(self.ticker.clone(), Some(5000));
        self.client
            .delete_signed(API::Spot(Spot::OpenOrders), Some(req))
            .await
    }

    /// Set current active trade to track quantity to exit trade
    pub async fn set_active_order(&mut self) -> Result<()> {
        debug!("Setting active order");
        let last_order = self.last_order(self.ticker.clone()).await?;
        debug!("Current active order: {:?}", self.active_order);
        debug!("Last order: {:?}", last_order);
        self.active_order = last_order;
        Ok(())
    }

    /// Get current active trade
    pub fn get_active_order(&self) -> Option<HistoricalOrder> {
        self.active_order.clone()
    }
}
