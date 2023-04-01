source ./scripts/hda/config.txt

export LEFT_BARS="$LEFT_BARS"
export RIGHT_BARS="$RIGHT_BARS"
export HDA_MARGIN="$HDA_MARGIN"
export START_YEAR="$START_YEAR"
export START_MONTH="$START_MONTH"
export START_DAY="$START_DAY"
export END_YEAR="$END_YEAR"
export END_MONTH="$END_MONTH"
export END_DAY="$END_DAY"
export PATH_TO_DIR="$PATH_TO_DIR"

cargo run -r -p hda