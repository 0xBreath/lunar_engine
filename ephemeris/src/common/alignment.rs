
#[derive(Clone, Debug)]
pub enum Reversal {
  RangeHigh,
  RangeLow
}

#[derive(Clone, Debug)]
pub struct ReversalInfo {
  /// Start of movement at range high or low?
  pub reversal_type: Reversal,
  /// Angle at start of movement
  pub reversal_angle: f32,
  /// Alignment to search for
  pub alignment: Alignment,
}

#[derive(Clone, Debug)]
pub enum Alignment {
  Conjunct,
  Opposite,
  Trine120,
  Trine240,
  Square90,
  Square270,
  Quintile72,
  Quintile144,
  Quintile216,
  Quintile288,
  Sextile60,
  Sextile300,
  Septile51,
  Septile102,
  Septile154,
  Septile205,
  Septile257,
  Septile308,
  Octile45,
  Octile135,
  Octile225,
  Octile315,
}

impl Alignment {
  pub fn to_str(&self) -> &'static str {
    match *self {
      Alignment::Conjunct => "Conjunct",
      Alignment::Opposite => "Opposite",
      Alignment::Trine120 => "Trine120",
      Alignment::Trine240 => "Trine240",
      Alignment::Square90 => "Square90",
      Alignment::Square270 => "Square270",
      Alignment::Quintile72 => "Quintile72",
      Alignment::Quintile144 => "Quintile144",
      Alignment::Quintile216 => "Quintile216",
      Alignment::Quintile288 => "Quintile288",
      Alignment::Sextile60 => "Sextile60",
      Alignment::Sextile300 => "Sextile300",
      Alignment::Septile51 => "Septile51",
      Alignment::Septile102 => "Septile102",
      Alignment::Septile154 => "Septile154",
      Alignment::Septile205 => "Septile205",
      Alignment::Septile257 => "Septile257",
      Alignment::Septile308 => "Septile308",
      Alignment::Octile45 => "Octile45",
      Alignment::Octile135 => "Octile135",
      Alignment::Octile225 => "Octile225",
      Alignment::Octile315 => "Octile315",
    }
  }
  
  pub fn to_num(&self) -> f32 {
    match *self {
      Alignment::Conjunct => 0.0,
      Alignment::Opposite => 180.0,
      Alignment::Trine120 => 120.0,
      Alignment::Trine240 => 240.0,
      Alignment::Square90 => 90.0,
      Alignment::Square270 => 270.0,
      Alignment::Quintile72 => 72.0,
      Alignment::Quintile144 => 144.0,
      Alignment::Quintile216 => 216.0,
      Alignment::Quintile288 => 288.0,
      Alignment::Sextile60 => 60.0,
      Alignment::Sextile300 => 300.0,
      Alignment::Septile51 => 51.42857143,
      Alignment::Septile102 => 102.8571429,
      Alignment::Septile154 => 154.2857143,
      Alignment::Septile205 => 205.7142857,
      Alignment::Septile257 => 257.1428571,
      Alignment::Septile308 => 308.5714286,
      Alignment::Octile45 => 45.0,
      Alignment::Octile135 => 135.0,
      Alignment::Octile225 => 225.0,
      Alignment::Octile315 => 315.0,
    }
  }

  pub fn normalize(angle: f32) -> f32 {
    if angle < 0.0 {
      angle + 360.0
    } else if angle > 360.0 {
      angle - 360.0
    } else {
      angle
    }
  }

  pub fn find_alignment(a: f32, b: f32, margin: f32) -> Option<Self> {
    let diff = Alignment::normalize(a - b);
    if diff < margin {
      Some(Alignment::Conjunct)
    }
    else if Alignment::normalize(diff - Alignment::Opposite.to_num()) < margin {
      Some(Alignment::Opposite)
    }
    else if Alignment::normalize(diff - Alignment::Trine120.to_num()) < margin {
      Some(Alignment::Trine120)
    }
    else if Alignment::normalize(diff - Alignment::Trine240.to_num()) < margin {
      Some(Alignment::Trine240)
    }
    else if Alignment::normalize(diff - Alignment::Square90.to_num()) < margin {
      Some(Alignment::Square90)
    }
    else if Alignment::normalize(diff - Alignment::Square270.to_num()) < margin {
      Some(Alignment::Square270)
    }
    else if Alignment::normalize(diff - Alignment::Quintile72.to_num()) < margin {
      Some(Alignment::Quintile72)
    }
    else if Alignment::normalize(diff - Alignment::Quintile144.to_num()) < margin {
      Some(Alignment::Quintile144)
    }
    else if Alignment::normalize(diff - Alignment::Quintile216.to_num()) < margin {
      Some(Alignment::Quintile216)
    }
    else if Alignment::normalize(diff - Alignment::Quintile288.to_num()) < margin {
      Some(Alignment::Quintile288)
    }
    else if Alignment::normalize(diff - Alignment::Sextile60.to_num()) < margin {
      Some(Alignment::Sextile60)
    }
    else if Alignment::normalize(diff - Alignment::Sextile300.to_num()) < margin {
       Some(Alignment::Sextile300)
    }
    else if Alignment::normalize(diff - Alignment::Septile51.to_num()) < margin {
      Some(Alignment::Septile51)
    }
    else if Alignment::normalize(diff - Alignment::Septile102.to_num()) < margin {
      Some(Alignment::Septile102)
    }
    else if Alignment::normalize(diff - Alignment::Septile154.to_num()) < margin {
      Some(Alignment::Septile154)
    }
    else if Alignment::normalize(diff - Alignment::Septile205.to_num()) < margin {
      Some(Alignment::Septile205)
    }
    else if Alignment::normalize(diff - Alignment::Septile257.to_num()) < margin {
      Some(Alignment::Septile257)
    }
    else if Alignment::normalize(diff - Alignment::Septile308.to_num()) < margin {
      Some(Alignment::Septile308)
    }
    else if Alignment::normalize(diff - Alignment::Octile45.to_num()) < margin {
      Some(Alignment::Octile45)
    }
    else if Alignment::normalize(diff - Alignment::Octile135.to_num()) < margin {
      Some(Alignment::Octile135)
    }
    else if Alignment::normalize(diff - Alignment::Octile225.to_num()) < margin {
      Some(Alignment::Octile225)
    }
    else if Alignment::normalize(diff - Alignment::Octile315.to_num()) < margin {
      Some(Alignment::Octile315)
    }
    else {
      None
    }
  }

  pub fn to_vec() -> Vec<Alignment> {
    vec![
      Alignment::Conjunct,
      Alignment::Opposite,
      Alignment::Trine120,
      Alignment::Trine240,
      Alignment::Square90,
      Alignment::Square270,
      Alignment::Quintile72,
      Alignment::Quintile144,
      Alignment::Quintile216,
      Alignment::Quintile288,
      Alignment::Sextile60,
      Alignment::Sextile300,
      Alignment::Septile51,
      Alignment::Septile102,
      Alignment::Septile154,
      Alignment::Septile205,
      Alignment::Septile257,
      Alignment::Septile308,
      Alignment::Octile45,
      Alignment::Octile135,
      Alignment::Octile225,
      Alignment::Octile315,
    ]
  }

  pub fn trine() -> Vec<Alignment> {
    vec![
      Alignment::Trine120,
      Alignment::Trine240
    ]
  }

  pub fn square() -> Vec<Alignment> {
    vec![
      Alignment::Square90,
      Alignment::Square270
    ]
  }

  pub fn quintile() -> Vec<Alignment> {
    vec![
      Alignment::Quintile72,
      Alignment::Quintile144,
      Alignment::Quintile216,
      Alignment::Quintile288
    ]
  }

  pub fn sextile() -> Vec<Alignment> {
    vec![
      Alignment::Sextile60,
      Alignment::Sextile300
    ]
  }

  pub fn octile() -> Vec<Alignment> {
    vec![
      Alignment::Octile45,
      Alignment::Octile135,
      Alignment::Octile225,
      Alignment::Octile315
    ]
  }
}

impl PartialEq<Self> for Alignment {
  fn eq(&self, other: &Self) -> bool {
    self.to_str() == other.to_str()
  }
}
