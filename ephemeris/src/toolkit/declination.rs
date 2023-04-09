use std::path::PathBuf;
use log::debug;
use crate::*;
use time_series::{Direction, ReversalType, TickerData, Time};


#[derive(Clone, Debug)]
pub struct EquatorCross {
  pub date: Time,
  pub declination: f32,
  pub direction: Direction,
  pub planet: Planet
}
impl EquatorCross {
  pub fn new(date: Time, declination: f32, direction: Direction, planet: Planet) -> Self {
    Self {
      date,
      declination,
      direction,
      planet
    }
  }
}

#[derive(Debug, Clone)]
pub struct PlanetEquatorCrosses {
  /// Vector of dates of planet declinations with precise declination as f32.
  pub equator_crosses: Vec<EquatorCross>
}

impl PlanetEquatorCrosses {
  pub async fn new(start_time: Time, stop_time: Time) -> Self {
    let mut all_planet_declinations = Vec::<Vec<(Time, f32)>>::new();
    let planets = Planet::to_vec();
    for planet in planets.into_iter() {
      debug!("Querying planet: {:?}", planet);
      let declinations = Query::query(
        Origin::Geocentric,
        &planet,
        DataType::Declination,
        start_time,
        stop_time
      ).await.expect("failed to query planet declinations");
      debug!("Got {} declinations for planet: {:?}", declinations.len(), planet);
      all_planet_declinations.push(declinations);
    }

    let mut equator_crosses = Vec::<EquatorCross>::new();
    for (index, planet_declinations) in all_planet_declinations.iter().enumerate() {
      let planet = &Planet::to_vec()[index];

      for index in 0..(planet_declinations.len() - 2) {
        let declination = planet_declinations[index];

        if declination.1 < 0.0 && planet_declinations[index+1].1 > 0.0  {
          let (time, angle) = declination;
          equator_crosses.push(EquatorCross::new(time, angle, Direction::Up, planet.clone()));
        } else if declination.1 > 0.0 && planet_declinations[index+1].1 < 0.0 {
          let (time, angle) = declination;
          equator_crosses.push(EquatorCross::new(time, angle, Direction::Down, planet.clone()));
        }
      }
    }
    Self { equator_crosses }
  }

  pub async fn test_declinations(start_date: Time, stop_date: Time, candle_range: usize, error_margin_days: i64) {
    let mut ticker_data = TickerData::new();
    ticker_data.add_csv_series(&PathBuf::from(TICKER_DATA_PATH)).expect("Failed to add CSV to TickerData");
    let reversals = ticker_data.find_reversals(candle_range);
    let declinations = PlanetEquatorCrosses::new(start_date, stop_date).await;

    // iterate over lunar_declinations, identify if it is within +/- error_margin_days of a reversal
    // if so, increment win count
    println!("DATE\t\tDECLINATION\tDIRECTION\tPLANET");
    let mut win_count = 0;
    let total_count = declinations.equator_crosses.len();
    for equator_cross in declinations.equator_crosses {
      let time = equator_cross.date;
      let declination = equator_cross.declination;
      let direction= equator_cross.direction;
      let planet = equator_cross.planet;

      println!("{}\t{}°\t{:?}\t{}", time.to_string(), declination, direction, planet.to_str());
      for reversal in reversals.iter() {
        let range_start = time.delta_date(-(error_margin_days));
        let range_end = time.delta_date(error_margin_days);
        if reversal.candle.date.within_range(range_start, range_end) {
          debug!("{}\t{}°\t{:?}\t{}", time.to_string(), declination, direction, planet.to_str());

          match reversal.reversal_type {
            // price goes down, planet goes from + to - declination
            ReversalType::High => {
              match direction {
                Direction::Up => win_count += 1,
                Direction::Down => win_count += 0
              }
            },
            // price goes up, planet goes from - to + declination
            ReversalType::Low => {
              match direction {
                Direction::Up => win_count += 0,
                Direction::Down => win_count += 1
              }
            }
          }
        }
      }
    }
    let win_rate = win_count as f64 / total_count as f64 * 100.0;
    println!("Win Rate: {}%\t\tWin Events: {}\t\tTotal Events: {}", win_rate, win_count, total_count);
  }
}