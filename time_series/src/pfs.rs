use crate::{TickerData, Time};
use chrono::{Duration, Local, NaiveDate, TimeZone};
use plotters::prelude::*;

/// Polarity Factor System
pub struct PFS {
  pub date: Time,
  pub value: f64
}

impl PFS {
  pub fn new(date: Time, value: f64) -> Self {
    Self { date, value }
  }
}

pub struct PlotPFS {
  pub cycle_years: u32,
  pub start_date: Time,
  pub end_date: Time
}

impl PlotPFS {
  pub fn new(cycle_years: u32, start_date: Time, end_date: Time) -> Self {
    Self {
      cycle_years,
      start_date,
      end_date
    }
  }

  pub fn pfs(&self, ticker_data: &TickerData) -> Vec<PFS> {
    let mut daily_pfs = Vec::<PFS>::new();

    // compute number of cycles possible in candle history
    let earliest_candle_year = ticker_data.earliest_date().year;
    let latest_candle_year = ticker_data.latest_date().year;
    let num_cycles = (latest_candle_year - earliest_candle_year) / self.cycle_years as i32;

    let time_period = self.start_date.time_period(&self.end_date);
    for date in time_period.iter() {
      // PFS for this date
      let mut pfs = (100.0, 1);
      // iterate possible cycles in candle history
      let mut found_cycle_date = false;
      for cycle in 1..num_cycles + 1 {
        // find candle X cycles back
        for (index, candle) in ticker_data.candles.iter().enumerate() {
          if index == 0 {
            continue;
          }
          // used to compute percent change between candles
          let prev_candle = ticker_data.candles.get(index - 1).expect("Failed to get previous candle");
          // candle X cycles back
          let cycle_date = Time::new(date.year - self.cycle_years as i32 * cycle, &date.month, &date.day, None, None);
          if &cycle_date < ticker_data.earliest_date() {
            continue;
          }
          // found candle X cycles back
          if prev_candle.date < cycle_date && candle.date >= cycle_date {
            let change = candle.percent_change(prev_candle.close);
            pfs = (pfs.0 + change, pfs.1 + 1);
            found_cycle_date = true;
            break;
          }
        }
      }
      daily_pfs.push(PFS {
        date: *date,
        value: pfs.0 / pfs.1 as f64
      });
    }
    daily_pfs
  }

  pub fn plot_pfs(&self, daily_pfs: &[PFS], out_file: &str, plot_title: &str, plot_color: &RGBColor, plot_height: (f32, f32)) {
    // get daily PFS data
    let data = self.get_data(daily_pfs);
    // draw chart
    let root = BitMapBackend::new(out_file, (2048, 1024)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    // PFS start date
    let from_date_index = self.find_date_index(&data, &self.start_date);
    let from_date_input = self.parse_time(&data[from_date_index].0);
    let from_date = from_date_input - Duration::days(1);
    println!("PFS Start Date: {}", from_date);
    // PFS end date
    let to_date_index = self.find_date_index(&data, &self.end_date);
    let to_date_input = self.parse_time(&data[to_date_index].0);
    let to_date = to_date_input + Duration::days(1);
    println!("PFS End Date: {}", to_date);
    // label chart
    let y_min = plot_height.0;
    let y_max = plot_height.1;
    let mut chart = ChartBuilder::on(&root)
      .x_label_area_size(40)
      .y_label_area_size(40)
      .caption(plot_title, ("sans-serif", 50.0).into_font())
      .build_cartesian_2d(from_date..to_date, y_min..y_max).unwrap();
    chart.configure_mesh().light_line_style(WHITE).draw().unwrap();
    // plot PFS values
    chart.draw_series(
      LineSeries::new(data.iter().map(|x| (self.parse_time(&x.0), x.1)), ShapeStyle {
        color: RGBAColor::from(*plot_color),
        filled: true,
        stroke_width: 2,
      })
        .point_size(5)
    ).unwrap();
    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect("Unable to write result to file, please make sure 'plotters-doc-data' dir exists under current dir");
    println!("Result has been saved to {}", out_file);
  }

  fn get_data(&self, daily_pfs: &[PFS]) -> Vec<(String, f32)> {
    let mut data = Vec::new();
    for pfs in daily_pfs.iter() {
      data.push((
        pfs.date.to_string_daily(),
        pfs.value as f32,
      ));
    }
    data
  }

  fn find_date_index(&self, data: &[(String, f32)], date: &Time) -> usize {
    for (i, (d, _)) in data.iter().enumerate() {
      if d == &date.to_string_daily() {
        return i;
      }
    }
    let mut not_found = true;
    let mut change_date = *date;
    while not_found {
      change_date = change_date.delta_date(-1);
      // get previous index in data
      for (i, (d, _)) in data.iter().enumerate() {
        if d == &change_date.to_string_daily() {
          not_found = false;
          return i;
        }
      }
    }
    panic!("Date not found");
  }

  fn parse_time(&self, t: &str) -> NaiveDate {
    Local
      .datetime_from_str(&format!("{} 0:0", t), "%Y-%m-%d %H:%M")
      .unwrap_or_else(|_| panic!("Failed to parse time {}", t))
      .date_naive()
  }
}