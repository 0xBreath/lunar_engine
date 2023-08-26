#[macro_export]
macro_rules! precise_round {
    ($num:expr, $decimals:expr) => {{
        let factor = 10.0_f64.powi($decimals);
        ($num * factor).round() / factor
    }};
}
