#[macro_export]
macro_rules! precise_round {
    ($num:expr, $decimals:expr) => {{
        let factor = 10.0_f64.powi($decimals);
        ($num * factor).round() / factor
    }};
}

#[macro_export]
macro_rules! f64_to_num {
    ($num:expr) => {{
        let decimals = $num.to_string().split('.').last().unwrap().len();
        let denominator = 10_f64.powi(decimals as i32);
        let numerator = $num * denominator;
        Num::new(numerator as i64, denominator as u64)
    }};
}
