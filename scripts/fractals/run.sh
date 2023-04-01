source ./scripts/fractals/config.txt

export LEFT_BARS="$LEFT_BARS"
export RIGHT_BARS="$RIGHT_BARS"
export PIVOTS_BACK="$PIVOTS_BACK"
export USE_TIME="$USE_TIME"
export NUM_COMPARE="$NUM_COMPARE"
export PATH_TO_DIR="$PATH_TO_DIR"

cargo run -r -p fractals

