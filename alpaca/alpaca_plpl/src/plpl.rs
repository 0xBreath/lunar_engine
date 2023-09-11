use crate::error::*;
use crate::utils::{BracketStopLoss, BracketTrailingTakeProfit, ExitType};
use apca::api::v2::order::{Amount, Class, OrderReqInit, Post, Side, StopLoss, TimeInForce, Type};
use apca::Client;
use ephemeris::PLPLSystem;
use log::*;
use num_decimal::Num;
use time_series::{f64_to_num, Candle};

#[allow(clippy::too_many_arguments)]
pub async fn process_candle(
    client: &Client,
    plpl_system: &PLPLSystem,
    prev_candle: &Candle,
    candle: &Candle,
    timestamp: String,
    trailing_take_profit: ExitType,
    stop_loss: ExitType,
) -> Result<()> {
    let plpl = plpl_system.closest_plpl(&candle)?;
    if plpl_system.long_signal(prev_candle, candle, plpl) {
        info!("🟢 Long");
        info!("🔔 Prev: {}, Current: {}", prev_candle.close, candle.close);
        info!("🔔 Current: {}", candle.close);
        info!("🪐 PLPL: {}", plpl);
        handle_signal(
            client,
            candle,
            timestamp,
            trailing_take_profit,
            stop_loss,
            Side::Buy,
        )
        .await?;
    } else if plpl_system.short_signal(prev_candle, candle, plpl) {
        info!("🔴Short");
        info!("🔔 Prev: {}, Current: {}", prev_candle.close, candle.close);
        info!("🪐 PLPL: {}", plpl);
        handle_signal(
            client,
            candle,
            timestamp,
            trailing_take_profit,
            stop_loss,
            Side::Sell,
        )
        .await?;
    }

    Ok(())
}

async fn handle_signal(
    client: &Client,
    candle: &Candle,
    timestamp: String,
    trailing_take_profit: ExitType,
    stop_loss: ExitType,
    side: Side,
) -> Result<()> {
    let tp = BracketTrailingTakeProfit::new(trailing_take_profit);
    let sl = BracketStopLoss::new(candle.close, side, stop_loss);

    // TODO: quantity based on account balance

    let request = OrderReqInit {
        class: Class::Bracket,
        type_: Type::Limit,
        limit_price: Some(f64_to_num!(candle.close)),
        trail_price: tp.trail_price,
        trail_percent: tp.trail_percent,
        stop_loss: Some(StopLoss::StopLimit(sl.stop_price, sl.limit_price)),
        client_order_id: Some(format!("{}-{}", timestamp, "BRACKET")),
        time_in_force: TimeInForce::UntilCanceled,
        ..Default::default()
    }
    .init("BTC/USD", side, Amount::quantity(1));

    let order = client.issue::<Post>(&request).await.unwrap();
    info!("{:?}", order);

    Ok(())
}
