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
    pub extreme: f64,
    pub trigger: f64,
}

impl TrailingTakeProfitTracker {
    pub fn new(entry: f64, method: ExitType, exit_side: Side) -> Self {
        match exit_side {
            // exit is Long, so entry is Short
            // therefore take profit is below entry
            Side::Long => match method {
                ExitType::Percent(bips) => Self {
                    entry,
                    method,
                    exit_side,
                    extreme: precise_round!(entry - (entry * (bips * 2.0) / 100.0), 2),
                    trigger: precise_round!(entry - (entry * bips / 100.0), 2),
                },
                ExitType::Fixed(pip) => Self {
                    entry,
                    method,
                    exit_side,
                    extreme: precise_round!(entry - (pip as f64 * 2.0) / 100.0, 2),
                    trigger: precise_round!(entry - pip as f64 / 100.0, 2),
                },
            },
            // exit is Short, so entry is Long
            // therefore take profit is above entry price
            Side::Short => match method {
                ExitType::Percent(bips) => Self {
                    entry,
                    method,
                    exit_side,
                    extreme: precise_round!(entry + (entry * (bips * 2.0) / 100.0), 2),
                    trigger: precise_round!(entry + (entry * bips / 100.0), 2),
                },
                ExitType::Fixed(pip) => Self {
                    entry,
                    method,
                    exit_side,
                    extreme: precise_round!(entry + (pip as f64 * 2.0) / 100.0, 2),
                    trigger: precise_round!(entry + pip as f64 / 100.0, 2),
                },
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
                    if candle.low < self.trigger {
                        UpdateAction::Close
                    } else if candle.high > self.extreme {
                        self.extreme = candle.high;
                        self.trigger =
                            precise_round!(candle.high - (candle.high * bips / 100.0), 2);
                        UpdateAction::CancelAndUpdate
                    } else {
                        UpdateAction::None
                    }
                }
                ExitType::Fixed(pip) => {
                    if candle.low < self.trigger {
                        UpdateAction::Close
                    } else if candle.high > self.extreme {
                        self.extreme = candle.high;
                        self.trigger = precise_round!(candle.high - pip as f64 / 100.0, 2);
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
                    if candle.high > self.trigger {
                        UpdateAction::Close
                    } else if candle.low < self.extreme {
                        self.extreme = candle.low;
                        self.trigger = precise_round!(candle.low + (candle.low * bips / 100.0), 2);
                        UpdateAction::CancelAndUpdate
                    } else {
                        UpdateAction::None
                    }
                }
                ExitType::Fixed(pip) => {
                    if candle.high > self.trigger {
                        UpdateAction::Close
                    } else if candle.low < self.extreme {
                        self.extreme = candle.low;
                        self.trigger = precise_round!(candle.low + pip as f64 / 100.0, 2);
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
    pub trigger: f64,
}

impl StopLossTracker {
    pub fn new(entry: f64, method: ExitType, exit_side: Side) -> StopLossTracker {
        match exit_side {
            // exit is Short, so entry is Long
            // therefore stop loss is below entry
            Side::Short => match method {
                ExitType::Percent(bips) => StopLossTracker {
                    entry,
                    method,
                    exit_side,
                    trigger: precise_round!(entry - (entry * bips / 100.0), 2),
                },
                ExitType::Fixed(pip) => StopLossTracker {
                    entry,
                    method,
                    exit_side,
                    trigger: precise_round!(entry - pip as f64 / 100.0, 2),
                },
            },
            // exit is Long, so entry is Short
            // therefore stop loss is above entry
            Side::Long => match method {
                ExitType::Percent(bips) => StopLossTracker {
                    entry,
                    method,
                    exit_side,
                    trigger: precise_round!(entry + (entry * bips / 100.0), 2),
                },
                ExitType::Fixed(pip) => StopLossTracker {
                    entry,
                    method,
                    exit_side,
                    trigger: precise_round!(entry + pip as f64 / 100.0, 2),
                },
            },
        }
    }
}
