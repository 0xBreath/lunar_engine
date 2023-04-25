pub mod planet_matrix;
pub mod retrograde;
pub mod declination;
pub mod eclipses;
pub mod plpl;

pub use planet_matrix::*;
pub use retrograde::*;
pub use declination::*;
pub use eclipses::*;
pub use plpl::*;

use std::path::PathBuf;
use csv::WriterBuilder;
use crate::*;
use time_series::Time;

pub async fn print_planet_ephemeris(
  results_path: &PathBuf,
  origin: Origin,
  planet: Planet,
  data_type: DataType,
  start_time: Time,
  end_time: Time
) {
  let ephemeris = Query::query(
    origin,
    &planet,
    data_type,
    start_time,
    end_time
  ).await.expect("failed to query planet angles");

  let mut wtr = WriterBuilder::new()
    .has_headers(false)
    .from_path(results_path)
    .expect("failed to create csv writer");

  for (time, angle) in ephemeris {
    // write angle and date to file
    wtr.write_record(&[
      format!("{}", angle),
      time.to_string().to_string()
    ]).expect("failed to write record");
    wtr.flush().expect("failed to flush");

    // write angle to file
    // wtr.write_record(&[
    //   format!("{}", angle),
    //   String::new()
    // ]).expect("failed to write record");
    // wtr.flush().expect("failed to flush");

    // // write date to file
    // wtr.write_record(&[
    //   time.as_string().to_string(),
    //   String::new()
    // ]).expect("failed to write record");
    // wtr.flush().expect("failed to flush");

    // // write date as UNIX milliseconds to file
    // let unix_ms = time.to_naive_date().and_hms_opt(0, 0, 0).unwrap().timestamp_millis();
    // wtr.write_record(&[
    //   format!("{}", unix_ms),
    //   String::new()
    // ]).expect("failed to write record");
    // wtr.flush().expect("failed to flush");
  }
}
