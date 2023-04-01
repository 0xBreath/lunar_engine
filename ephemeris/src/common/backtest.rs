use crate::Planet;
use time_series::Candle;

#[derive(Debug, Clone)]
pub struct Backtest {
  /// Vector of trade signals caused by:
  /// planet alignment (as f32 angle) with price on a specific date (as Candle)
  /// Multiple signals could occur for one backtested candle.
  pub signals: Option<Vec<(Planet, f32, Candle)>>,
  pub win_count: u64,
  pub total_count: u64,
  pub win_rate: f64,
}
impl Backtest {
  pub fn default() -> Backtest {
    Backtest {
      signals: None,
      win_count: 0,
      total_count: 0,
      win_rate: 0.0
    }
  }

  pub fn add_signal(&mut self, signal: (Planet, f32, Candle)) {
    if self.signals.is_some() {
      self.signals.as_mut().unwrap().push(signal);
    } else {
      self.signals = Some(vec![signal]);
    }
  }

  pub fn get_win_count(&self) -> u64 {
    self.win_count
  }

  pub fn increment_win_count(&mut self) {
    self.win_count += 1;
    self.set_win_rate();
  }

  pub fn get_total_count(&self) -> u64 {
    self.total_count
  }

  pub fn increment_total_count(&mut self) {
    self.total_count += 1;
    self.set_win_rate();
  }

  fn set_win_rate(&mut self) {
    self.win_rate = self.win_count as f64 / self.total_count as f64;
  }

  pub fn get_win_rate(&self) -> f64 {
    self.win_rate
  }
}