
pub struct Quantities {
  pub value: String,
}

/// [Documentation](https://ssd.jpl.nasa.gov/horizons/manual.html#output)
impl Quantities {
  /// Astrometric right ascension and declination
  pub fn default() -> Self {
    Self {
      value: String::from("&QUANTITIES='1'"),
    }
  }
}