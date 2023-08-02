use crate::quantities::Quantities;
use crate::step_size::StepSize;
use crate::target::Target;
use crate::{Alignment, DataType, Declination, Origin, Planet, RightAscension};
use log::debug;
use std::fmt::Display;
use time_series::time::Time;
use time_series::TimeError;

pub const BASE_QUERY: &str = "https://ssd.jpl.nasa.gov/api/horizons.api?format=text";

#[derive(Debug)]
pub enum QueryError {
    StopTimeBeforeEndTime,
    ReqwestError(reqwest::Error),
    TimeError(TimeError),
}

pub type QueryResult<T> = Result<T, QueryError>;

impl Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::StopTimeBeforeEndTime => write!(f, "Stop time must be after start time"),
            QueryError::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
            QueryError::TimeError(e) => write!(f, "Time error: {}", e),
        }
    }
}

impl From<reqwest::Error> for QueryError {
    fn from(e: reqwest::Error) -> Self {
        QueryError::ReqwestError(e)
    }
}

pub struct Query {
    pub value: String,
}

impl Query {
    /// Compute alignment of a planet relative to the Origin (Earth or Sun) for a period of time.
    /// Return a vector of dates with significant alignments.
    pub async fn query(
        origin: Origin,
        planet: &Planet,
        data_type: DataType,
        start_time: Time,
        stop_time: Time,
    ) -> QueryResult<Vec<(Time, f32)>> {
        // swap start and stop time if period is historical rather than for the future
        let diff_days = start_time
            .diff_days(&stop_time)
            .map_err(QueryError::TimeError)?;
        if diff_days < 0 {
            //std::mem::swap(&mut start_time, &mut stop_time);
            return Err(QueryError::StopTimeBeforeEndTime);
        }
        let query = Query::build_query(
            Target::new(planet),
            start_time,
            stop_time,
            Quantities::default(),
            origin,
        );

        let data = reqwest::get(query.value).await?.text().await?;
        let data = Self::extract_data(data);
        match data_type {
            DataType::RightAscension => Ok(Self::format_for_right_ascension(data)),
            DataType::Declination => Ok(Self::format_for_declination(data)),
        }
    }

    pub fn sync_query(
        origin: Origin,
        planet: &Planet,
        data_type: DataType,
        start_time: Time,
        stop_time: Time,
    ) -> QueryResult<Vec<(Time, f32)>> {
        // swap start and stop time if period is historical rather than for the future
        let diff_days = start_time
            .diff_days(&stop_time)
            .map_err(QueryError::TimeError)?;
        if diff_days < 0 {
            //std::mem::swap(&mut start_time, &mut stop_time);
            return Err(QueryError::StopTimeBeforeEndTime);
        }
        let query = Query::build_query(
            Target::new(planet),
            start_time,
            stop_time,
            Quantities::default(),
            origin,
        );

        let res = reqwest::blocking::get(query.value)?;
        debug!("Horizons API Status: {}", res.status());
        let data = res.text()?;

        let data = Self::extract_data(data);
        match data_type {
            DataType::RightAscension => Ok(Self::format_for_right_ascension(data)),
            DataType::Declination => Ok(Self::format_for_declination(data)),
        }
    }

    /// Construct a query to interact with the 'Horizon API'
    fn build_query(
        command: Target,
        start_time: Time,
        stop_time: Time,
        quantities: Quantities,
        origin: Origin,
    ) -> Self {
        let origin = origin.as_query();
        let default_args = "&OBJ_DATA='YES'&MAKE_EPHEM='YES'&EPHEM_TYPE='OBSERVER'";
        let step_size = StepSize::default().value;
        let value = format!(
            "{}{}{}{}{}{}{}{}",
            BASE_QUERY,
            command.value,
            default_args,
            origin,
            start_time.start_time(),
            stop_time.stop_time(),
            step_size,
            quantities.value
        );
        Self { value }
    }

    /// Isolate planetary ephemeral data from API response
    pub fn extract_data(response: String) -> String {
        let mut data = String::new();
        let mut lines = response.lines();
        while let Some(line) = lines.next() {
            if line.contains("$$SOE") {
                for line in lines.by_ref() {
                    if line.contains("$$EOE") {
                        break;
                    }
                    data.push_str(line);
                    data.push_str("\r\n");
                }
            }
        }
        data
    }

    /// Format API response of into vector of (Time, right ascension as degrees)
    pub fn format_for_right_ascension(data: String) -> Vec<(Time, f32)> {
        let mut vec = Vec::new();
        let lines = data.lines();
        for line in lines {
            let mut line = line.split_whitespace();
            let time = String::from(line.next().unwrap());
            let time = Time::convert_api_response(&time);
            // next after YYYY-MM-DD is seconds... skip it
            line.next().unwrap();
            // next 3 values are Right Ascension (HH MM SS.SS)
            let ra_hh = line.next().unwrap().parse::<f32>().unwrap();
            let ra_mm = line.next().unwrap().parse::<f32>().unwrap();
            let ra_ssss = line.next().unwrap().parse::<f32>().unwrap();
            let ra_degrees = RightAscension::new(ra_hh as i32, ra_mm as i32, ra_ssss).to_degrees();
            vec.push((time, ra_degrees));
        }
        vec
    }

    /// Format API response of into vector of (Time, declination as degrees)
    pub fn format_for_declination(data: String) -> Vec<(Time, f32)> {
        let mut vec = Vec::new();
        let lines = data.lines();
        for line in lines {
            let mut line = line.split_whitespace();
            let time = String::from(line.next().unwrap());
            let time = Time::convert_api_response(&time);
            // next after YYYY-MM-DD is seconds... skip it
            line.next().unwrap();
            // next 3 values are Right Ascension (HH MM SS.SS)... skip it
            line.next().unwrap();
            line.next().unwrap();
            line.next().unwrap();
            // next 3 values are Declination (degrees MM SS.S)
            // isolate sign +/- from degrees
            let degrees_with_sign = line.next().unwrap();
            let sign = degrees_with_sign.chars().next().unwrap();
            let is_positive = match sign {
                '+' => true,
                '-' => false,
                _ => panic!("invalid sign"),
            };
            let degrees = degrees_with_sign[1..].parse::<f32>().unwrap();
            let minutes = line.next().unwrap().parse::<f32>().unwrap();
            let seconds = line.next().unwrap().parse::<f32>().unwrap();
            let declination =
                Declination::from_api_response(is_positive, degrees, minutes, seconds);
            vec.push((time, declination));
        }
        vec
    }

    /// Finds duplicate Alignments on consecutive dates where f32 is within margin of error
    /// filters for the date with f32 closest to the actual Alignment angle
    pub fn remove_duplicate_values(
        vec: &mut Vec<(Time, f32, Alignment)>,
    ) -> Vec<(Time, f32, Alignment)> {
        if vec.is_empty() {
            return vec.to_vec();
        }
        let mut clean_values = Vec::new();
        let mut i = 0;
        while i < vec.len() - 1 {
            let mut index = i + 1;
            if vec.get(index).is_none() || vec.get(i).is_none() {
                break;
            }
            let mut closest_to_alignment_angle = (vec[i].2.to_num() - vec[i].1).abs();
            let mut closest_to_alignment_index = i;
            while vec[index].2 == vec[i].2 {
                let angle_to_alignment = (vec[index].2.to_num() - vec[index].1).abs();
                if angle_to_alignment < closest_to_alignment_angle {
                    closest_to_alignment_angle = angle_to_alignment;
                    closest_to_alignment_index = index;
                }
                index += 1;
                if vec.get(index).is_none() {
                    break;
                }
            }
            i = index;
            clean_values.push(vec[closest_to_alignment_index].clone());
            if i >= vec.len() {
                break;
            }
        }
        clean_values
    }
}
