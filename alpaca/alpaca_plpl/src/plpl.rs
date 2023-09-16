use crate::error::*;
use crate::utils::*;
use apca::api::v2::account::{Account, Get};
use apca::api::v2::order::*;
use apca::api::v2::updates::OrderUpdate;
use apca::Client;
use ephemeris::PLPLSystem;
use log::*;
use num_decimal::Num;
use time_series::{f64_to_num, precise_round, Candle};

#[derive(Debug, Clone)]
pub struct ActiveOrder {
    pub entry: Option<Order>,
    pub take_profit: Option<Order>,
    pub take_profit_handler: TakeProfitHandler,
    pub stop_loss: Option<Order>,
    pub stop_loss_handler: StopLossHandler,
}

impl ActiveOrder {
    pub fn new(take_profit_handler: TakeProfitHandler, stop_loss_handler: StopLossHandler) -> Self {
        Self {
            entry: None,
            take_profit: None,
            take_profit_handler,
            stop_loss: None,
            stop_loss_handler,
        }
    }

    pub fn add_entry(&mut self, entry: Order) {
        self.entry = Some(entry);
    }

    pub fn add_exits(&mut self, take_profit: Order, stop_loss: Order) {
        self.take_profit = Some(take_profit);
        self.stop_loss = Some(stop_loss);
    }

    pub fn reset(&mut self) {
        self.entry = None;
        self.take_profit = None;
        self.stop_loss = None;
    }
}

pub struct Engine {
    pub client: Client,
    pub ticker: String,
    pub plpl_system: PLPLSystem,
    pub trailing_take_profit: ExitType,
    pub stop_loss: ExitType,
    pub active_order: ActiveOrder,
}

impl Engine {
    pub fn new(
        client: Client,
        ticker: String,
        plpl_system: PLPLSystem,
        trailing_take_profit: ExitType,
        stop_loss: ExitType,
    ) -> Self {
        let take_profit_handler = TakeProfitHandler::new(trailing_take_profit.clone());
        let stop_loss_handler = StopLossHandler::new(stop_loss.clone());
        let active_order = ActiveOrder::new(take_profit_handler, stop_loss_handler);
        Self {
            client,
            ticker,
            plpl_system,
            trailing_take_profit,
            stop_loss,
            active_order,
        }
    }

    async fn account(&self) -> Result<Account> {
        let res = self.client.issue::<Get>(&()).await;
        debug!("Get account: {:?}", res);
        res.map_err(AlpacaError::ApcaGetAccount)
    }

    fn take_profit_pnl(&self, entry: &Option<Order>, take_profit: &Option<Order>) -> Result<f64> {
        match (entry, take_profit) {
            (Some(entry), Some(take_profit)) => {
                if let (Some(entry_price), Some(take_profit_price)) =
                    (&entry.average_fill_price, &take_profit.average_fill_price)
                {
                    let entry_price = entry_price.to_f64().ok_or(AlpacaError::NumUnwrap)?;
                    let take_profit_price =
                        take_profit_price.to_f64().ok_or(AlpacaError::NumUnwrap)?;
                    let pnl = precise_round!(
                        match entry.side {
                            Side::Buy => {
                                (take_profit_price - entry_price) / entry_price * 100_f64
                            }
                            Side::Sell => {
                                (entry_price - take_profit_price) / entry_price * 100_f64
                            }
                        },
                        5
                    );
                    Ok(pnl)
                } else {
                    Err(AlpacaError::NumUnwrap)
                }
            }
            _ => Err(AlpacaError::NumUnwrap),
        }
    }

    fn stop_loss_pnl(&self, entry: &Option<Order>, stop_loss: &Option<Order>) -> Result<f64> {
        match (entry, stop_loss) {
            (Some(entry), Some(stop_loss)) => {
                if let (Some(entry_price), Some(stop_loss_price)) =
                    (&entry.average_fill_price, &stop_loss.average_fill_price)
                {
                    let entry_price = entry_price.to_f64().ok_or(AlpacaError::NumUnwrap)?;
                    let stop_loss_price = stop_loss_price.to_f64().ok_or(AlpacaError::NumUnwrap)?;
                    let pnl = precise_round!(
                        match entry.side {
                            Side::Buy => {
                                (stop_loss_price - entry_price) / entry_price * 100_f64
                            }
                            Side::Sell => {
                                (entry_price - stop_loss_price) / entry_price * 100_f64
                            }
                        },
                        5
                    );
                    Ok(pnl)
                } else {
                    Err(AlpacaError::NumUnwrap)
                }
            }
            _ => Err(AlpacaError::NumUnwrap),
        }
    }

    pub async fn process_candle(
        &mut self,
        prev_candle: &Candle,
        candle: &Candle,
        timestamp: String,
    ) -> Result<()> {
        let plpl = self.plpl_system.closest_plpl(candle)?;
        if self.plpl_system.long_signal(prev_candle, candle, plpl) {
            info!("🟢 Long");
            info!(
                "🔔 Prev: {}, Current: {}, 🪐 PLPL: {}",
                prev_candle.close, candle.close, plpl
            );
            self.handle_signal(candle, timestamp, Side::Buy).await?;
        } else if self.plpl_system.short_signal(prev_candle, candle, plpl) {
            info!("🔴Short");
            info!(
                "🔔 Prev: {}, Current: {}, 🪐 PLPL: {}",
                prev_candle.close, candle.close, plpl
            );
            self.handle_signal(candle, timestamp, Side::Sell).await?;
        }

        Ok(())
    }

    async fn handle_signal(
        &mut self,
        candle: &Candle,
        timestamp: String,
        side: Side,
    ) -> Result<()> {
        if self.active_order.entry.is_none() {
            let account = self.account().await?;
            let cash = account.cash.to_f64().ok_or(AlpacaError::NumUnwrap)?;
            info!("Cash: {}", cash);
            let entry = self
                .create_entry_order(candle, timestamp, side, cash)
                .await?;
            self.active_order.add_entry(entry);
        }
        // if active order entry is Some, exit orders will be placed in `update_active_order`
        // as websocket trade updates come in, it checks if entry is filled before placing exit orders

        Ok(())
    }

    pub fn update_active_order(&mut self, order: OrderUpdate) -> Result<()> {
        let id = order_id_suffix(&order.order);
        match &*id {
            "ENTRY" => {
                self.active_order.entry = Some(order.order);
            }
            "TAKE_PROFIT" => {
                self.active_order.take_profit = Some(order.order);
            }
            "STOP_LOSS" => {
                self.active_order.stop_loss = Some(order.order);
            }
            _ => debug!("Unknown order id: {}", id),
        }

        Ok(())
    }

    pub async fn check_active_order(&mut self) -> Result<()> {
        let copy = self.active_order.clone();
        match (&copy.entry, &copy.take_profit, &copy.stop_loss) {
            (Some(entry), None, None) => {
                if entry.status == Status::Filled {
                    let take_profit = self.create_take_profit_order().await?;
                    let stop_loss = self.create_stop_loss_order().await?;
                    self.active_order.add_exits(take_profit, stop_loss);
                }
            }
            // OCO is done in websocket trade update handling function
            // so just validate that active order is not in a bad state
            (Some(_), Some(tp), Some(sl)) => {
                if tp.status == Status::Filled && sl.status == Status::New {
                    // take profit filled, so cancel stop loss
                    self.client.issue::<Delete>(&sl.id).await?;
                    self.active_order.reset();
                    info!("✅ Take profit filled, canceled stop loss");
                    let pnl = self.take_profit_pnl(
                        &self.active_order.entry,
                        &self.active_order.take_profit,
                    )?;
                    info!("📈 PNL: {}%", pnl);
                }
                if sl.status == Status::Filled && tp.status == Status::New {
                    // stop loss filled, so cancel take profit
                    self.client.issue::<Delete>(&tp.id).await?;
                    self.active_order.reset();
                    info!("❌ Stop loss filled, canceled take profit");
                    let pnl =
                        self.stop_loss_pnl(&self.active_order.entry, &self.active_order.stop_loss)?;
                    info!("📈 PNL: {}%", pnl);
                }
                if tp.status == Status::Filled && sl.status == Status::Filled {
                    error!("Both take profit and stop loss are filled");
                    self.active_order.reset();
                    return Err(AlpacaError::BothExitsFilled);
                }
            }
            _ => {
                return Err(AlpacaError::InvalidActiveOrder);
            }
        }

        Ok(())
    }

    /// Quantity is equal to 1/3 of account cash
    async fn create_entry_order(
        &self,
        candle: &Candle,
        timestamp: String,
        side: Side,
        cash: f64,
    ) -> Result<Order> {
        let entry = OrderReqInit {
            type_: Type::Limit,
            limit_price: Some(f64_to_num!(candle.close)),
            client_order_id: Some(format!("{}-{}", timestamp, "ENTRY")),
            time_in_force: TimeInForce::UntilCanceled,
            ..Default::default()
        }
        .init(
            &self.ticker,
            side,
            Amount::quantity(f64_to_num!(precise_round!(cash / 3.0 / candle.close, 5))),
        );
        debug!("Entry order: {:?}", entry);
        let res = self.client.issue::<Post>(&entry).await;
        debug!("Entry order response: {:?}", res);
        res.map_err(AlpacaError::ApcaPostOrder)
    }

    async fn create_take_profit_order(&self) -> Result<Order> {
        match &self.active_order.entry {
            None => {
                error!("Entry order should exist before creating take profit order");
                Err(AlpacaError::NoEntryOrder)
            }
            Some(entry) => {
                let tp_side = match entry.side {
                    Side::Buy => Side::Sell,
                    Side::Sell => Side::Buy,
                };
                let tp = OrderReqInit {
                    class: Class::Simple,
                    type_: Type::TrailingStop,
                    client_order_id: Some(format!("{}-{}", order_id_prefix(entry), "TAKE_PROFIT")),
                    time_in_force: TimeInForce::UntilCanceled,
                    ..Default::default()
                }
                .init(
                    &self.ticker,
                    tp_side,
                    Amount::quantity(entry.filled_quantity.clone()),
                );
                self.client
                    .issue::<Post>(&tp)
                    .await
                    .map_err(AlpacaError::ApcaPostOrder)
            }
        }
    }

    async fn create_stop_loss_order(&self) -> Result<Order> {
        match &self.active_order.entry {
            None => {
                error!("Entry order should exist before creating stop loss order");
                Err(AlpacaError::NoEntryOrder)
            }
            Some(entry) => {
                let sl_side = match entry.side {
                    Side::Buy => Side::Sell,
                    Side::Sell => Side::Buy,
                };
                let entry_price = entry
                    .limit_price
                    .clone()
                    .ok_or(AlpacaError::NumUnwrap)?
                    .to_f64()
                    .ok_or(AlpacaError::NumUnwrap)?;
                let (stop_price, limit_price) = self
                    .active_order
                    .stop_loss_handler
                    .build(entry_price, entry.side);
                let sl = OrderReqInit {
                    class: Class::Simple,
                    type_: Type::Limit,
                    stop_loss: Some(StopLoss::StopLimit(stop_price, limit_price)),
                    client_order_id: Some(format!("{}-{}", order_id_prefix(entry), "STOP_LOSS")),
                    time_in_force: TimeInForce::UntilCanceled,
                    ..Default::default()
                }
                .init(
                    &self.ticker,
                    sl_side,
                    Amount::quantity(entry.filled_quantity.clone()),
                );
                self.client
                    .issue::<Post>(&sl)
                    .await
                    .map_err(AlpacaError::ApcaPostOrder)
            }
        }
    }
}
