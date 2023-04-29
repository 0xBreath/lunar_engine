source ./scripts/plpl/config.txt

export YEAR="$YEAR"
export MONTH="$MONTH"
export DAY="$DAY"
export PATH_TO_DIR="$PWD"
export PLANET="$PLANET"
export PLPL_SCALE="$PLPL_SCALE"
export CROSS_MARGIN_PCT="$CROSS_MARGIN_PCT"
export TRAILING_STOP_USE_PCT="$TRAILING_STOP_USE_PCT"
export TRAILING_STOP="$TRAILING_STOP"
export STOP_LOSS_PCT="$STOP_LOSS_PCT"

cargo run -r -p plpl