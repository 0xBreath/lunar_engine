use crate::*;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use time_series::*;

pub type Matrix = Vec<(Planet, Planet, Vec<(Time, f32, Alignment)>)>;
pub type ConfluentMatrix = Vec<(Time, Vec<PlanetPairAlignment>)>;
pub type FilteredMatrix = Vec<PlanetPairAlignment>;

#[derive(Debug)]
pub enum PlanetMatrixError {
    TimeError(TimeError),
}

impl std::fmt::Display for PlanetMatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanetMatrixError::TimeError(e) => write!(f, "Time error: {}", e),
        }
    }
}

pub type PlanetMatrixResult<T> = Result<T, PlanetMatrixError>;

#[derive(Debug, Clone)]
pub struct PlanetPairAlignment {
    pub planet_pair: (Planet, Planet),
    pub alignment: Alignment,
    pub date: Time,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanetPairAlignmentWinRate {
    pub planet_1: Planet,
    pub planet_2: Planet,
    pub alignment: Alignment,
    pub alignment_total_count: u32,
    pub win_count: u32,
}

#[derive(Debug, Clone)]
pub struct PlanetMatrix {
    /// Vector of harmonic angles between two planets for a period of time.
    /// Compares all combinations of planets; a matrix of planetary alignments.
    pub matrix: Matrix,
    /// The start date of the planet positions
    pub start_date: Time,
    /// The number of days +/- start_time to query for planet positions.
    pub end_date: Time,
}
impl PlanetMatrix {
    /// Compare geocentric right ascension of two planets.
    /// Compare each planet to all other planets (matrix).
    pub async fn new(
        origin: Origin,
        start_time: &Time,
        end_time: &Time,
        alignment_margin_error: f32,
        planets: &[Planet],
        harmonics: &[Alignment],
    ) -> PlanetMatrixResult<Self> {
        let _diff_days = start_time
            .diff_days(end_time)
            .map_err(PlanetMatrixError::TimeError)?;
        let mut matrix: Matrix = Vec::new();

        let mut planet_alignments = Vec::new();
        for planet in planets.iter() {
            planet_alignments.push(
                Query::query(
                    origin,
                    planet,
                    DataType::RightAscension,
                    *start_time,
                    *end_time,
                )
                .await
                .expect("failed to query planet angles"),
            );
        }

        for (index, planet_a_alignments) in planet_alignments.iter().enumerate() {
            for planet_b_index in (index + 1)..planet_alignments.len() {
                let planet_a = &planets[index];
                let planet_b = &planets[planet_b_index];
                let planet_b_alignments = planet_alignments[planet_b_index].clone();

                let mut vec: Vec<(Time, f32, Alignment)> = Vec::new();
                for ((time, planet_a_ra), (_, planet_b_ra)) in
                    planet_a_alignments.iter().zip(planet_b_alignments.iter())
                {
                    let angle = (planet_a_ra - planet_b_ra).abs();
                    let alignment = Alignment::find_alignment(
                        *planet_a_ra,
                        *planet_b_ra,
                        alignment_margin_error,
                    );
                    if let Some(alignment) = alignment {
                        if harmonics.contains(&alignment) {
                            vec.push((*time, angle, alignment));
                        }
                    }
                }
                vec = Query::remove_duplicate_values(&mut vec);
                vec.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                matrix.push((planet_a.clone(), planet_b.clone(), vec));
            }
        }
        Ok(Self {
            matrix,
            start_date: *start_time,
            end_date: *end_time,
        })
    }

    /// Search for all alignments on a given date.
    pub fn alignments_on_date(&self, date: &Time) -> Vec<PlanetPairAlignment> {
        let mut alignments = Vec::new();
        for (planet_a, planet_b, vec) in self.matrix.iter() {
            for (time, _, alignment) in vec.iter() {
                if time == date {
                    alignments.push(PlanetPairAlignment {
                        planet_pair: (planet_a.clone(), planet_b.clone()),
                        alignment: alignment.clone(),
                        date: *time,
                    });
                }
            }
        }
        alignments
    }

    pub fn filter_matrix(
        &self,
        _error_margin_days: u64,
        planet_filter: Vec<Planet>,
        alignment_filter: Vec<Alignment>,
    ) -> Vec<PlanetPairAlignment> {
        let mut filtered_matrix = Vec::<PlanetPairAlignment>::new();
        for (planet_a, planet_b, alignments) in self.matrix.iter() {
            if planet_filter.contains(planet_a) && planet_filter.contains(planet_b) {
                for (time, _, alignment) in alignments.iter() {
                    if alignment_filter.contains(alignment) {
                        filtered_matrix.push(PlanetPairAlignment {
                            planet_pair: (planet_a.clone(), planet_b.clone()),
                            alignment: alignment.clone(),
                            date: *time,
                        });
                    }
                }
            }
        }
        filtered_matrix
    }

    pub fn confluent_matrix(
        &self,
        error_margin_days: u64,
        confluence_requirement: usize,
    ) -> PlanetMatrixResult<ConfluentMatrix> {
        // guaranteed to be >0 as an i64 because of check in Self::new()
        let period = self
            .start_date
            .diff_days(&self.end_date)
            .map_err(PlanetMatrixError::TimeError)? as u64;

        let mut all_alignments: ConfluentMatrix = Vec::new();
        for index in 0..period {
            if index < error_margin_days || index > period - error_margin_days {
                continue;
            }
            let date = self.start_date.delta_date(index as i64);
            let mut date_alignments = Vec::<PlanetPairAlignment>::new();
            for range_index in (index - error_margin_days)..(index + error_margin_days + 1) {
                let range_date = self.start_date.delta_date(range_index as i64);
                let range_alignments = self.alignments_on_date(&range_date);
                date_alignments.extend(range_alignments);
            }
            if date_alignments.len() >= confluence_requirement {
                all_alignments.push((date, date_alignments));
            }
        }
        Ok(all_alignments)
    }

    pub fn print_confluent_matrix(&self, confluent_matrix: ConfluentMatrix) {
        for (date, confluent_alignments) in confluent_matrix.iter() {
            println!("{}\t{:?}", date.to_string(), confluent_alignments.len());
        }
    }

    pub fn print_filtered_matrix(&self, filtered_matrix: FilteredMatrix) {
        for ppa in filtered_matrix.iter() {
            println!(
                "{}-{}\t{}\t{:?}",
                ppa.planet_pair.0.to_str(),
                ppa.planet_pair.1.to_str(),
                ppa.date.to_string(),
                ppa.alignment.to_str()
            );
        }
    }

    /// Find the total count of each planet pair alignment
    pub fn build_planet_pair_alignment_counts(&self) -> Vec<PlanetPairAlignmentWinRate> {
        let mut vec = Vec::<PlanetPairAlignmentWinRate>::new();
        for (planet_a, planet_b, alignments) in self.matrix.iter() {
            // index follows `Alignment` enum order
            let mut alignment_counts = vec![0; 21];

            for data in alignments.iter() {
                let alignment = &data.2;
                match alignment {
                    Alignment::Conjunct => alignment_counts[0] += 1,
                    Alignment::Opposite => alignment_counts[1] += 1,
                    Alignment::Trine120 => alignment_counts[2] += 1,
                    Alignment::Trine240 => alignment_counts[3] += 1,
                    Alignment::Square90 => alignment_counts[4] += 1,
                    Alignment::Square270 => alignment_counts[5] += 1,
                    Alignment::Quintile72 => alignment_counts[6] += 1,
                    Alignment::Quintile144 => alignment_counts[7] += 1,
                    Alignment::Quintile216 => alignment_counts[8] += 1,
                    Alignment::Quintile288 => alignment_counts[9] += 1,
                    Alignment::Sextile60 => alignment_counts[10] += 1,
                    Alignment::Sextile300 => alignment_counts[11] += 1,
                    Alignment::Septile51 => alignment_counts[12] += 1,
                    Alignment::Septile102 => alignment_counts[13] += 1,
                    Alignment::Septile154 => alignment_counts[14] += 1,
                    Alignment::Septile205 => alignment_counts[15] += 1,
                    Alignment::Septile257 => alignment_counts[16] += 1,
                    Alignment::Septile308 => alignment_counts[17] += 1,
                    Alignment::Octile45 => alignment_counts[18] += 1,
                    Alignment::Octile135 => alignment_counts[19] += 1,
                    Alignment::Octile225 => alignment_counts[20] += 1,
                    Alignment::Octile315 => alignment_counts[21] += 1,
                }
            }
            for (index, alignment) in Alignment::to_vec().iter().enumerate() {
                vec.push(PlanetPairAlignmentWinRate {
                    planet_1: planet_a.clone(),
                    planet_2: planet_b.clone(),
                    alignment: alignment.clone(),
                    alignment_total_count: alignment_counts[index],
                    win_count: 0,
                });
            }
        }
        vec
    }

    /// Print to a file all planet pair alignments for the time period.
    pub fn print_alignments(&self, results_file: &PathBuf) {
        let mut file = File::create(results_file).unwrap();
        println!("\t\t### PLANET MATRIX ###\t\t");
        writeln!(file, "\t\t### PLANET MATRIX ###\t\t")
            .expect("failed to write planet alignment to file");
        println!(
            "Planet alignments from {} to {}\n",
            self.start_date.to_string(),
            self.end_date.to_string()
        );
        writeln!(
            file,
            "Planet alignments from {} to {}\n",
            self.start_date.to_string(),
            self.end_date.to_string()
        )
        .expect("failed to write planet alignment to file");
        for (planet_a, planet_b, alignments) in self.matrix.iter() {
            for data in alignments.iter() {
                let (time, _, alignment) = data;
                println!(
                    "{:?}-{:?}\t{:?}\t{:?}",
                    planet_a,
                    planet_b,
                    time.to_string(),
                    alignment.to_str(),
                );
                writeln!(
                    file,
                    "{:?}-{:?}\t{:?}\t{:?}",
                    planet_a,
                    planet_b,
                    time.to_string(),
                    alignment.to_str(),
                )
                .expect("failed to write planet alignment to file");
            }
        }
    }

    /// Search for a `PlanetPairAlignmentWinRate` by two `Planet` and their `Alignment`
    pub fn get_planet_pair_alignment_win_rate(
        alignment_counts: &[PlanetPairAlignmentWinRate],
        planet_a: &Planet,
        planet_b: &Planet,
        alignment: &Alignment,
    ) -> Option<PlanetPairAlignmentWinRate> {
        let mut value: Option<PlanetPairAlignmentWinRate> = None;
        for data in alignment_counts.iter() {
            if data.planet_1.eq(planet_a)
                && data.planet_2.eq(planet_b)
                && data.alignment.eq(alignment)
            {
                value = Some(data.clone());
                break;
            }
        }
        value
    }

    /// Compare each planet to one other planet for all possible harmonics.
    /// Compare all possibilities of planet pairs as a "matrix" of possibilities.
    /// For each planet pair in the matrix, find if the date (+/- a margin of days) equals a known reversal date (analyzed from existing price data).
    /// A "win" is considered if the alignment occurred on the same day as a known reversal.
    /// Compute the win rate of each planet pair. How likely could the planet pair predict a reversal? (across all harmonics)
    /// Win rate is across all harmonics for a given planet pair.
    pub async fn test_planet_matrix(
        ticker_data_path: &PathBuf,
        margin_of_error_days: u32,
        alignment_margin_error: f32,
        candle_range: usize,
        planets: &[Planet],
        harmonics: &[Alignment],
    ) {
        let mut ticker_data = TickerData::new();
        ticker_data
            .add_csv_series(ticker_data_path)
            .expect("Failed to add CSV to TickerData");
        let reversals = ticker_data.find_reversals(candle_range);
        if ticker_data.candles.is_empty() {
            return;
        }
        let earliest_candle_date = &ticker_data.get_candles()[0].date;
        let latest_candle_date =
            &ticker_data.get_candles()[ticker_data.get_candles().len() - 1].date;
        let _max_history_days = Time::today().diff_days(earliest_candle_date);

        let planet_matrix = PlanetMatrix::new(
            Origin::Geocentric,
            earliest_candle_date,
            latest_candle_date,
            alignment_margin_error,
            planets,
            harmonics,
        )
        .await
        .unwrap();
        println!("PLANET PAIR\tALIGNMENT\tWIN RATE\tWIN EVENTS\tTOTAL EVENTS");

        let alignment_counts = planet_matrix.build_planet_pair_alignment_counts();

        for (planet_a, planet_b, alignments) in planet_matrix.matrix.into_iter() {
            for (time, _ra, alignment) in alignments.iter() {
                let planet_pair_alignment_win_rate: Option<PlanetPairAlignmentWinRate> =
                    PlanetMatrix::get_planet_pair_alignment_win_rate(
                        &alignment_counts,
                        &planet_a,
                        &planet_b,
                        alignment,
                    );

                if let Some(mut ppawn) = planet_pair_alignment_win_rate {
                    // backtest planet-pair-alignment signal against known reversals
                    for reversal in reversals.iter() {
                        let range_start = time.delta_date(-(margin_of_error_days as i64));
                        let range_end = time.delta_date(margin_of_error_days as i64);
                        if reversal.candle.date.within_range(range_start, range_end) {
                            // increment the win count for this planet pair alignment
                            ppawn.win_count += 1;
                            break;
                        }
                    }
                }
            }
        }
        // print result for each PlanetPairAlignmentCount
        for ppawn in alignment_counts.iter() {
            let win_rate =
                ((ppawn.win_count as f32 / ppawn.alignment_total_count as f32) * 100.0).round();
            if win_rate.is_nan() {
                println!(
                    "{}-{}\t{}\t-\t\t{}\t\t{}",
                    ppawn.planet_1.to_str(),
                    ppawn.planet_2.to_str(),
                    ppawn.alignment.to_str(),
                    ppawn.win_count,
                    ppawn.alignment_total_count
                );
            } else {
                println!(
                    "{}-{}\t{}\t{}%\t\t{}\t\t{}",
                    ppawn.planet_1.to_str(),
                    ppawn.planet_2.to_str(),
                    ppawn.alignment.to_str(),
                    win_rate,
                    ppawn.win_count,
                    ppawn.alignment_total_count
                );
            }
        }
    }
}
