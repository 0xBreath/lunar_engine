
pub struct StepSize {
    pub value: String,
}

impl StepSize {
  pub fn default() -> Self {
    Self {
      value: String::from("&STEP_SIZE='1d'"),
    }
  }
}