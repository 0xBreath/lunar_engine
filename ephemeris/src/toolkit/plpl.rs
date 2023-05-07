use log::debug;
use crate::*;
use time_series::{Candle, TickerDataError, Time};

#[derive(Debug, Clone, Copy)]
pub enum PLPLError {
  NumPLPLsNotEven,
  NoPLPLForDate,
  NoPLPLClosest,
  TickerDataError(TickerDataError)
}

pub type PLPLResult<T> = Result<T, PLPLError>;

#[derive(Debug, Clone)]
pub struct PLPLSystemConfig {
  pub planet: Planet,
  pub origin: Origin,
  pub date: Time,
  pub plpl_scale: f32,
  pub plpl_price: f32,
  pub num_plpls: u32,
  pub cross_margin_pct: f32
}

#[derive(Debug, Clone)]
pub struct PLPLSystem {
  pub planet: Planet,
  pub origin: Origin,
  pub date: Time,
  pub planet_angle: f32,
  pub plpls: Vec<f32>,
  pub scale: f32,
  pub price: f32,
  pub cross_margin_pct: f32,
  pub num_plpls: u32
}

impl PLPLSystem {
  pub fn new(config: PLPLSystemConfig) -> PLPLResult<Self> {
    if config.num_plpls % 2 != 0 {
      return Err(PLPLError::NumPLPLsNotEven);
    }
    let mut me = Self {
      planet: config.planet,
      origin: config.origin,
      date: config.date,
      planet_angle: 0.0,
      plpls: vec![],
      scale: config.plpl_scale,
      price: config.plpl_price,
      cross_margin_pct: config.cross_margin_pct,
      num_plpls: config.num_plpls
    };
    me.planet_angle = me.helio();
    me.plpls = me.plpls()?;
    Ok(me)
  }

  fn helio(&self) -> f32 {
    debug!("Querying ephemeris from Horizons API");
    let start_date = self.date.delta_date(-1);
    let query = Query::sync_query(
      self.origin,
      &self.planet,
      DataType::RightAscension,
      start_date,
      self.date
    ).expect("failed to query planet angles");
    let target = match query.last() {
      Some(last) => last,
      None => panic!("Planet longitude query returned no results")
    };
    if target.0 != self.date {
      panic!("Planet longitude query returned no results for target date");
    }
    target.1
  }

  fn up_op(&self) -> f32 {
    360.0 * self.scale
  }

  fn down_op(&self) -> f32 {
    -360.0 * self.scale
  }

  /// Compute the PLPL for today based on the planet longitude, scale and price
  /// Find all PLPL values (360 cycles) scaled up and down from the base PLPL
  /// Find all PLPLs for each date
  pub fn plpls(&self) -> PLPLResult<Vec<f32>> {
    let angle = self.planet_angle;
    let price_factor = (self.price / (360.0 * self.scale)).round();
    let scale_360 = 360.0 * self.scale;
    let plpl = price_factor * scale_360 + angle;
    self.plpls_inner(plpl)
  }

  /// All PLPL values (360 cycles) scaled up and down
  /// from the origin PLPL which is derived from price and scale
  fn plpls_inner(&self, plpl: f32) -> PLPLResult<Vec<f32>> {
    let mut plpls = Vec::<f32>::new();
    let mut op_mult = self.num_plpls/2;
    for _ in 0..(self.num_plpls/2 - 1) {
      let plpl_down = plpl + self.down_op() * op_mult as f32;
      plpls.push(plpl_down);
      op_mult -= 1;
    }
    op_mult = 0;
    for _ in (self.num_plpls/2)..(self.num_plpls - 1) {
      let plpl_up = plpl + self.up_op() * op_mult as f32;
      plpls.push(plpl_up);
      op_mult += 1;
    }
    Ok(plpls)
  }

  /// Find the closest PLPL to price on this date
  pub fn closest_plpl(&self, candle: &Candle) -> PLPLResult<f32> {
    let mut closest_plpl = None;
    let mut closest_plpl_distance: Option<f64> = None;
    for plpl in self.plpls.iter() {
      match closest_plpl_distance {
        None => {
          closest_plpl_distance = Some((*plpl as f64 - candle.close).abs());
          closest_plpl = Some(plpl);
        },
        Some(closest_distance) => {
          let curr_distance = (*plpl as f64 - candle.close).abs();
          if curr_distance < closest_distance {
            closest_plpl_distance = Some(curr_distance);
            closest_plpl = Some(plpl);
          }
        }
      }
    }
    match closest_plpl {
      Some(plpl) => Ok(*plpl),
      None => Err(PLPLError::NoPLPLClosest)
    }
  }

  pub fn margin(&self) -> f32 {
    self.up_op() * self.cross_margin_pct/100.0
  }

  pub fn long_signal(&self, prev_candle: &Candle, candle: &Candle, closest_plpl: f32) -> bool {
    let plpl = closest_plpl as f64;
    prev_candle.close <= plpl && candle.close > plpl - self.margin() as f64
  }

  pub fn short_signal(&self, prev_candle: &Candle, candle: &Candle, closest_plpl: f32) -> bool {
    let plpl = closest_plpl as f64;
    prev_candle.close >= plpl && candle.close < plpl + self.margin() as f64
  }
}