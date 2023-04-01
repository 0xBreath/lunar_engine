use time_series::Time;
use crate::*;


#[derive(Clone, Debug)]
pub struct PlanetEquatorCrossTwoEclipses {
  pub planet: Planet,
  pub first_eclipse: EclipseEvent,
  pub second_eclipse: EclipseEvent,
}
impl PlanetEquatorCrossTwoEclipses {
  pub fn new(
    planet: Planet,
    first_eclipse: EclipseEvent,
    second_eclipse: EclipseEvent,
  ) -> Self {
    Self {
      planet,
      first_eclipse,
      second_eclipse,
    }
  }
}

#[derive(Clone, Debug)]
pub struct PlanetSelfAlignmentTwoEclipses {
  pub planet: Planet,
  pub first_eclipse: EclipseEvent,
  pub second_eclipse: EclipseEvent,
  pub alignment: Alignment,
}
impl PlanetSelfAlignmentTwoEclipses {
  pub fn new(
    planet: Planet,
    first_eclipse: EclipseEvent,
    second_eclipse: EclipseEvent,
    alignment: Alignment,
  ) -> Self {
    Self {
      planet,
      first_eclipse,
      second_eclipse,
      alignment,
    }
  }
}

#[derive(Clone, Debug)]
pub struct PlanetPairAlignmentOnEclipse {
  pub planet_pair: (Planet, Planet),
  pub alignment: Alignment,
  pub eclipse: EclipseEvent,
}
impl PlanetPairAlignmentOnEclipse {
  pub fn new(
    planet_a: Planet,
    planet_b: Planet,
    alignment: Alignment,
    eclipse: EclipseEvent,
  ) -> Self {
    Self {
      planet_pair: (planet_a, planet_b),
      alignment,
      eclipse,
    }
  }
}

#[derive(Clone, Debug)]
pub struct PlanetRetrogradeOnEclipse {
  pub planet: Planet,
  pub eclipse: EclipseEvent,
  pub retrograde: RetrogradeEvent,
}
impl PlanetRetrogradeOnEclipse {
  pub fn new(
    planet: Planet,
    eclipse: EclipseEvent,
    retrograde: RetrogradeEvent,
  ) -> Self {
    Self {
      planet,
      eclipse,
      retrograde,
    }
  }
}

#[derive(Debug, Clone)]
pub enum EclipseClass {
  Solar,
  Lunar,
}

#[derive(Debug, Clone)]
pub enum EclipseType {
  TotalSolar,
  AnnularSolar,
  PartialSolar,
  PenumbralLunar,
  PartialLunar,
  TotalLunar,
}

impl EclipseType {
  /// ## Lunar Eclipse Types
  /// N = penumbral eclipse: Moon traverses Earth's penumbra, but misses umbra.
  ///
  /// P = partial eclipse: Moon traverses Earth's penumbra and umbra, but does not enter umbra entirely.
  ///
  /// T = total eclipse: Moon passes completely into Earth's umbra.
  ///
  /// Read lunar eclipse CSV into Vec<(Time, EclipseType, EclipseRank)>
  ///
  /// ## Solar Eclipse Types
  /// P = partial eclipse: Earth traverses Moon’s penumbra, but misses umbra.
  ///
  /// A = annular eclipse: Earth enters Moon's antumbra, but does not completely cover the Sun.
  ///
  /// T = total eclipse: Earth completely enters Moon’s umbra.
  ///
  /// H = hybrid eclipse: Earth traverses Moon's umbra and antumbra, so annular and total eclipses are visible in different locations on Earth.
  ///
  pub fn from_symbol(symbol: &str, kind: EclipseClass) -> Self {
    match kind {
      EclipseClass::Lunar => {
        match symbol {
          "N" => EclipseType::PenumbralLunar,
          "Nx" => EclipseType::PenumbralLunar,
          "Ne" => EclipseType::PenumbralLunar,
          "Nb" => EclipseType::PenumbralLunar,
          "P" => EclipseType::PartialLunar,
          "T" => EclipseType::TotalLunar,
          "T+" => EclipseType::TotalLunar,
          "T-" => EclipseType::TotalLunar,
          _ => panic!("Invalid lunar eclipse symbol"),
        }
      },
      EclipseClass::Solar => {
        match symbol {
          "P" => EclipseType::PartialSolar,
          "Pe" => EclipseType::PartialSolar,
          "Pb" => EclipseType::PartialSolar,
          "A" => EclipseType::AnnularSolar,
          "A+" => EclipseType::AnnularSolar,
          "A-" => EclipseType::AnnularSolar,
          "Am" => EclipseType::AnnularSolar,
          "An" => EclipseType::AnnularSolar,
          "As" => EclipseType::AnnularSolar,
          "T" => EclipseType::TotalSolar,
          "T+" => EclipseType::TotalSolar,
          "T-" => EclipseType::TotalSolar,
          "Tm" => EclipseType::TotalSolar,
          "Tn" => EclipseType::TotalSolar,
          "Ts" => EclipseType::TotalSolar,
          "H" => EclipseType::TotalSolar,
          "Hm" => EclipseType::TotalSolar,
          "H2" => EclipseType::TotalSolar,
          "H3" => EclipseType::TotalSolar,
          _ => panic!("Invalid solar eclipse symbol"),
        }
      },
    }
  }

  /// ## Eclipse Importance Ranking Greatest To Least:
  ///
  /// 1) Total Solar Eclipse (TSE)
  ///
  /// 2) Annular Solar Eclipse (ASE) & Penumbral Lunar Eclipse (PenLE)
  ///
  /// 3) Partial Solar Eclipse (PSE) & Partial Lunar Eclipse (PLE)
  ///
  /// 4) Total Lunar Eclipse (TLE)
  pub fn to_rank(&self) -> u8 {
    match self {
      EclipseType::TotalSolar => 1,
      EclipseType::AnnularSolar => 2,
      EclipseType::PenumbralLunar => 2,
      EclipseType::PartialSolar => 3,
      EclipseType::PartialLunar => 3,
      EclipseType::TotalLunar => 4,
    }
  }
}

#[derive(Debug, Clone)]
pub struct EclipseEvent {
  pub date: Time,
  pub kind: EclipseType,
}

impl EclipseEvent {
  pub fn new(date: Time, kind: EclipseType) -> Self {
    Self {
      date,
      kind,
    }
  }
}

#[derive(Clone, Debug)]
pub struct EclipseSignals {
  pub eclipse: EclipseEvent,
  pub retrograde_signals: Option<Vec<PlanetRetrogradeOnEclipse>>,
  pub planet_pair_alignment_signals: Option<Vec<PlanetPairAlignmentOnEclipse>>,
  pub self_alignment_signals: Option<Vec<PlanetSelfAlignmentTwoEclipses>>,
  pub equator_cross_signals: Option<Vec<PlanetEquatorCrossTwoEclipses>>,
}