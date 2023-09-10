use crate::Side;
use log::*;
use time_series::*;

#[derive(Debug, Clone)]
pub enum ExitType {
    /// Bip (1/100th of a percent). 1 bip = 0.01%
    Bips(u32),
    /// Tick (smallest unit of price change). For BTCUSD this is $0.01
    Ticks(u32),
}

impl ExitType {
    /// Tick is $0.01 * 100, so 350 pips = $3.50
    /// ticks / entry * 100 = % of price
    /// bip = 1/100th of a percent, so multiply by 100 again
    pub fn ticks_to_bips(ticks: u32, origin: f64) -> u32 {
        ((ticks as f64 / 100.0) / origin * 10_000.0).ceil() as u32
    }

    pub fn calc_exit(exit_side: Side, method: ExitType, origin: f64) -> f64 {
        match exit_side {
            Side::Short => match method {
                ExitType::Bips(bips) => {
                    precise_round!(origin - (origin * bips as f64 / 10_000.0), 2)
                }
                ExitType::Ticks(ticks) => precise_round!(origin - ticks as f64 / 100.0, 2),
            },
            Side::Long => match method {
                ExitType::Bips(bips) => {
                    precise_round!(origin + (origin * bips as f64 / 10_000.0), 2)
                }
                ExitType::Ticks(ticks) => precise_round!(origin + ticks as f64 / 100.0, 2),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum UpdateAction {
    None,
    CancelAndUpdate,
}

#[derive(Debug, Clone)]
pub struct UpdateActionInfo {
    pub action: UpdateAction,
    pub exit_trigger: f64,
    pub exit: f64,
}

#[derive(Debug, Clone)]
pub struct TrailingTakeProfitTracker {
    pub entry: f64,
    pub method: ExitType,
    // exit side is opposite entry side
    pub exit_side: Side,
    /// Price extreme from which exit is calculated as bips/ticks back towards entry
    pub exit_trigger: f64,
    /// Calculated exit trailing bips below exit trigger
    pub exit: f64,
}

impl TrailingTakeProfitTracker {
    pub fn new(entry: f64, method: ExitType, exit_side: Side) -> Self {
        match exit_side {
            // exit is Short, so entry is Long
            // therefore take profit is above entry price
            Side::Short => match method {
                ExitType::Bips(bips) => {
                    // bips away from entry
                    let exit_trigger =
                        precise_round!(entry + (entry * (bips as f64 * 2.0) / 100.0), 2);
                    let exit = ExitType::calc_exit(exit_side.clone(), method.clone(), exit_trigger);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        exit,
                    }
                }
                ExitType::Ticks(ticks) => {
                    let exit_trigger = precise_round!(entry + (ticks as f64 * 2.0) / 100.0, 2);
                    // Tick is $0.01 * 100, so 350 pips = $3.50
                    // ticks / entry * 100 = % of price
                    // bip = 1/100th of a percent, so multiply by 100 again
                    let exit = ExitType::calc_exit(exit_side.clone(), method.clone(), exit_trigger);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        exit,
                    }
                }
            },
            // exit is Long, so entry is Short
            // therefore take profit is below entry
            Side::Long => match method {
                ExitType::Bips(bips) => {
                    let exit_trigger =
                        precise_round!(entry - (entry * (bips as f64 * 2.0) / 100.0), 2);
                    let exit = ExitType::calc_exit(exit_side.clone(), method.clone(), exit_trigger);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        exit,
                    }
                }
                ExitType::Ticks(ticks) => {
                    let exit_trigger = precise_round!(entry - (ticks as f64 * 2.0) / 100.0, 2);
                    let exit = ExitType::calc_exit(exit_side.clone(), method.clone(), exit_trigger);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        exit,
                    }
                }
            },
        }
    }

    #[allow(clippy::needless_return)]
    /// Returns true if trailing stop was triggered to exit trade, false otherwise
    pub fn check(&mut self, candle: &Candle) -> UpdateActionInfo {
        let action = match self.exit_side {
            // exit is Short, so entry is Long
            // therefore take profit is above entry
            // and new candle highs increment take profit further above entry
            Side::Short => {
                if candle.high > self.exit_trigger {
                    let old_exit_trigger = self.exit_trigger;
                    let new_exit_trigger = candle.high;
                    let old_exit = self.exit;
                    let new_exit = ExitType::calc_exit(
                        self.exit_side.clone(),
                        self.method.clone(),
                        candle.high,
                    );
                    debug!(
                        "Pre-Update TP exit trigger, Old: {}, New: {}",
                        old_exit_trigger, new_exit_trigger
                    );
                    debug!("Pre-Update TP exit, Old: {}, New: {}", old_exit, new_exit);
                    self.exit_trigger = new_exit_trigger;
                    self.exit = new_exit;
                    debug!(
                        "Post-Update TP exit trigger, Old: {}, New: {}",
                        old_exit_trigger, self.exit_trigger
                    );
                    debug!("Post-Update TP exit, Old: {}, New: {}", old_exit, self.exit);
                    UpdateAction::CancelAndUpdate
                } else {
                    UpdateAction::None
                }
            }
            // exit is Long, so entry is Short
            // therefore take profit is below entry
            // and new candle lows decrement take profit further below entry
            Side::Long => {
                if candle.low < self.exit_trigger {
                    let old_exit_trigger = self.exit_trigger;
                    let new_exit_trigger = candle.low;
                    let old_exit = self.exit;
                    let new_exit = ExitType::calc_exit(
                        self.exit_side.clone(),
                        self.method.clone(),
                        candle.low,
                    );
                    debug!(
                        "Pre-Update TP exit trigger, Old: {}, New: {}",
                        old_exit_trigger, new_exit_trigger
                    );
                    debug!("Pre-Update TP exit, Old: {}, New: {}", old_exit, new_exit);
                    self.exit_trigger = new_exit_trigger;
                    self.exit = new_exit;
                    debug!(
                        "Post-Update TP exit trigger, Old: {}, New: {}",
                        old_exit_trigger, self.exit_trigger
                    );
                    debug!("Post-Update TP exit, Old: {}, New: {}", old_exit, self.exit);
                    UpdateAction::CancelAndUpdate
                } else {
                    UpdateAction::None
                }
            }
        };
        UpdateActionInfo {
            action,
            exit_trigger: self.exit_trigger,
            exit: self.exit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StopLossTracker {
    pub entry: f64,
    pub method: ExitType,
    pub exit_side: Side,
    pub exit_trigger: f64,
    pub exit: f64,
}

impl StopLossTracker {
    pub fn new(entry: f64, method: ExitType, exit_side: Side) -> StopLossTracker {
        match exit_side {
            // exit is Short, so entry is Long
            // therefore stop loss is below entry
            Side::Short => {
                let exit = ExitType::calc_exit(exit_side.clone(), method.clone(), entry);
                let exit_trigger = precise_round!(exit + ((exit - entry).abs() / 4.0), 2);
                StopLossTracker {
                    entry,
                    method,
                    exit_side,
                    exit_trigger,
                    exit,
                }
            }
            // exit is Long, so entry is Short
            // therefore stop loss is above entry
            Side::Long => {
                let exit = ExitType::calc_exit(exit_side.clone(), method.clone(), entry);
                let exit_trigger = precise_round!(exit - ((exit - entry).abs() / 4.0), 2);
                StopLossTracker {
                    entry,
                    method,
                    exit_side,
                    exit_trigger,
                    exit,
                }
            }
        }
    }
}
