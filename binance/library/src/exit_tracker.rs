use crate::Side;
use time_series::*;

#[derive(Debug, Clone)]
pub enum ExitType {
    Percent(f64),
    /// Tick (smallest unit of price change). For BTCUSD this is $0.01
    Fixed(u32),
}

#[derive(Debug, Clone)]
pub enum UpdateAction {
    None,
    Close,
    CancelAndUpdate,
}

#[derive(Debug, Clone)]
pub struct TrailingTakeProfitTracker {
    pub entry: f64,
    pub method: ExitType,
    // exit side is opposite entry side
    pub exit_side: Side,
    pub exit_trigger: f64,
    pub update_exit_trigger: f64,
    pub exit: f64,
}

impl TrailingTakeProfitTracker {
    pub fn new(entry: f64, method: ExitType, exit_side: Side) -> Self {
        match exit_side {
            // exit is Short, so entry is Long
            // therefore take profit is above entry price
            Side::Short => match method {
                ExitType::Percent(bips) => {
                    // bips away from entry
                    let exit = precise_round!(entry + (entry * bips / 100.0), 2);
                    let exit_trigger = precise_round!(exit + (exit * bips / 100.0), 2);
                    let update_exit_trigger =
                        precise_round!(exit_trigger + (exit_trigger * bips / 100.0), 2);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        update_exit_trigger,
                        exit,
                    }
                }
                ExitType::Fixed(pip) => {
                    let exit = precise_round!(entry + pip as f64 / 100.0, 2);
                    let exit_trigger = precise_round!(exit + pip as f64 / 100.0, 2);
                    let update_exit_trigger = precise_round!(exit_trigger + pip as f64 / 100.0, 2);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        update_exit_trigger,
                        exit,
                    }
                }
            },
            // exit is Long, so entry is Short
            // therefore take profit is below entry
            Side::Long => match method {
                ExitType::Percent(bips) => {
                    let exit = precise_round!(entry - (entry * bips / 100.0), 2);
                    let exit_trigger = precise_round!(exit - (exit * bips / 100.0), 2);
                    let update_exit_trigger =
                        precise_round!(exit_trigger - (exit_trigger * (bips * 2.0) / 100.0), 2);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        update_exit_trigger,
                        exit,
                    }
                }
                ExitType::Fixed(pip) => {
                    let exit = precise_round!(entry - pip as f64 / 100.0, 2);
                    let exit_trigger = precise_round!(exit - pip as f64 / 100.0, 2);
                    let update_exit_trigger = precise_round!(exit_trigger - pip as f64 / 100.0, 2);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        update_exit_trigger,
                        exit,
                    }
                }
            },
        }
    }

    #[allow(clippy::needless_return)]
    /// Returns true if trailing stop was triggered to exit trade, false otherwise
    pub fn check(&mut self, candle: &Candle) -> UpdateAction {
        return match self.exit_side {
            // exit is Short, so entry is Long
            // therefore take profit is above entry
            // and new candle highs increment take profit further above entry
            Side::Short => match self.method {
                ExitType::Percent(bips) => {
                    if candle.low < self.exit {
                        UpdateAction::Close
                    } else if candle.high > self.update_exit_trigger {
                        self.update_exit_trigger =
                            precise_round!(candle.high + (candle.high * bips / 100.0), 2);
                        self.exit_trigger = candle.high;
                        self.exit = precise_round!(candle.high - (candle.high * bips / 100.0), 2);
                        UpdateAction::CancelAndUpdate
                    } else {
                        UpdateAction::None
                    }
                }
                ExitType::Fixed(pip) => {
                    if candle.low < self.exit {
                        UpdateAction::Close
                    } else if candle.high > self.update_exit_trigger {
                        self.update_exit_trigger =
                            precise_round!(candle.high + pip as f64 / 100.0, 2);
                        self.exit_trigger = candle.high;
                        self.exit = precise_round!(candle.high - pip as f64 / 100.0, 2);
                        UpdateAction::CancelAndUpdate
                    } else {
                        UpdateAction::None
                    }
                }
            },
            // exit is Long, so entry is Short
            // therefore take profit is below entry
            // and new candle lows decrement take profit further below entry
            Side::Long => match self.method {
                ExitType::Percent(bips) => {
                    if candle.high > self.exit {
                        UpdateAction::Close
                    } else if candle.low < self.update_exit_trigger {
                        self.update_exit_trigger =
                            precise_round!(candle.low - (candle.low * bips / 100.0), 2);
                        self.exit_trigger = candle.low;
                        self.exit = precise_round!(candle.low + (candle.low * bips / 100.0), 2);
                        UpdateAction::CancelAndUpdate
                    } else {
                        UpdateAction::None
                    }
                }
                ExitType::Fixed(pip) => {
                    if candle.high > self.exit {
                        UpdateAction::Close
                    } else if candle.low < self.update_exit_trigger {
                        self.update_exit_trigger =
                            precise_round!(candle.low - pip as f64 / 100.0, 2);
                        self.exit_trigger = candle.low;
                        self.exit = precise_round!(candle.low + pip as f64 / 100.0, 2);
                        UpdateAction::CancelAndUpdate
                    } else {
                        UpdateAction::None
                    }
                }
            },
        };
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
            Side::Short => match method {
                ExitType::Percent(bips) => {
                    let exit = precise_round!(entry - (entry * bips / 100.0), 2);
                    let exit_trigger = precise_round!(exit + ((exit - entry).abs() / 4.0), 2);
                    StopLossTracker {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        exit,
                    }
                }
                ExitType::Fixed(pip) => {
                    let exit = precise_round!(entry - pip as f64 / 100.0, 2);
                    let exit_trigger = precise_round!(exit + ((exit - entry).abs() / 4.0), 2);
                    StopLossTracker {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        exit,
                    }
                }
            },
            // exit is Long, so entry is Short
            // therefore stop loss is above entry
            Side::Long => match method {
                ExitType::Percent(bips) => {
                    let exit = precise_round!(entry + (entry * bips / 100.0), 2);
                    let exit_trigger = precise_round!(exit - ((exit - entry).abs() / 4.0), 2);
                    StopLossTracker {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        exit,
                    }
                }
                ExitType::Fixed(pip) => {
                    let exit = precise_round!(entry + pip as f64 / 100.0, 2);
                    let exit_trigger = precise_round!(exit - ((exit - entry).abs() / 4.0), 2);
                    StopLossTracker {
                        entry,
                        method,
                        exit_side,
                        exit,
                        exit_trigger,
                    }
                }
            },
        }
    }
}
