use std::path::PathBuf;
use time_series::*;
#[allow(unused_imports)]
use ephemeris::*;
#[allow(unused_imports)]
use csv::{Writer, WriterBuilder};
#[allow(unused_imports)]
use log::info;


pub fn ticker_dataframe(ticker_data: &TickerData, results_csv_path: &PathBuf) {
  if ticker_data.candles.is_empty() {
    return
  }
  let mut wtr = WriterBuilder::new()
    .has_headers(true)
    .from_path(results_csv_path)
    .expect("failed to create csv writer");
  wtr.write_field("date").expect("failed to write date field");
  wtr.write_field("close").expect("failed to write price field");
  wtr.write_field("open").expect("failed to write price field");
  wtr.write_field("high").expect("failed to write price field");
  wtr.write_field("low").expect("failed to write price field");
  wtr.write_record(None::<&[u8]>).expect("failed to write record");
  wtr.flush().expect("failed to flush csv writer");

  for candle in ticker_data.get_candles().iter() {
    wtr.write_record(&[
      candle.date.to_unix().to_string(),
      candle.close.to_string(),
      candle.open.to_string(),
      candle.high.to_string(),
      candle.low.to_string(),
    ]).expect("failed to write record");
    wtr.flush().expect("failed to flush");
  }
}

