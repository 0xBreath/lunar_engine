LOG_FILE="$PWD/plpl.log"

cargo build -r && cargo run -r -p plpl_binance LOG_FILE="$LOG_FILE"