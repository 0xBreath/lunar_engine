
pub enum DataType {
  RightAscension,
  Declination,
}

pub struct RightAscension {
  pub hours: i32,
  pub minutes: i32,
  pub seconds: f32
}

impl RightAscension {
  pub fn new(hours: i32, minutes: i32, seconds: f32) -> Self {
    Self {
      hours,
      minutes,
      seconds,
    }
  }
  /// Convert Right Ascension to degrees
  pub fn to_degrees(&self) -> f32 {
    let hours = self.hours as f32;
    let minutes = self.minutes as f32;
    let seconds = self.seconds;
    let total_seconds = (hours * 3600.0) + (minutes * 60.0) + seconds;
    // 86_400 seconds/day / 240 = 360 degrees/day
    total_seconds / 240.0
  }
}

pub struct Declination {
  pub degrees: f32
}

impl Declination {
  pub fn from_api_response(is_positive: bool, degrees: f32, minutes: f32, seconds: f32) -> f32 {
    let degrees = degrees + (minutes / 60.0) + (seconds / 3600.0);
    if is_positive {
      degrees
    } else {
      degrees * -1.0
    }
  }
}