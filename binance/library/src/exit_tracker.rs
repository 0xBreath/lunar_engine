use crate::Side;
use time_series::*;

#[derive(Debug, Clone)]
pub enum ExitType {
    /// Bip (1/100th of a percent). 1 bip = 0.01%
    Bips(u32),
    /// Tick (smallest unit of price change). For BTCUSD this is $0.01
    Ticks(u32),
}

impl ExitType {
    pub fn ticks_to_bips(ticks: u32, origin: f64) -> u32 {
        let bips = ((ticks as f64 / 100.0) / origin * 10_000.0).ceil() as u32;
        // Binance trailing delta bips must be [10, 2000]
        if bips < 10 {
            10
        } else if bips > 2000 {
            2000
        } else {
            bips
        }
    }

    pub fn calc_exit(exit_side: Side, trail: ExitType, origin: f64) -> f64 {
        let trailing_bips = match trail {
            ExitType::Bips(bips) => bips,
            ExitType::Ticks(ticks) => ExitType::ticks_to_bips(ticks, origin),
        };
        match exit_side {
            Side::Short => precise_round!(origin - (origin * trailing_bips as f64 / 10_000.0), 2),
            Side::Long => precise_round!(origin + (origin * trailing_bips as f64 / 10_000.0), 2),
        }
    }
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
    /// Price extreme from which trailing bips begin
    pub exit_trigger: f64,
    /// If method is Ticks, convert to Bips
    pub trailing_bips: u32,
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
                        trailing_bips: bips,
                        exit,
                    }
                }
                ExitType::Ticks(ticks) => {
                    let exit_trigger = precise_round!(entry + (ticks as f64 * 2.0) / 100.0, 2);
                    // Tick is $0.01 * 100, so 350 pips = $3.50
                    // ticks / entry * 100 = % of price
                    // bip = 1/100th of a percent, so multiply by 100 again
                    let trailing_bips = ExitType::ticks_to_bips(ticks, exit_trigger);
                    let exit = ExitType::calc_exit(exit_side.clone(), method.clone(), exit_trigger);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        trailing_bips,
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
                        trailing_bips: bips,
                        exit,
                    }
                }
                ExitType::Ticks(ticks) => {
                    let exit_trigger = precise_round!(entry - (ticks as f64 * 2.0) / 100.0, 2);
                    let trailing_bips = ExitType::ticks_to_bips(ticks, exit_trigger);
                    let exit = ExitType::calc_exit(exit_side.clone(), method.clone(), exit_trigger);
                    Self {
                        entry,
                        method,
                        exit_side,
                        exit_trigger,
                        trailing_bips,
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
                ExitType::Bips(bips) => {
                    if candle.low < self.exit {
                        UpdateAction::Close
                    } else if candle.high > self.exit_trigger {
                        self.exit_trigger = candle.high;
                        self.trailing_bips = bips;
                        self.exit = ExitType::calc_exit(
                            self.exit_side.clone(),
                            self.method.clone(),
                            self.exit_trigger,
                        );
                        UpdateAction::CancelAndUpdate
                    } else {
                        UpdateAction::None
                    }
                }
                ExitType::Ticks(ticks) => {
                    if candle.low < self.exit {
                        UpdateAction::Close
                    } else if candle.high > self.exit_trigger {
                        self.exit_trigger = candle.high;
                        self.trailing_bips = ExitType::ticks_to_bips(ticks, self.exit_trigger);
                        self.exit = ExitType::calc_exit(
                            self.exit_side.clone(),
                            self.method.clone(),
                            self.exit_trigger,
                        );
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
                ExitType::Bips(bips) => {
                    if candle.high > self.exit {
                        UpdateAction::Close
                    } else if candle.low < self.exit_trigger {
                        self.exit_trigger = candle.low;
                        self.trailing_bips = bips;
                        self.exit = ExitType::calc_exit(
                            self.exit_side.clone(),
                            self.method.clone(),
                            self.exit_trigger,
                        );
                        UpdateAction::CancelAndUpdate
                    } else {
                        UpdateAction::None
                    }
                }
                ExitType::Ticks(ticks) => {
                    if candle.high > self.exit {
                        UpdateAction::Close
                    } else if candle.low < self.exit_trigger {
                        self.exit_trigger = candle.low;
                        self.trailing_bips = ExitType::ticks_to_bips(ticks, self.exit_trigger);
                        self.exit = ExitType::calc_exit(
                            self.exit_side.clone(),
                            self.method.clone(),
                            self.exit_trigger,
                        );
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
