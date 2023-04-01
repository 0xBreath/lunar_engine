use log::debug;
use crate::*;
use time_series::*;


pub type ConfluentRetrograde = (Time, Vec<RetrogradeEvent>);

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RetrogradeEvent {
  pub planet: Planet,
  pub start_date: Time,
  pub start_angle: f32,
  pub end_date: Time,
  pub end_angle: f32,
}

#[derive(Debug, Clone)]
pub struct Retrograde {
  pub retrogrades: Vec<RetrogradeEvent>,
  pub start_date: Time,
  pub end_date: Time
}

impl Retrograde {
  /// Search time period for retrograde events
  pub async fn new(start_date: Time, end_date: Time, planets: &Vec<Planet>) -> std::io::Result<Self> {
    if start_date > end_date {
      return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Start date must be before end date"));
    }
    let mut retrogrades = Vec::new();
    for planet in planets.iter() {
      let daily_angles = Query::query(
        Origin::Geocentric,
        planet,
        DataType::RightAscension,
        start_date,
        end_date
      ).await.expect("failed to query planet angles");

      // retrograde identified as longitude decreasing (except)
      // covers case where angle passes through 360
      let mut in_retrograde = false;
      let mut retro_start_date: Option<Time> = None;
      let mut retro_start_angle: Option<f32> = None;
      for index in 0..daily_angles.len() - 1 {
        let (date, angle) = daily_angles[index];
        let (_, next_angle) = daily_angles[index + 1];
        // start of retrograde
        if Self::is_retrograde(angle, next_angle) && !in_retrograde {
          in_retrograde = true;
          retro_start_date = Some(date);
          retro_start_angle = Some(angle);
          continue;
        }
        // end of retrograde
        else if !Self::is_retrograde(angle, next_angle) && in_retrograde {
          in_retrograde = false;
          if retro_start_date.is_some() && retro_start_angle.is_some() {
            let start_date = retro_start_date.expect("failed to unwrap retrograde start date");
            let start_angle = retro_start_angle.expect("failed to unwrap retrograde start angle");
            retrogrades.push(RetrogradeEvent {
              planet: planet.clone(),
              start_date,
              start_angle,
              end_date: date,
              end_angle: angle,
            });
            debug!(
              "{}\t{} at {} to {} at {}",
              planet.to_str(), start_date.to_string(), start_angle, date.to_string(), angle
            );
            retro_start_date = None;
            retro_start_angle = None;
          } else if retro_start_date.is_none() {
            panic!("Retrograde start date not set");
          }
          else if retro_start_angle.is_none() {
            panic!("Retrograde start angle not set");
          }
        }
      }
    }
    Ok(Self {
      retrogrades,
      start_date,
      end_date
    })
  }

  // TODO: identify when >1 retrograde events happens within error margin of days
  pub fn confluent_retrograde(&self, error_margin_days: u8) -> Vec<ConfluentRetrograde> {
    // guaranteed to be positive i64 because of error catch in Self::new()
    let period = self.start_date.diff_days(&self.end_date);
    let mut confluent_retrogrades = Vec::<ConfluentRetrograde>::new();
    for index in 0..period {
      let date = self.start_date.delta_date(index);
      let range_low = date.delta_date(-(error_margin_days as i64));
      let range_high = date.delta_date(error_margin_days as i64);
      let mut events_on_date = Vec::<RetrogradeEvent>::new();
      for retrograde in self.retrogrades.iter() {
        if retrograde.start_date.within_range(range_low, range_high)
          || retrograde.end_date.within_range(range_low, range_high) {
          events_on_date.push(retrograde.clone());
        }
      }
      if events_on_date.len() > 1 {
        confluent_retrogrades.push((date, events_on_date.clone()));
        println!("{}\t{}", date.to_string(), events_on_date.len());
      }
    }
    confluent_retrogrades
  }

  pub fn is_retrograde(first: f32, second: f32) -> bool {
    second - first < 0.0
  }

  pub fn print(&self) {
    for retrograde in self.retrogrades.iter() {
      println!("{}\t{}\t{}", retrograde.planet.to_str(), retrograde.start_date.to_string(), retrograde.end_date.to_string());
    }
  }
}