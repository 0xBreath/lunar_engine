
#[derive(Clone, Debug, Copy)]
pub enum Origin {
  Geocentric,
  Heliocentric
}

impl Origin {
  pub fn as_query(&self) -> &str {
    match self {
      Origin::Geocentric => "&CENTER='500@399'",
      Origin::Heliocentric => "&CENTER='500@sun'",
    }
  }
}