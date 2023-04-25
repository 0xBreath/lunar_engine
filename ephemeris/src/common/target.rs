
#[derive(Clone, Debug)]
pub struct Target {
  pub value: String,
}
#[allow(dead_code)]
impl Target {
  pub fn new(planet: &Planet) -> Self {
    Self {
      value: format!("&COMMAND='{}'", planet.to_earth_center()),
    }
  }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Planet {
  Moon,
  Sun,
  Mercury,
  Venus,
  Mars,
  Jupiter,
  Saturn,
  Uranus,
  Neptune,
  Pluto,
}

#[allow(dead_code)]
impl Planet {
  pub fn to_str(&self) -> &str {
    match self {
      Planet::Moon => "Moon",
      Planet::Sun => "Sun",
      Planet::Mercury => "Mercury",
      Planet::Venus => "Venus",
      Planet::Mars => "Mars",
      Planet::Jupiter => "Jupiter",
      Planet::Saturn => "Saturn",
      Planet::Uranus => "Uranus",
      Planet::Neptune => "Neptune",
      Planet::Pluto => "Pluto",
    }
  }
  /// Map planet to ordered integer
  pub fn to_num(&self) -> usize {
    match self {
      Planet::Moon => 0,
      Planet::Sun => 1,
      Planet::Mercury => 2,
      Planet::Venus => 3,
      Planet::Mars => 4,
      Planet::Jupiter => 5,
      Planet::Saturn => 6,
      Planet::Uranus => 7,
      Planet::Neptune => 8,
      Planet::Pluto => 9,
    }
  }

  pub fn to_vec() -> Vec<Planet> {
    vec![
      Planet::Moon,
      Planet::Sun,
      Planet::Mercury,
      Planet::Venus,
      Planet::Mars,
      Planet::Jupiter,
      Planet::Saturn,
      Planet::Uranus,
      Planet::Neptune,
      Planet::Pluto,
    ]
  }

  pub fn to_index(&self) -> usize {
    Self::to_vec().iter().position(|p| p == self).unwrap()
  }


  /// API mapping for object relative to earth center
  fn to_earth_center(&self) -> &str {
    match self {
      Planet::Moon => "301",
      Planet::Sun => "10",
      Planet::Mercury => "199",
      Planet::Venus => "299",
      Planet::Mars => "499",
      Planet::Jupiter => "599",
      Planet::Saturn => "699",
      Planet::Uranus => "799",
      Planet::Neptune => "899",
      Planet::Pluto => "999",
    }
  }
}

impl PartialEq for Planet {
  fn eq(&self, other: &Self) -> bool {
    self.to_str() == other.to_str()
  }
}

impl From<&str> for Planet {
  fn from(s: &str) -> Self {
    match s {
      "Moon" => Planet::Moon,
      "Sun" => Planet::Sun,
      "Mercury" => Planet::Mercury,
      "Venus" => Planet::Venus,
      "Mars" => Planet::Mars,
      "Jupiter" => Planet::Jupiter,
      "Saturn" => Planet::Saturn,
      "Uranus" => Planet::Uranus,
      "Neptune" => Planet::Neptune,
      "Pluto" => Planet::Pluto,
      _ => panic!("Invalid planet"),
    }
  }
}