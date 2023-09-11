use crate::error::*;
use ephemeris::PLPLSystem;
use log::*;
use time_series::{Candle, ExitType, Time};

#[allow(clippy::too_many_arguments)]
pub fn handle_signal(
    plpl_system: &PLPLSystem,
    plpl: f32,
    prev_candle: &Candle,
    candle: &Candle,
    date: &Time,
    timestamp: String,
    trailing_take_profit: ExitType,
    stop_loss: ExitType,
) -> Result<()> {
    if plpl_system.long_signal(prev_candle, candle, plpl) {
        info!("🟢 Long");
        info!("🔔 Prev: {}, Current: {}", prev_candle.close, candle.close);
        info!("🔔 Current: {}", candle.close);
        info!("🪐 PLPL: {}", plpl);
    } else if plpl_system.short_signal(prev_candle, candle, plpl) {
        info!("🔴Short");
        info!("🔔 Prev: {}, Current: {}", prev_candle.close, candle.close);
        info!("🪐 PLPL: {}", plpl);
    }

    Ok(())
}
