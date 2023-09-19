use binance_lib::*;
use log::*;
use simplelog::{
    ColorChoice, CombinedLogger, Config as SimpleLogConfig, ConfigBuilder, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
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
            ConfigBuilder::new().set_time_format_rfc3339().build(),
            File::create(log_file)?,
        ),
    ])
    .map_err(|_| BinanceError::Custom("Failed to initialize PLPL Binance logger".to_string()))
}

pub fn is_testnet() -> Result<bool> {
    std::env::var("TESTNET")?
        .parse::<bool>()
        .map_err(BinanceError::ParseBool)
}

pub fn kline_to_candle(kline_event: &KlineEvent) -> Result<Candle> {
    let date = Time::from_unix_msec(kline_event.event_time as i64);
    Ok(Candle {
        date,
        open: kline_event.kline.open.parse::<f64>()?,
        high: kline_event.kline.high.parse::<f64>()?,
        low: kline_event.kline.low.parse::<f64>()?,
        close: kline_event.kline.close.parse::<f64>()?,
        volume: None,
    })
}

pub struct OrderBuilder {
    pub entry: BinanceTrade,
    pub take_profit: BinanceTrade,
    pub stop_loss: BinanceTrade,
}

#[derive(Debug, Clone)]
pub struct TradeInfo {
    pub client_order_id: String,
    pub order_id: u64,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub event_time: u64,
    pub quantity: f64,
    pub price: f64,
    pub side: Side,
}

impl TradeInfo {
    #[allow(dead_code)]
    pub fn from_historical_order(historical_order: &HistoricalOrder) -> Result<Self> {
        Ok(Self {
            client_order_id: historical_order.client_order_id.clone(),
            order_id: historical_order.order_id,
            order_type: OrderType::from_str(historical_order._type.as_str())?,
            status: OrderStatus::from_str(&historical_order.status)?,
            event_time: historical_order.update_time as u64,
            quantity: historical_order.executed_qty.parse::<f64>()?,
            price: historical_order.price.parse::<f64>()?,
            side: Side::from_str(&historical_order.side)?,
        })
    }

    pub fn from_order_trade_event(order_trade_event: &OrderTradeEvent) -> Result<Self> {
        let order_type = OrderType::from_str(order_trade_event.order_type.as_str())?;
        let status = OrderStatus::from_str(&order_trade_event.order_status)?;
        Ok(Self {
            client_order_id: order_trade_event.new_client_order_id.clone(),
            order_id: order_trade_event.order_id,
            order_type,
            status,
            event_time: order_trade_event.event_time,
            quantity: order_trade_event.qty.parse::<f64>()?,
            price: order_trade_event.price.parse::<f64>()?,
            side: Side::from_str(&order_trade_event.side)?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum PendingOrActiveOrder {
    Pending(BinanceTrade),
    Active(TradeInfo),
}

#[derive(Debug, Clone)]
pub struct ActiveOrder {
    pub entry: Option<PendingOrActiveOrder>,
    pub take_profit_handler: TakeProfitHandler,
    pub take_profit: Option<PendingOrActiveOrder>,
    pub stop_loss_handler: StopLossHandler,
    pub stop_loss: Option<PendingOrActiveOrder>,
}

impl ActiveOrder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(take_profit_handler: TakeProfitHandler, stop_loss_handler: StopLossHandler) -> Self {
        Self {
            entry: None,
            take_profit: None,
            take_profit_handler,
            stop_loss: None,
            stop_loss_handler,
        }
    }

    pub fn client_order_id_prefix(client_order_id: &str) -> String {
        client_order_id.split('-').next().unwrap().to_string()
    }

    pub fn client_order_id_suffix(client_order_id: &str) -> String {
        client_order_id.split('-').last().unwrap().to_string()
    }

    pub fn add_entry(&mut self, entry: BinanceTrade) {
        self.entry = Some(PendingOrActiveOrder::Pending(entry));
    }

    pub fn add_exits(&mut self, take_profit: BinanceTrade, stop_loss: BinanceTrade) {
        self.take_profit = Some(PendingOrActiveOrder::Pending(take_profit));
        self.stop_loss = Some(PendingOrActiveOrder::Pending(stop_loss));
    }

    pub fn reset(&mut self) {
        self.entry = None;
        self.take_profit = None;
        self.stop_loss = None;
        self.take_profit_handler.reset();
        self.stop_loss_handler.reset();
    }
}
