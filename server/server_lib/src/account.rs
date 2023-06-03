use crate::api::*;
use crate::builder::*;
use crate::client::Client;
use crate::errors::Result;
use crate::model::ExchangeInformation;
use crate::response::{AccountInfoResponse, HistoricalOrder, OrderCanceled, PriceResponse};
use crate::CoinInfo;
use log::debug;
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct Account {
    pub client: Client,
    pub recv_window: u64,
    pub base_asset: String,
    pub quote_asset: String,
    pub ticker: String,
    pub active_order: Option<HistoricalOrder>,
    pub quote_asset_free: Option<f64>,
    pub quote_asset_locked: Option<f64>,
    pub base_asset_free: Option<f64>,
    pub base_asset_locked: Option<f64>,
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
            quote_asset_free: None,
            quote_asset_locked: None,
            base_asset_free: None,
            base_asset_locked: None,
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
        debug!("Trade Request: {:?}", req);
        let res = self.client.post_signed::<T>(API::Spot(Spot::Order), req)?;
        self.set_active_order()?;
        Ok(res)
    }

    /// Get account info which includes token balances
    pub fn account_info(&self) -> Result<AccountInfoResponse> {
        let req = AccountInfo::request();
        self.client
            .get_signed::<AccountInfoResponse>(API::Spot(Spot::Account), Some(req))
    }

    /// Get all assets
    /// Not available on testnet
    pub fn all_assets(&self) -> Result<Vec<CoinInfo>> {
        let req = AllAssets::request();
        self.client
            .get_signed::<Vec<CoinInfo>>(API::Savings(Sapi::AllCoins), Some(req))
    }

    /// Get price of a single symbol
    pub fn get_price(&self, symbol: String) -> Result<f64> {
        let req = Price::request(symbol);
        let res = self
            .client
            .get::<PriceResponse>(API::Spot(Spot::Price), Some(req))?;
        let price = res.price.parse::<f64>().expect("failed to parse price");
        Ok(price)
    }

    /// Get historical orders for a single symbol
    pub fn all_orders(&self, symbol: String) -> Result<Vec<HistoricalOrder>> {
        let req = AllOrders::request(symbol);
        self.client
            .get_signed::<Vec<HistoricalOrder>>(API::Spot(Spot::AllOrders), Some(req))
    }

    /// Get last open trade for a single symbol
    /// Returns Some if there is an open trade, None otherwise
    pub fn last_order(&self, symbol: String) -> Result<Option<HistoricalOrder>> {
        let req = AllOrders::request(symbol);
        let orders = self
            .client
            .get_signed::<Vec<HistoricalOrder>>(API::Spot(Spot::AllOrders), Some(req))?;
        // filter out orders that are not open
        let open_orders = orders
            .into_iter()
            .filter(|order| order.is_working)
            .collect::<Vec<HistoricalOrder>>();
        // filter latest open order by time
        let last_order = open_orders.into_iter().max_by(|a, b| a.time.cmp(&b.time));
        Ok(last_order)
    }

    /// Cancel all open orders for a single symbol
    pub fn cancel_all_active_orders(&self) -> Result<Vec<OrderCanceled>> {
        let req = CancelOrders::request(self.ticker.clone());
        self.client
            .delete_signed(API::Spot(Spot::OpenOrders), Some(req))
    }

    /// Set current active trade to track quantity to exit trade
    pub fn set_active_order(&mut self) -> Result<()> {
        let last_order = self.last_order(self.ticker.clone())?;
        self.active_order = last_order;
        Ok(())
    }

    /// Get current active trade
    pub fn get_active_order(&self) -> Option<HistoricalOrder> {
        self.active_order.clone()
    }
}
