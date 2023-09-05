use log::*;
use rand::Rng;
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
fn compounded_win_loss_return(
    win_rate_pct: f64,
    avg_win_pct: f64,
    avg_loss_pct: f64,
    num_trades: u32,
    trade_capital: f64,
) -> f64 {
    let trades = simulated_trades(win_rate_pct, num_trades);

    let win_pct_return = 1.0 + avg_win_pct / 100.0;
    let loss_pct_return = 1.0 - avg_loss_pct / 100.0;
    let mut capital = trade_capital;
    for trade in trades.into_iter() {
        match trade {
            TradeResult::Win => {
                let trade_profit = (win_pct_return * capital) - capital;
                capital += trade_profit;
            }
            TradeResult::Loss => {
                let trade_profit = (loss_pct_return * capital) - capital;
                capital += trade_profit;
            }
        }
    }
    (capital - trade_capital) / trade_capital * 100.0
}

enum TradeResult {
    Win,
    Loss,
}

fn simulated_trades(win_rate_pct: f64, num_trades: u32) -> Vec<TradeResult> {
    // make a vector of booleans that are evenly distributed according to the parameter percentage of "win_pct"
    // true = win, false = loss
    let mut trades: Vec<TradeResult> = Vec::new();
    let mut rng = rand::thread_rng();
    let win_rate = win_rate_pct / 100.0;
    for _ in 0..num_trades {
        // [0, 1)
        let r: f64 = rng.gen();
        if r < win_rate {
            trades.push(TradeResult::Win);
        } else {
            trades.push(TradeResult::Loss);
        }
    }
    trades
}

fn main() {
    init_logger();

    let win_rate_pct = std::env::var("WIN_RATE")
        .unwrap_or_else(|_| "66.0".to_string())
        .parse::<f64>()
        .expect("Failed to parse WIN_RATE_PCT");

    let win_pct_return = std::env::var("WIN")
        .unwrap_or_else(|_| "0.0134".to_string())
        .parse::<f64>()
        .expect("Failed to parse WIN");

    let loss_pct_return = std::env::var("LOSS")
        .unwrap_or_else(|_| "0.05".to_string())
        .parse::<f64>()
        .expect("Failed to parse LOSS");

    let trade_capital = 4_320.0; // trade every 10 minutes for a month
    let num_trades = 10_000;

    let returned = compounded_win_loss_return(
        win_rate_pct,
        win_pct_return,
        loss_pct_return,
        num_trades,
        trade_capital,
    );
    println!("Win Rate: {:.1}%", win_rate_pct);
    println!("Avg Win: {:.5}%", win_pct_return);
    println!("Avg Loss: {:.5}%", loss_pct_return);
    println!("Total Return: {:.2}%", returned);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulate_trade_returns() {
        let trade_capital = 1_000.0;
        let win_pct_return = 0.03;
        let loss_pct_return = 0.05;
        let num_trades = 10_000;
        let win_rate_pct = 66.0;

        let returned = compounded_win_loss_return(
            win_rate_pct,
            win_pct_return,
            loss_pct_return,
            num_trades,
            trade_capital,
        );
        println!("Total Return: {:.2}%", returned);
    }
}
