source ./scripts/pfs_days/config.txt

export START_YEAR="$START_YEAR"
export START_MONTH="$START_MONTH"
export START_DAY="$START_DAY"
export END_YEAR="$END_YEAR"
export END_MONTH="$END_MONTH"
export END_DAY="$END_DAY"
export PATH_TO_DIR="$PWD"

cargo run -r -p pfs_days