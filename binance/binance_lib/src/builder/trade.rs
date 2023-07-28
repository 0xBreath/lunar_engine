use crate::alert::*;
use std::io::Result;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct BinanceTrade {
    /// Ticker symbol (e.g. BTCUSDC
    pub symbol: String,
    /// Side of the trade (BUY or SELL)
    pub side: Side,
    /// Type of order (LIMIT, MARKET, STOP_LOSS_LIMIT, STOP_LOSS)
    pub order_type: OrderType,
    /// Quantity in quote asset of the symbol to trade (e.g. BTCUSDC with quantity 10000 would trade 10000 USDC)
    pub quantity: f64,
    /// Price of the order (if market then None, if limit then Some)
    pub price: Option<f64>,
    /// Stop loss price
    pub stop_loss: Option<f64>,
    /// Trailing stop
    pub trailing_stop: Option<f64>,
    /// The number of milliseconds the request is valid for
    pub recv_window: Option<u32>,
}

impl BinanceTrade {
    pub fn new(
        symbol: String,
        side: Side,
        order_type: OrderType,
        quantity: f64,
        price: Option<f64>,
        stop_loss: Option<f64>,
        trailing_stop: Option<f64>,
        recv_window: Option<u32>,
    ) -> Self {
        Self {
            symbol,
            side,
            order_type,
            quantity,
            price,
            stop_loss,
            trailing_stop,
            recv_window,
        }
    }

    fn get_timestamp(&self) -> Result<u64> {
        let system_time = SystemTime::now();
        let since_epoch = system_time
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX EPOCH");
        Ok(since_epoch.as_secs() * 1000 + u64::from(since_epoch.subsec_nanos()) / 1_000_000)
    }

    fn build(&self) -> Vec<(String, String)> {
        let mut btree = Vec::<(String, String)>::new();
        btree.push(("symbol".to_string(), self.symbol.clone()));
        btree.push(("side".to_string(), self.side.fmt_binance().to_string()));
        btree.push((
            "type".to_string(),
            self.order_type.fmt_binance().to_string(),
        ));
        if self.order_type == OrderType::StopLossLimit
            || self.order_type == OrderType::Limit
            || self.order_type == OrderType::TakeProfitLimit
        {
            btree.push(("timeInForce".to_string(), "GTC".to_string()));
        }
        btree.push(("quantity".to_string(), self.quantity.to_string()));
        if let Some(price) = self.price {
            btree.push(("price".to_string(), price.to_string()));
        }
        if let Some(trailing_stop) = self.trailing_stop {
            btree.push(("trailingDelta".to_string(), trailing_stop.to_string()));
        }
        if let Some(stop_loss) = self.stop_loss {
            btree.push(("stopPrice".to_string(), stop_loss.to_string()));
        }
        let timestamp = self.get_timestamp().expect("Failed to get timestamp");
        btree.push(("timestamp".to_string(), timestamp.to_string()));
        if let Some(recv_window) = self.recv_window {
            btree.push(("recvWindow".to_string(), recv_window.to_string()));
        }
        btree
    }

    pub fn request(&self) -> String {
        let data = self.build();
        let mut request = String::new();
        for (key, value) in data.iter() {
            request.push_str(&format!("{}={}&", key, value));
        }
        request.pop();
        request
    }

    /// 1 BIP = 100 * % price change, 1% = 100 BIPS
    pub fn bips_trailing_stop(trailing_stop_pct: f64) -> f64 {
        trailing_stop_pct * 100.0
    }

    pub fn round_quantity(quantity: f64, decimals: u32) -> f64 {
        let pow = 10_u64.pow(decimals);
        (quantity * pow as f64).round() / pow as f64
    }

    pub fn round_price(quantity: f64) -> f64 {
        (quantity * 100.0).round() / 100.0
    }

    pub fn calc_stop_loss(order: Side, price: f64, stop_loss_pct: f64) -> f64 {
        match order {
            Side::Long => Self::round_price(price * (1.0 - stop_loss_pct)),
            Side::Short => Self::round_price(price * (1.0 + stop_loss_pct)),
        }
    }
}
