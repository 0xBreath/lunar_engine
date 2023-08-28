# Lunar Engine

### Steps to deploy to virtual machine
```bash
# Download source code dependency
sudo apt install git

# Manage terminal process dependency
sudo apt install screen

# Install Rust
curl https://sh.rustup.rs -sSf | sh

# Cargo build dependencies
sudo apt install build-essential
sudo apt-get install -y libsasl2-dev
sudo apt install pkg-config
sudo apt-get install libfontconfig libfontconfig1-dev

# Set github remote
git remote add origin https://github.com/LunarEngine/lunar_engine.git
git reset --hard origin/main
git pull origin main

# Move to a screen to run the algorithm
screen -R plpl

# Start the algorithm on Binance testnet
TESTNET=true cargo run -r -p plpl

# Exit screen with Ctrl+A then D

# Print logs on the main screen
cat plpl.log
# Follow logs on the main screen
tail -f plpl.log

# To reenter the screen
screen -r plpl
```