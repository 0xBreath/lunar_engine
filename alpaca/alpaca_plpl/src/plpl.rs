use crate::error::*;
use crate::utils;
use apca::data::v2::stream::Data;
use log::*;
use time_series::Candle;

pub fn handle_stream(data: Data) -> Result<()> {
    match data {
        Data::Quote(quote) => {
            debug!("quote: {:?}", quote);
            Ok(())
        }
        Data::Trade(trade) => {
            debug!("trade: {:?}", trade);
            Ok(())
        }
        Data::Bar(bar) => {
            debug!("bar: {:?}", bar);
            let candle = utils::bar_to_candle(bar);
            process_candle(candle)
        }
        _ => {
            debug!("other: {:?}", data);
            Ok(())
        }
    }
}

pub fn process_candle(candle: Candle) -> Result<()> {
    info!("{:?}", candle);

    Ok(())
}
