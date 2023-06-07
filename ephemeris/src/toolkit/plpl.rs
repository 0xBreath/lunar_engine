use crate::*;
use log::{debug, error};
use std::fmt::Display;
use time_series::{Candle, TickerDataError, Time};

#[derive(Debug)]
pub enum PLPLError {
    NumPLPLsNotEven,
    NoPLPLForDate,
    NoPLPLClosest,
    TickerDataError(TickerDataError),
    QueryError(QueryError),
}

impl Display for PLPLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PLPLError::NumPLPLsNotEven => write!(f, "Number of PLPLs must be even"),
            PLPLError::NoPLPLForDate => write!(f, "No PLPL for date"),
            PLPLError::NoPLPLClosest => write!(f, "No PLPL closest to date"),
            PLPLError::TickerDataError(e) => write!(f, "TickerDataError: {}", e),
            PLPLError::QueryError(e) => write!(f, "QueryError: {}", e),
        }
    }
}

pub type PLPLResult<T> = Result<T, PLPLError>;

#[derive(Debug, Clone)]
pub struct PLPLSystemConfig {
    pub planet: Planet,
    pub origin: Origin,
    pub first_date: Time,
    pub last_date: Time,
    pub plpl_scale: f32,
    pub plpl_price: f32,
    pub num_plpls: u32,
    pub cross_margin_pct: f32,
}

#[derive(Debug, Clone)]
pub struct PLPLSystem {
    pub planet: Planet,
    pub origin: Origin,
    pub first_date: Time,
    pub last_date: Time,
    pub planet_angles: Vec<(Time, f32)>,
    pub plpls: Vec<PLPL>,
    pub scale: f32,
    pub price: f32,
    pub cross_margin_pct: f32,
    pub num_plpls: u32,
}

#[derive(Debug, Clone)]
pub struct PLPL {
    pub date: Time,
    pub plpls: Vec<f32>,
}

impl PLPLSystem {
    pub async fn new(config: PLPLSystemConfig) -> PLPLResult<Self> {
        if config.num_plpls % 2 != 0 {
            return Err(PLPLError::NumPLPLsNotEven);
        }
        let mut me = Self {
            planet: config.planet,
            origin: config.origin,
            first_date: config.first_date,
            last_date: config.last_date,
            planet_angles: vec![],
            plpls: vec![],
            scale: config.plpl_scale,
            price: config.plpl_price,
            cross_margin_pct: config.cross_margin_pct,
            num_plpls: config.num_plpls,
        };
        me.planet_angles = me.helio().await?;
        me.plpls = me.plpls()?;
        Ok(me)
    }

    async fn helio(&self) -> PLPLResult<Vec<(Time, f32)>> {
        debug!("Querying ephemeris from Horizons API");
        let start_date = self.first_date.delta_date(-1);
        let end_date = self.last_date.delta_date(1);
        let query = Query::query(
            self.origin,
            &self.planet,
            DataType::RightAscension,
            start_date,
            end_date,
        )
        .await;
        match query {
            Ok(query) => Ok(query),
            Err(e) => Err(PLPLError::QueryError(e)),
        }
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
    pub fn plpls(&self) -> PLPLResult<Vec<PLPL>> {
        let mut plpls = Vec::<PLPL>::new();
        for planet_angle in self.planet_angles.iter() {
            let angle = planet_angle.1;
            let price_factor = (self.price / (360.0 * self.scale)).round();
            let scale_360 = 360.0 * self.scale;
            let plpl = price_factor * scale_360 + angle;
            let res = self.plpls_inner(plpl)?;
            plpls.push(PLPL {
                date: planet_angle.0,
                plpls: res,
            });
        }
        Ok(plpls)
    }

    /// All PLPL values (360 cycles) scaled up and down
    /// from the origin PLPL which is derived from price and scale
    fn plpls_inner(&self, plpl: f32) -> PLPLResult<Vec<f32>> {
        let mut plpls = Vec::<f32>::new();
        let mut op_mult = self.num_plpls / 2;
        for _ in 0..(self.num_plpls / 2 - 1) {
            let plpl_down = plpl + self.down_op() * op_mult as f32;
            plpls.push(plpl_down);
            op_mult -= 1;
        }
        op_mult = 0;
        for _ in (self.num_plpls / 2)..(self.num_plpls - 1) {
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
        let plpls = self.plpls_for_date(candle.date)?;
        for plpl in plpls {
            match closest_plpl_distance {
                None => {
                    closest_plpl_distance = Some((plpl as f64 - candle.close).abs());
                    closest_plpl = Some(plpl);
                }
                Some(closest_distance) => {
                    let curr_distance = (plpl as f64 - candle.close).abs();
                    if curr_distance < closest_distance {
                        closest_plpl_distance = Some(curr_distance);
                        closest_plpl = Some(plpl);
                    }
                }
            }
        }
        match closest_plpl {
            Some(plpl) => Ok(plpl),
            None => {
                error!("No closest PLPL found for date {}", candle.date.to_string());
                Err(PLPLError::NoPLPLClosest)
            }
        }
    }

    fn plpls_for_date(&self, date: Time) -> PLPLResult<Vec<f32>> {
        let mut plpls = None;
        for plpl in self.plpls.iter() {
            if plpl.date == date {
                plpls = Some(plpl.plpls.clone());
                break;
            }
        }
        match plpls {
            Some(plpls) => Ok(plpls),
            None => Err(PLPLError::NoPLPLForDate),
        }
    }

    pub fn margin(&self) -> f32 {
        self.up_op() * self.cross_margin_pct / 100.0
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
