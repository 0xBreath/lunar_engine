LOG_FILE="$PWD/plpl.log"

cargo build -r && cargo run -r -p plpl LOG_FILE="$LOG_FILE"