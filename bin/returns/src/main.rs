use log::*;
use simplelog::{ColorChoice, Config as SimpleLogConfig, TermLogger, TerminalMode};

fn init_logger() {
    TermLogger::init(
        LevelFilter::Info,
        SimpleLogConfig::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");
}

/// Each trade uses trade_capital + profits from previous trades (compounded profits)
/// This leads to astronomical returns, with risk increasing but isolated to assumulated profits
fn compounded_position_profits(
    trade_avg_profit_pct: f64,
    num_trades: u32,
    trade_capital: f64,
) -> f64 {
    // PnL per trade
    let trade_pct_return = 1.0 + trade_avg_profit_pct / 100.0;
    let mut capital = trade_capital;
    for _ in 0..num_trades {
        let trade_profit = (trade_pct_return * capital) - capital;
        capital += trade_profit;
    }
    capital - trade_capital
}

/// Each trade uses initial trade_capital (constant profits)
/// This leads to lower risk over time
fn static_position_profits(trade_avg_profit_pct: f64, num_trades: u32, trade_capital: f64) -> f64 {
    // PnL per trade
    let trade_pct_return = 1.0 + trade_avg_profit_pct / 100.0;
    // all trades use constant trade_capital
    let trade_profit = (trade_pct_return * trade_capital) - trade_capital;
    // PnL for all trades
    trade_profit * num_trades as f64
}

fn trade_capital(initial_capital: f64, risk_pct: f64) -> f64 {
    initial_capital * (risk_pct / 100.0)
}

fn pct_return(cash_return: f64, risk: f64) -> f64 {
    cash_return / risk * 100.0
}

fn compounded_returns(return_per_trade: f64, num_trades: u32, initial_capital: f64, risk_pct: f64) {
    info!("Compounded Returns");
    info!("Each trade uses initial capital + profits from previous trades (compounded profits)");

    let initial_trade_capital = trade_capital(initial_capital, risk_pct);
    let y1_cash_return =
        compounded_position_profits(return_per_trade, num_trades, initial_trade_capital);
    let y1_pct_return = pct_return(y1_cash_return, initial_capital);
    info!("Year 1 Compounded Return: {:.2}%", y1_pct_return);
    let y1_capital = y1_cash_return + initial_capital;
    info!("Total Capital: ${:.2}", y1_capital);

    let y2_trade_capital = trade_capital(y1_capital, risk_pct);
    let y2_cash_return =
        compounded_position_profits(return_per_trade, num_trades, y2_trade_capital);
    let y2_pct_return = pct_return(y2_cash_return, initial_capital);
    info!("Year 2 Compounded Return: {:.2}%", y2_pct_return);
    let y2_capital = y2_cash_return + y1_capital;
    info!("Total Capital: ${:.2}", y2_capital);

    let y3_trade_capital = trade_capital(y2_capital, risk_pct);
    let y3_cash_return =
        compounded_position_profits(return_per_trade, num_trades, y3_trade_capital);
    let y3_pct_return = pct_return(y3_cash_return, initial_capital);
    info!("Year 3 Compounded Return: {:.2}%", y3_pct_return);
    let y3_capital = y3_cash_return + y2_capital;
    info!("Total Capital: ${:.2}", y3_capital);

    let y4_trade_capital = trade_capital(y3_capital, risk_pct);
    let y4_cash_return =
        compounded_position_profits(return_per_trade, num_trades, y4_trade_capital);
    let y4_pct_return = pct_return(y4_cash_return, initial_capital);
    info!("Year 4 Compounded Return: {:.2}%", y4_pct_return);
    let y4_capital = y4_cash_return + y3_capital;
    info!("Total Capital: ${:.2}", y4_capital);
}

fn static_returns(return_per_trade: f64, num_trades: u32, initial_capital: f64, risk_pct: f64) {
    info!("Static Returns");
    info!("For each year of trading, it uses the same trade capital");

    let initial_trade_capital = trade_capital(initial_capital, risk_pct);
    let y1_cash_return =
        static_position_profits(return_per_trade, num_trades, initial_trade_capital);
    let y1_pct_return = pct_return(y1_cash_return, initial_capital);
    info!("Year 1 Static Return: {:.2}%", y1_pct_return);
    let y1_capital = y1_cash_return + initial_capital;
    info!("Total Capital: ${:.2}", y1_capital);

    let y2_trade_capital = trade_capital(y1_capital, risk_pct);
    let y2_cash_return = static_position_profits(return_per_trade, num_trades, y2_trade_capital);
    let y2_pct_return = pct_return(y2_cash_return, initial_capital);
    info!("Year 2 Static Return: {:.2}%", y2_pct_return);
    let y2_capital = y2_cash_return + y1_capital;
    info!("Total Capital: ${:.2}", y2_capital);

    let y3_trade_capital = trade_capital(y2_capital, risk_pct);
    let y3_cash_return = static_position_profits(return_per_trade, num_trades, y3_trade_capital);
    let y3_pct_return = pct_return(y3_cash_return, initial_capital);
    info!("Year 3 Static Return: {:.2}%", y3_pct_return);
    let y3_capital = y3_cash_return + y2_capital;
    info!("Total Capital: ${:.2}", y3_capital);

    let y4_trade_capital = trade_capital(y3_capital, risk_pct);
    let y4_cash_return = static_position_profits(return_per_trade, num_trades, y4_trade_capital);
    let y4_pct_return = pct_return(y4_cash_return, initial_capital);
    info!("Year 4 Static Return: {:.2}%", y4_pct_return);
    let y4_capital = y4_cash_return + y3_capital;
    info!("Total Capital: ${:.2}", y4_capital);
}

fn main() {
    init_logger();

    let return_per_trade = 0.02; // % return per trade
    let num_trades = 42_852; // number of trades per year
    let initial_capital = 10_000.0; // $ initial capital
    let risk_pct = 10.0; // % risk of capital per trade

    compounded_returns(return_per_trade, num_trades, initial_capital, risk_pct);
    info!("========================================================");
    static_returns(return_per_trade, num_trades, initial_capital, risk_pct);
}
