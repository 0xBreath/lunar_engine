use crate::errors::{BinanceError, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Deserialize, Clone)]
pub struct Empty {}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerTime {
    pub server_time: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeInformation {
    pub timezone: String,
    pub server_time: u64,
    pub rate_limits: Vec<RateLimit>,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub rate_limit_type: String,
    pub interval: String,
    pub interval_num: u16,
    pub limit: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub symbol: String,
    pub status: String,
    pub base_asset: String,
    pub base_asset_precision: u64,
    pub quote_asset: String,
    pub quote_precision: u64,
    pub order_types: Vec<String>,
    pub iceberg_allowed: bool,
    pub is_spot_trading_allowed: bool,
    pub is_margin_trading_allowed: bool,
    pub filters: Vec<Filters>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "filterType")]
pub enum Filters {
    #[serde(rename = "PRICE_FILTER")]
    #[serde(rename_all = "camelCase")]
    PriceFilter {
        min_price: String,
        max_price: String,
        tick_size: String,
    },
    #[serde(rename = "PERCENT_PRICE")]
    #[serde(rename_all = "camelCase")]
    PercentPrice {
        multiplier_up: String,
        multiplier_down: String,
        avg_price_mins: Option<f64>,
    },
    #[serde(rename = "PERCENT_PRICE_BY_SIDE")]
    #[serde(rename_all = "camelCase")]
    PercentPriceBySide {
        bid_multiplier_up: String,
        bid_multiplier_down: String,
        ask_multiplier_up: String,
        ask_multiplier_down: String,
        avg_price_mins: Option<f64>,
    },
    #[serde(rename = "LOT_SIZE")]
    #[serde(rename_all = "camelCase")]
    LotSize {
        min_qty: String,
        max_qty: String,
        step_size: String,
    },
    #[serde(rename = "MIN_NOTIONAL")]
    #[serde(rename_all = "camelCase")]
    MinNotional {
        notional: Option<String>,
        min_notional: Option<String>,
        apply_to_market: Option<bool>,
        avg_price_mins: Option<f64>,
    },
    #[serde(rename = "NOTIONAL")]
    #[serde(rename_all = "camelCase")]
    Notional {
        notional: Option<String>,
        min_notional: Option<String>,
        apply_to_market: Option<bool>,
        avg_price_mins: Option<f64>,
    },
    #[serde(rename = "ICEBERG_PARTS")]
    #[serde(rename_all = "camelCase")]
    IcebergParts { limit: Option<u16> },
    #[serde(rename = "MAX_NUM_ORDERS")]
    #[serde(rename_all = "camelCase")]
    MaxNumOrders { max_num_orders: Option<u16> },
    #[serde(rename = "MAX_NUM_ALGO_ORDERS")]
    #[serde(rename_all = "camelCase")]
    MaxNumAlgoOrders { max_num_algo_orders: Option<u16> },
    #[serde(rename = "MAX_NUM_ICEBERG_ORDERS")]
    #[serde(rename_all = "camelCase")]
    MaxNumIcebergOrders { max_num_iceberg_orders: u16 },
    #[serde(rename = "MAX_POSITION")]
    #[serde(rename_all = "camelCase")]
    MaxPosition { max_position: String },
    #[serde(rename = "MARKET_LOT_SIZE")]
    #[serde(rename_all = "camelCase")]
    MarketLotSize {
        min_qty: String,
        max_qty: String,
        step_size: String,
    },
    #[serde(rename = "TRAILING_DELTA")]
    #[serde(rename_all = "camelCase")]
    TrailingData {
        min_trailing_above_delta: Option<u16>,
        max_trailing_above_delta: Option<u16>,
        min_trailing_below_delta: Option<u16>,
        max_trailing_below_delta: Option<u16>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountInformation {
    pub maker_commission: f32,
    pub taker_commission: f32,
    pub buyer_commission: f32,
    pub seller_commission: f32,
    pub can_trade: bool,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    pub balances: Vec<Balance>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64,
    pub client_order_id: String,
    #[serde(with = "string_or_float")]
    pub price: f64,
    pub orig_qty: String,
    pub executed_qty: String,
    pub cummulative_quote_qty: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub side: String,
    #[serde(with = "string_or_float")]
    pub stop_price: f64,
    pub iceberg_qty: String,
    pub time: u64,
    pub update_time: u64,
    pub is_working: bool,
    pub orig_quote_order_qty: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderCanceled {
    pub symbol: String,
    pub orig_client_order_id: Option<String>,
    pub order_id: Option<u64>,
    pub client_order_id: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum SpotFuturesTransferType {
    SpotToUsdtFutures = 1,
    UsdtFuturesToSpot = 2,
    SpotToCoinFutures = 3,
    CoinFuturesToSpot = 4,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransactionId {
    pub tran_id: u64,
}

// #[derive(Debug, Serialize, Deserialize, Clone)]
// #[serde(rename_all = "camelCase")]
// pub struct Transaction {
//   pub symbol: String,
//   pub order_id: u64,
//   pub order_list_id: Option<i64>,
//   pub client_order_id: String,
//   pub transact_time: u64,
//   #[serde(with = "string_or_float")]
//   pub price: f64,
//   #[serde(with = "string_or_float")]
//   pub orig_qty: f64,
//   #[serde(with = "string_or_float")]
//   pub executed_qty: f64,
//   #[serde(with = "string_or_float")]
//   pub cummulative_quote_qty: f64,
//   #[serde(with = "string_or_float", default = "default_stop_price")]
//   pub stop_price: f64,
//   pub status: String,
//   pub time_in_force: String,
//   #[serde(rename = "type")]
//   pub type_name: String,
//   pub side: String,
//   pub fills: Option<Vec<FillInfo>>,
// }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub client_order_id: String,
    #[serde(with = "string_or_float")]
    pub cum_qty: f64,
    #[serde(with = "string_or_float")]
    pub cum_quote: f64,
    #[serde(with = "string_or_float")]
    pub executed_qty: f64,
    pub order_id: u64,
    #[serde(with = "string_or_float")]
    pub avg_price: f64,
    #[serde(with = "string_or_float")]
    pub orig_qty: f64,
    pub reduce_only: bool,
    pub side: String,
    pub position_side: String,
    pub status: String,
    #[serde(with = "string_or_float")]
    pub stop_price: f64,
    pub close_position: bool,
    pub symbol: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub orig_type: String,
    #[serde(default)]
    #[serde(with = "string_or_float_opt")]
    pub activate_price: Option<f64>,
    #[serde(default)]
    #[serde(with = "string_or_float_opt")]
    pub price_rate: Option<f64>,
    pub update_time: u64,
    pub working_type: String,
    price_protect: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FillInfo {
    #[serde(with = "string_or_float")]
    pub price: f64,
    #[serde(with = "string_or_float")]
    pub qty: f64,
    #[serde(with = "string_or_float")]
    pub commission: f64,
    pub commission_asset: String,
    pub trade_id: Option<u64>,
}
/// Response to a test order (endpoint /api/v3/order/test).
///
/// Currently, the API responds {} on a successfull test transaction,
/// hence this struct has no fields.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TestResponse {}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderBook {
    pub last_update_id: u64,
    pub bids: Vec<Bids>,
    pub asks: Vec<Asks>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub struct Bids {
    #[serde(with = "string_or_float")]
    pub price: f64,
    #[serde(with = "string_or_float")]
    pub qty: f64,
}

impl Bids {
    #[allow(dead_code)]
    pub fn new(price: f64, qty: f64) -> Bids {
        Bids { price, qty }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Asks {
    #[serde(with = "string_or_float")]
    pub price: f64,
    #[serde(with = "string_or_float")]
    pub qty: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDataStream {
    pub listen_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Success {}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum Prices {
    AllPrices(Vec<SymbolPrice>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolPrice {
    pub symbol: String,
    #[serde(with = "string_or_float")]
    pub price: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AveragePrice {
    pub mins: u64,
    #[serde(with = "string_or_float")]
    pub price: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum BookTickers {
    AllBookTickers(Vec<Tickers>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tickers {
    pub symbol: String,
    #[serde(with = "string_or_float")]
    pub bid_price: f64,
    #[serde(with = "string_or_float")]
    pub bid_qty: f64,
    #[serde(with = "string_or_float")]
    pub ask_price: f64,
    #[serde(with = "string_or_float")]
    pub ask_qty: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistory {
    pub id: u64,
    #[serde(with = "string_or_float")]
    pub price: f64,
    #[serde(with = "string_or_float")]
    pub qty: f64,
    pub commission: String,
    pub commission_asset: String,
    pub time: u64,
    pub is_buyer: bool,
    pub is_maker: bool,
    pub is_best_match: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PriceStats {
    pub symbol: String,
    pub price_change: String,
    pub price_change_percent: String,
    pub weighted_avg_price: String,
    #[serde(with = "string_or_float")]
    pub prev_close_price: f64,
    #[serde(with = "string_or_float")]
    pub last_price: f64,
    #[serde(with = "string_or_float")]
    pub bid_price: f64,
    #[serde(with = "string_or_float")]
    pub ask_price: f64,
    #[serde(with = "string_or_float")]
    pub open_price: f64,
    #[serde(with = "string_or_float")]
    pub high_price: f64,
    #[serde(with = "string_or_float")]
    pub low_price: f64,
    #[serde(with = "string_or_float")]
    pub volume: f64,
    pub open_time: u64,
    pub close_time: u64,
    pub first_id: i64,
    pub last_id: i64,
    pub count: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AggTrade {
    #[serde(rename = "T")]
    pub time: u64,
    #[serde(rename = "a")]
    pub agg_id: u64,
    #[serde(rename = "f")]
    pub first_id: u64,
    #[serde(rename = "l")]
    pub last_id: u64,
    #[serde(rename = "m")]
    pub maker: bool,
    #[serde(rename = "M")]
    pub best_match: bool,
    #[serde(rename = "p", with = "string_or_float")]
    pub price: f64,
    #[serde(rename = "q", with = "string_or_float")]
    pub qty: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDataStreamExpiredEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdateEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "a")]
    pub data: AccountUpdateDataEvent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountUpdateDataEvent {
    #[serde(rename = "m")]
    pub reason: String,

    #[serde(rename = "B")]
    pub balances: Vec<EventBalance>,

    #[serde(rename = "P")]
    pub positions: Vec<EventPosition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventBalance {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "wb")]
    pub wallet_balance: String,
    #[serde(rename = "cw")]
    pub cross_wallet_balance: String,
    #[serde(rename = "bc")]
    pub balance_change: String, // Balance Change except PnL and Commission
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventPosition {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "pa")]
    pub position_amount: String,
    #[serde(rename = "ep")]
    pub entry_price: String,
    #[serde(rename = "cr")]
    pub accumulated_realized: String, // (Pre-fee) Accumulated Realized
    #[serde(rename = "up")]
    pub unrealized_pnl: String,
    #[serde(rename = "mt")]
    pub margin_type: String,
    #[serde(rename = "iw")]
    pub isolated_wallet: String,
    #[serde(rename = "ps")]
    pub position_side: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BalanceUpdateEvent {
    #[serde(rename = "B")]
    pub balance: Vec<EventBalance>,

    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "u")]
    pub last_account_update_time: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderTradeEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "c")]
    pub new_client_order_id: String,

    #[serde(rename = "S")]
    pub side: String,

    #[serde(rename = "o")]
    pub order_type: String,

    #[serde(rename = "f")]
    pub time_in_force: String,

    #[serde(rename = "q")]
    pub qty: String,

    #[serde(rename = "p")]
    pub price: String,

    #[serde(skip, rename = "P")]
    pub p_ignore: String,

    #[serde(skip, rename = "F")]
    pub f_ignore: String,

    #[serde(skip)]
    pub g: i32,

    #[serde(skip, rename = "C")]
    pub c_ignore: Option<String>,

    #[serde(rename = "x")]
    pub execution_type: String,

    #[serde(rename = "X")]
    pub order_status: String,

    #[serde(rename = "r")]
    pub order_reject_reason: String,

    #[serde(rename = "i")]
    pub order_id: u64,

    #[serde(rename = "l")]
    pub qty_last_filled_trade: String,

    #[serde(rename = "z")]
    pub accumulated_qty_filled_trades: String,

    #[serde(rename = "L")]
    pub price_last_filled_trade: String,

    #[serde(rename = "n")]
    pub commission: String,

    #[serde(skip, rename = "N")]
    pub asset_commisioned: Option<String>,

    #[serde(rename = "T")]
    pub trade_order_time: u64,

    #[serde(rename = "t")]
    pub trade_id: i64,

    #[serde(skip, rename = "I")]
    pub i_ignore: u64,

    #[serde(skip)]
    pub w: bool,

    #[serde(rename = "m")]
    pub is_buyer_maker: bool,

    #[serde(skip, rename = "M")]
    pub m_ignore: bool,
}

/// The Trade Streams push raw trade information; each trade has a unique buyer and seller.
///
/// Stream Name: \<symbol\>@trade
///
/// Update Speed: Real-time
///
/// <https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md#trade-streams>
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TradeEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "t")]
    pub trade_id: u64,

    #[serde(rename = "p")]
    pub price: String,

    #[serde(rename = "q")]
    pub qty: String,

    #[serde(rename = "b")]
    pub buyer_order_id: u64,

    #[serde(rename = "a")]
    pub seller_order_id: u64,

    #[serde(rename = "T")]
    pub trade_order_time: u64,

    #[serde(rename = "m")]
    pub is_buyer_maker: bool,

    #[serde(skip, rename = "M")]
    pub m_ignore: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndexPriceEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "i")]
    pub pair: String,

    #[serde(rename = "p")]
    pub price: String,
}
// https://binance-docs.github.io/apidocs/futures/en/#mark-price-stream
// https://binance-docs.github.io/apidocs/delivery/en/#mark-price-stream
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkPriceEvent {
    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "P")]
    pub estimate_settle_price: String,

    #[serde(rename = "T")]
    pub next_funding_time: u64,

    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "i")]
    pub index_price: Option<String>,

    #[serde(rename = "p")]
    pub mark_price: String,

    #[serde(rename = "r")]
    pub funding_rate: String,

    #[serde(rename = "s")]
    pub symbol: String,
}

// Object({"E": Number(1626118018407), "e": String("forceOrder"), "o": Object({"S": String("SELL"), "T": Number(1626118018404), "X": String("FILLED"), "ap": String("33028.07"), "f": String("IOC"), "l": String("0.010"), "o": String("LIMIT"), "p": String("32896.00"), "q": String("0.010"), "s": String("BTCUSDT"), "z": String("0.010")})})
// https://binance-docs.github.io/apidocs/futures/en/#liquidation-order-streams

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "o")]
    pub liquidation_order: LiquidationOrder,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationOrder {
    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "S")]
    pub side: String,

    #[serde(rename = "o")]
    pub order_type: String,

    #[serde(rename = "f")]
    pub time_in_force: String,

    #[serde(rename = "q")]
    pub original_quantity: String,

    #[serde(rename = "p")]
    pub price: String,

    #[serde(rename = "ap")]
    pub average_price: String,

    #[serde(rename = "X")]
    pub order_status: String,

    #[serde(rename = "l")]
    pub order_last_filled_quantity: String,

    #[serde(rename = "z")]
    pub order_filled_accumulated_quantity: String,

    #[serde(rename = "T")]
    pub order_trade_time: u64,
}

/// Response to the Savings API get all coins request
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CoinInfo {
    pub coin: String,
    pub deposit_all_enable: bool,
    #[serde(with = "string_or_float")]
    pub free: f64,
    #[serde(with = "string_or_float")]
    pub freeze: f64,
    #[serde(with = "string_or_float")]
    pub ipoable: f64,
    #[serde(with = "string_or_float")]
    pub ipoing: f64,
    pub is_legal_money: bool,
    #[serde(with = "string_or_float")]
    pub locked: f64,
    pub name: String,
    pub network_list: Vec<Network>,
    #[serde(with = "string_or_float")]
    pub storage: f64,
    pub trading: bool,
    pub withdraw_all_enable: bool,
    #[serde(with = "string_or_float")]
    pub withdrawing: f64,
}

/// Part of the Savings API get all coins response
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    pub address_regex: String,
    pub coin: String,
    /// shown only when "depositEnable" is false.
    pub deposit_desc: Option<String>,
    pub deposit_enable: bool,
    pub is_default: bool,
    pub memo_regex: String,
    /// min number for balance confirmation
    pub min_confirm: u32,
    pub name: String,
    pub network: String,
    pub reset_address_status: bool,
    pub special_tips: Option<String>,
    /// confirmation number for balance unlock
    pub un_lock_confirm: u32,
    /// shown only when "withdrawEnable" is false.
    pub withdraw_desc: Option<String>,
    pub withdraw_enable: bool,
    #[serde(with = "string_or_float")]
    pub withdraw_fee: f64,
    #[serde(with = "string_or_float")]
    pub withdraw_min: f64,
    // pub insert_time: Option<u64>, //commented out for now, because they are not inside the actual response (only the api doc example)
    // pub update_time: Option<u64>,
    pub withdraw_integer_multiple: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetDetail {
    #[serde(with = "string_or_float")]
    pub min_withdraw_amount: f64,
    /// false if ALL of networks' are false
    pub deposit_status: bool,
    #[serde(with = "string_or_float")]
    pub withdraw_fee: f64,
    /// false if ALL of networks' are false
    pub withdraw_status: bool,
    /// reason
    pub deposit_tip: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepositAddress {
    pub address: String,
    pub coin: String,
    pub tag: String,
    pub url: String,
}

pub(crate) mod string_or_float {
    use std::fmt;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: fmt::Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrFloat {
            String(String),
            Float(f64),
        }

        match StringOrFloat::deserialize(deserializer)? {
            StringOrFloat::String(s) => {
                if s == "INF" {
                    Ok(f64::INFINITY)
                } else {
                    s.parse().map_err(de::Error::custom)
                }
            }
            StringOrFloat::Float(i) => Ok(i),
        }
    }
}

pub(crate) mod string_or_float_opt {
    use std::fmt;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: fmt::Display,
        S: Serializer,
    {
        match value {
            Some(v) => crate::model::string_or_float::serialize(v, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrFloat {
            String(String),
            Float(f64),
        }

        Ok(Some(crate::model::string_or_float::deserialize(
            deserializer,
        )?))
    }
}

pub(crate) mod string_or_bool {
    use std::fmt;

    use serde::{de, Deserialize, Deserializer, Serializer};

    #[allow(dead_code)]
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: fmt::Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    #[allow(dead_code)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrFloat {
            String(String),
            Bool(bool),
        }

        match StringOrFloat::deserialize(deserializer)? {
            StringOrFloat::String(s) => s.parse().map_err(de::Error::custom),
            StringOrFloat::Bool(i) => Ok(i),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "k")]
    pub kline: Kline,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Kline {
    #[serde(rename = "t")]
    pub open_time: i64,

    #[serde(rename = "T")]
    pub close_time: i64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "i")]
    pub interval: String,

    #[serde(rename = "f")]
    pub first_trade_id: i64,

    #[serde(rename = "L")]
    pub last_trade_id: i64,

    #[serde(rename = "o")]
    pub open: String,

    #[serde(rename = "c")]
    pub close: String,

    #[serde(rename = "h")]
    pub high: String,

    #[serde(rename = "l")]
    pub low: String,

    #[serde(rename = "v")]
    pub volume: String,

    #[serde(rename = "n")]
    pub number_of_trades: i64,

    #[serde(rename = "x")]
    pub is_final_bar: bool,

    #[serde(rename = "q")]
    pub quote_asset_volume: String,

    #[serde(rename = "V")]
    pub taker_buy_base_asset_volume: String,

    #[serde(rename = "Q")]
    pub taker_buy_quote_asset_volume: String,

    #[serde(skip, rename = "B")]
    pub ignore_me: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Side {
    Long,
    Short,
}
impl FromStr for Side {
    type Err = BinanceError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "Long" | "BUY" => Ok(Self::Long),
            "Short" | "SELL" => Ok(Self::Short),
            _ => Err(BinanceError::SideInvalid),
        }
    }
}
impl Side {
    pub fn fmt_binance(&self) -> &str {
        match self {
            Side::Long => "BUY",
            Side::Short => "SELL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum OrderType {
    Limit,
    Market,
    StopLossLimit,
    StopLoss,
    TakeProfitLimit,
    TakeProfit,
}
impl OrderType {
    pub fn fmt_binance(&self) -> &str {
        match self {
            OrderType::Limit => "LIMIT",
            OrderType::Market => "MARKET",
            OrderType::StopLossLimit => "STOP_LOSS_LIMIT",
            OrderType::StopLoss => "STOP_LOSS",
            OrderType::TakeProfitLimit => "TAKE_PROFIT_LIMIT",
            OrderType::TakeProfit => "TAKE_PROFIT",
        }
    }
}
impl FromStr for OrderType {
    type Err = BinanceError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "LIMIT" => Ok(OrderType::Limit),
            "MARKET" => Ok(OrderType::Market),
            "STOP_LOSS_LIMIT" => Ok(OrderType::StopLossLimit),
            "STOP_LOSS" => Ok(OrderType::StopLoss),
            "TAKE_PROFIT_LIMIT" => Ok(OrderType::TakeProfitLimit),
            "TAKE_PROFIT" => Ok(OrderType::TakeProfit),
            _ => Err(BinanceError::OrderTypeInvalid),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitOrderResponse {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64,
    pub client_order_id: String,
    pub transact_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64,
    pub client_order_id: String,
    pub transact_time: u64,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    pub cummulative_quote_qty: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub type_: String,
    // BUY or SELL
    pub side: String,
    pub working_time: u64,
    pub self_trade_prevention_mode: String,
    pub fills: Vec<Fill>,
}

impl OrderResponse {
    #[allow(dead_code)]
    pub fn side(&self) -> Side {
        match self.side.as_str() {
            "BUY" => Side::Long,
            "SELL" => Side::Short,
            _ => panic!("Invalid side {}", self.side),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fill {
    pub price: String,
    pub qty: String,
    pub commission: String,
    pub commission_asset: String,
    pub trade_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfoResponse {
    pub maker_commission: u32,
    pub taker_commission: u32,
    pub buyer_commission: u32,
    pub seller_commission: u32,
    pub commission_rates: CommissionRates,
    pub can_trade: bool,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    pub brokered: bool,
    pub require_self_trade_prevention: bool,
    pub update_time: u64,
    pub account_type: String,
    pub balances: Vec<Balance>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommissionRates {
    pub maker: String,
    pub taker: String,
    pub buyer: String,
    pub seller: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceResponse {
    pub symbol: String,
    pub price: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalOrder {
    pub symbol: String,
    pub order_id: u64,
    pub order_list_id: i64, //Unless OCO, the value will always be -1
    pub client_order_id: String,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    pub cummulative_quote_qty: String,
    pub status: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub _type: String,
    pub side: String,
    pub stop_price: Option<String>,
    pub iceberg_qty: Option<String>,
    pub time: i64,
    pub update_time: i64,
    pub is_working: bool,
    pub orig_quote_order_qty: String,
    pub working_time: i64,
    pub self_trade_prevention_mode: String,
}

impl HistoricalOrder {
    pub fn side(&self) -> Side {
        match self.side.as_str() {
            "BUY" => Side::Long,
            "SELL" => Side::Short,
            _ => panic!("Invalid side {}", self.side),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    PendingCancel,
    Rejected,
    Expired,
    ExpiredInMatch,
}
impl OrderStatus {
    pub fn to_str(&self) -> &str {
        match self {
            OrderStatus::New => "NEW",
            OrderStatus::PartiallyFilled => "PARTIALLY_FILLED",
            OrderStatus::Filled => "FILLED",
            OrderStatus::Canceled => "CANCELED",
            OrderStatus::PendingCancel => "PENDING_CANCEL",
            OrderStatus::Rejected => "REJECTED",
            OrderStatus::Expired => "EXPIRED",
            OrderStatus::ExpiredInMatch => "EXPIRED_IN_MATCH",
        }
    }
}
impl FromStr for OrderStatus {
    type Err = BinanceError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "NEW" => Ok(OrderStatus::New),
            "PARTIALLY_FILLED" => Ok(OrderStatus::PartiallyFilled),
            "TRADE" | "FILLED" => Ok(OrderStatus::Filled),
            "CANCELED" => Ok(OrderStatus::Canceled),
            "PENDING_CANCEL" => Ok(OrderStatus::PendingCancel),
            "REJECTED" => Ok(OrderStatus::Rejected),
            "EXPIRED" => Ok(OrderStatus::Expired),
            "EXPIRED_IN_MATCH" => Ok(OrderStatus::ExpiredInMatch),
            _ => Err(BinanceError::OrderStatusParseError(s.to_string())),
        }
    }
}
