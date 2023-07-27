use crate::*;
use log::debug;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use time_series::{Time, TimeError};

#[derive(Debug)]
pub enum EclipseError {
    TimeError(TimeError),
}

impl std::fmt::Display for EclipseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EclipseError::TimeError(e) => write!(f, "TimeError: {}", e),
        }
    }
}

pub type EclipseResult<T> = Result<T, EclipseError>;

#[derive(Debug, Clone)]
pub struct Eclipses {
    pub events: Vec<EclipseEvent>,
}

impl Eclipses {
    pub fn new(solar_eclipse_csv: &PathBuf, lunar_eclipse_csv: &PathBuf) -> Self {
        //
        // read Solar Eclipse CSV and load into Vec<EclipseEvent>
        let solar_buffer = File::open(solar_eclipse_csv).unwrap();
        let mut solar = csv::Reader::from_reader(solar_buffer);
        let mut solar_events = Vec::<EclipseEvent>::new();
        for record in solar.records().flatten() {
            let date_unformatted = &record[1];
            let date = Time::from_eclipse_date_format(date_unformatted);
            let eclipse_type = &record[6];
            let eclipse_phase = EclipseType::from_symbol(eclipse_type, EclipseClass::Solar);
            solar_events.push(EclipseEvent::new(date, eclipse_phase));
        }
        //
        // read Lunar Eclipse CSV and load into Vec<EclipseEvent>
        let lunar_buffer = File::open(lunar_eclipse_csv).unwrap();
        let mut lunar = csv::Reader::from_reader(lunar_buffer);
        let mut lunar_events = Vec::<EclipseEvent>::new();
        for record in lunar.records().flatten() {
            let date_unformatted = &record[1];
            let date = Time::from_eclipse_date_format(date_unformatted);
            let eclipse_type = &record[6];
            let eclipse_phase = EclipseType::from_symbol(eclipse_type, EclipseClass::Lunar);
            lunar_events.push(EclipseEvent::new(date, eclipse_phase));
        }
        //
        // concatenate solar and lunar events into one Vec<(Time, EclipseType)>
        let mut events = [solar_events, lunar_events].concat();
        // sort events by Time
        events.sort_by(|eclipse_1, eclipse_2| eclipse_1.date.partial_cmp(&eclipse_2.date).unwrap());

        Self { events }
    }

    pub fn print(&self, file: &PathBuf, start_date: &Time, end_date: &Time) {
        let mut file = File::create(file).unwrap();
        println!("DATE\tRANK");
        let _ = file
            .write("date,rank\n".to_string().as_bytes())
            .expect("Unable to write to file");
        for event in self.events.iter() {
            if &event.date >= start_date && &event.date <= end_date {
                println!("{}\t{}", event.date.to_string(), event.kind.to_rank());
                let _ = file.write(
                    format!("{},{}\n", event.date.to_string(), event.kind.to_rank())
                        .to_string()
                        .as_bytes(),
                );
            }
        }
    }

    /// Compare previous eclipses and determine if the same planet
    /// will cross the celestial equator (zero declination cross) during both eclipses.
    pub async fn planet_equator_cross_on_two_eclipses(
        &self,
        start_time: Time,
        stop_time: Time,
        error_margin_days: i64,
    ) -> Vec<PlanetEquatorCrossTwoEclipses> {
        let planet_equator_crosses = PlanetEquatorCrosses::new(start_time, stop_time).await;

        // iterate planet declinations and find when
        // the same planet crosses zero declination on a second eclipse
        // return those events as Vec<PlanetEquatorCrossTwoEclipses>
        let mut signals = Vec::<PlanetEquatorCrossTwoEclipses>::new();
        let mut planet_equator_crosses_two_eclipses: Vec<Option<PlanetEquatorCrossTwoEclipses>> =
            vec![None; Planet::to_vec().len()];

        for equator_cross in planet_equator_crosses.equator_crosses.into_iter() {
            let planet_index = equator_cross.planet.to_index();
            for event in self.events.iter() {
                let range_start = event.date.delta_date(-(error_margin_days));
                let range_end = event.date.delta_date(error_margin_days);
                if equator_cross.date.within_range(range_start, range_end) {
                    if planet_equator_crosses_two_eclipses[planet_index].is_none() {
                        planet_equator_crosses_two_eclipses[planet_index] =
                            Some(PlanetEquatorCrossTwoEclipses::new(
                                equator_cross.planet.clone(),
                                event.clone(),
                                event.clone(),
                            ));
                    } else {
                        let mut planet_equator_cross_two_eclipses =
                            planet_equator_crosses_two_eclipses[planet_index]
                                .take()
                                .unwrap();
                        planet_equator_cross_two_eclipses.second_eclipse = event.clone();

                        debug!(
                            "{}\t{}\t{}",
                            &planet_equator_cross_two_eclipses.planet.to_str(),
                            &planet_equator_cross_two_eclipses
                                .first_eclipse
                                .date
                                .to_string(),
                            &planet_equator_cross_two_eclipses
                                .second_eclipse
                                .date
                                .to_string(),
                        );

                        signals.push(planet_equator_cross_two_eclipses.clone());
                        planet_equator_crosses_two_eclipses[planet_index] =
                            Some(PlanetEquatorCrossTwoEclipses::new(
                                equator_cross.planet.clone(),
                                planet_equator_cross_two_eclipses.second_eclipse.clone(),
                                planet_equator_cross_two_eclipses.second_eclipse.clone(),
                            ));
                    }
                }
            }
        }
        signals
    }

    /// Find confluence between PlanetMatrix and EclipseEvents.
    pub async fn planet_matrix_alignments_on_eclipses(
        &self,
        start_time: Time,
        end_time: Time,
        error_margin_degrees: f32,
        planets: &Vec<Planet>,
        harmonics: &Vec<Alignment>,
    ) -> Vec<PlanetPairAlignmentOnEclipse> {
        let planet_matrix = PlanetMatrix::new(
            Origin::Geocentric,
            &start_time,
            &end_time,
            error_margin_degrees,
            planets,
            harmonics,
        )
        .await
        .unwrap();

        let mut signals = Vec::<PlanetPairAlignmentOnEclipse>::new();
        for event in self.events.iter() {
            let planet_pair_alignments: Vec<PlanetPairAlignment> =
                planet_matrix.alignments_on_date(&event.date);
            for alignment in planet_pair_alignments.iter() {
                let signal = PlanetPairAlignmentOnEclipse {
                    planet_pair: alignment.planet_pair.clone(),
                    eclipse: event.clone(),
                    alignment: alignment.alignment.clone(),
                };
                debug!(
                    "{}-{}\t{}\t{}\t{:?}",
                    signal.planet_pair.0.to_str(),
                    signal.planet_pair.1.to_str(),
                    signal.eclipse.date.to_string(),
                    alignment.date.to_string(),
                    signal.alignment.to_str(),
                );
                signals.push(signal);
            }
        }
        signals
    }

    /// Search for planets entering/exiting retrograde motion during an eclipse.
    pub async fn planet_retrograde_on_eclipses(
        &self,
        start_date: Time,
        end_date: Time,
        error_margin_days: i64,
        planets: &Vec<Planet>,
    ) -> Vec<PlanetRetrogradeOnEclipse> {
        let retrograde = Retrograde::new(start_date, end_date, planets)
            .await
            .unwrap();

        let mut signals = Vec::<PlanetRetrogradeOnEclipse>::new();
        for eclipse in self.events.iter() {
            for retro_event in retrograde.retrogrades.iter() {
                // retrograde start or end occurred within error margin of days from an eclipse.
                if eclipse.date.within_range(
                    retro_event.start_date.delta_date(-(error_margin_days)),
                    retro_event.start_date.delta_date(error_margin_days),
                ) || eclipse.date.within_range(
                    retro_event.end_date.delta_date(-(error_margin_days)),
                    retro_event.end_date.delta_date(error_margin_days),
                ) {
                    let signal = PlanetRetrogradeOnEclipse::new(
                        retro_event.planet.clone(),
                        eclipse.clone(),
                        retro_event.clone(),
                    );
                    debug!("{}\t{:?}", signal.planet.to_str(), signal.eclipse.kind,);
                    signals.push(signal);
                }
            }
        }
        signals
    }

    // TODO: refactor to compare eclipse as equal to other other signals,
    //  so that we can use the same function for all signals.
    //  iterate through all dates in range, instead of filtering for eclipses.
    /// Search for confluence between all signals.
    pub async fn test_confluence(
        &self,
        start_time: Time,
        end_time: Time,
        error_margin_days: i64,
        error_margin_degrees: f32,
        planets: Vec<Planet>,
        harmonics: Vec<Alignment>,
    ) -> EclipseResult<Vec<EclipseSignals>> {
        let _diff_days = start_time
            .diff_days(&end_time)
            .map_err(EclipseError::TimeError)?;
        let eclipse_equator_cross: Vec<PlanetEquatorCrossTwoEclipses> = self
            .planet_equator_cross_on_two_eclipses(start_time, end_time, error_margin_days)
            .await;
        let eclipse_planet_matrix: Vec<PlanetPairAlignmentOnEclipse> = self
            .planet_matrix_alignments_on_eclipses(
                start_time,
                end_time,
                error_margin_degrees,
                &planets,
                &harmonics,
            )
            .await;
        // println!("\t\t### PLANET RETROGRADE START/END ON ECLIPSE ###\t\t");
        let eclipse_retrograde: Vec<PlanetRetrogradeOnEclipse> = self
            .planet_retrograde_on_eclipses(start_time, end_time, error_margin_days, &planets)
            .await;

        // find EclipseEvent at start_time and end_time to reduce iteration time
        let (start_index, _) = self
            .events
            .iter()
            .enumerate()
            .find(|(_, event)| event.date.year == start_time.year)
            .expect("failed to find start time in EclipseEvents");
        let (end_index, _) = self
            .events
            .iter()
            .enumerate()
            .find(|(_, event)| event.date.year == end_time.year)
            .expect("failed to find end time in EclipseEvents");

        // iterate through dates and check if each signal is present
        // for each date store the signals in a vector as `Signal`
        let mut signals = Vec::<EclipseSignals>::new();
        for eclipse_index in start_index..end_index + 1 {
            let eclipse = &self.events[eclipse_index];
            let range_low = eclipse.date.delta_date(-(error_margin_days));
            let range_high = eclipse.date.delta_date(error_margin_days);

            let mut eclipse_signals = EclipseSignals {
                eclipse: eclipse.clone(),
                retrograde_signals: None,
                planet_pair_alignment_signals: None,
                self_alignment_signals: None,
                equator_cross_signals: None,
            };

            for signal in eclipse_equator_cross.iter() {
                if signal
                    .second_eclipse
                    .date
                    .within_range(range_low, range_high)
                {
                    match &mut eclipse_signals.equator_cross_signals {
                        None => {
                            let vec = vec![signal.clone()];
                            eclipse_signals.equator_cross_signals = Some(vec);
                        }
                        Some(equator_cross_signals) => equator_cross_signals.push(signal.clone()),
                    }
                }
            }
            for signal in eclipse_planet_matrix.iter() {
                if signal.eclipse.date.within_range(range_low, range_high) {
                    match &mut eclipse_signals.planet_pair_alignment_signals {
                        None => {
                            let vec = vec![signal.clone()];
                            eclipse_signals.planet_pair_alignment_signals = Some(vec);
                        }
                        Some(planet_pair_alignment_signals) => {
                            planet_pair_alignment_signals.push(signal.clone())
                        }
                    }
                }
            }
            for signal in eclipse_retrograde.iter() {
                if signal.eclipse.date.within_range(range_low, range_high) {
                    match &mut eclipse_signals.retrograde_signals {
                        None => {
                            let vec = vec![signal.clone()];
                            eclipse_signals.retrograde_signals = Some(vec);
                        }
                        Some(retrograde_signals) => retrograde_signals.push(signal.clone()),
                    }
                }
            }
            signals.push(eclipse_signals);
        }

        println!("DATE\t\tRANK\tINGRESS\tRETRO\tMATRIX\tSELF\tDECLINATION");
        for signal in signals.iter() {
            let retrograde = match &signal.retrograde_signals {
                Some(signals) => signals.len(),
                None => 0,
            };
            let planet_matrix = match &signal.planet_pair_alignment_signals {
                Some(signals) => signals.len(),
                None => 0,
            };
            let planet_self_alignment = match &signal.self_alignment_signals {
                Some(signals) => signals.len(),
                None => 0,
            };
            let equator_cross = match &signal.equator_cross_signals {
                Some(signals) => signals.len(),
                None => 0,
            };
            let signal_count = retrograde + planet_matrix + planet_self_alignment + equator_cross;

            // only print confluent signals
            if signal_count > 0 {
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    signal.eclipse.date.to_string(),
                    signal.eclipse.kind.to_rank(),
                    retrograde,
                    planet_matrix,
                    planet_self_alignment,
                    equator_cross
                );
            }
        }
        Ok(signals)
    }
}
