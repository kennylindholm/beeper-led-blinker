# Beeper LED Blinker

Blinks your Caps Lock LED when you have unread Beeper messages because it's cool?

Built for a Lenovo Z16 Gen 2 currently running on Fedora 42, but should work on any Linux machine where you can write to `/sys/class/leds/`.

An LLM helped write most of this code because apparently I needed AI assistance to make an LED go blink-blink. The future is weird.

## Why?

Because getting a notification on your phone, computer, smartwatch, and smart fridge wasn't enough. Now your keyboard can judge you for having unread messages too.

Also, blinking LEDs are scientifically proven to make everything cooler.

## What it does

- Polls Beeper Desktop's API for unread messages
- Filters by age (default: last 7 days) and ignores archived chats
- Blinks your Caps Lock LED when you have unread messages
- Reconnects automatically if Beeper Desktop restarts
- Runs as a systemd user service
- Written in Rust because I wanted a single binary with minimal overhead

## Requirements

- Rust (for building)
- Beeper Desktop v4.1.169+ with API enabled
- Sudo permissions to write to LED sysfs paths (see sudoers setup below)
- A machine with LEDs controllable via `/sys/class/leds/` (tested on Lenovo Z16 Gen 2)

### Setup

1. **Enable Beeper Desktop API**:
   - Settings → Developers → Enable "Beeper Desktop API"
   - Generate a Bearer token

2. **Configure sudo for LED control** (so you don't need to run the whole thing as root):
   ```bash
   sudo cp sudoers-beeper-blinker /etc/sudoers.d/beeper-blinker
   sudo chmod 440 /etc/sudoers.d/beeper-blinker
   ```

## Building & Running

```bash
# Build it
cargo build --release

# Run it
./target/release/beeper-led-blinker --token YOUR_TOKEN
```

### Install as a systemd service

The easy way:

```bash
# Option 1: Set token via environment variable
export BEEPER_API_TOKEN="your-token-here"
./buildandinstall.sh

# Option 2: Let the script prompt you
./buildandinstall.sh
```

Or manually:

```bash
systemctl --user link $(pwd)/beeper-led-blinker.service
systemctl --user enable beeper-led-blinker.service
systemctl --user start beeper-led-blinker.service
```

## Configuration

### CLI Options

```
--token <TOKEN>                    Beeper API token (required)
--led-path <LED_PATH>              LED device path [default: /sys/class/leds/input3::capslock/brightness]
--api-url <API_URL>                API base URL [default: http://localhost:23373]
--interval <INTERVAL>              Check interval in seconds [default: 5]
--blink-interval <BLINK_INTERVAL>  Blink interval in milliseconds [default: 500]
--max-age-days <MAX_AGE_DAYS>      Only check messages newer than N days (0 = all history) [default: 7]
```

### Examples

```bash
# Basic
./target/release/beeper-led-blinker --token YOUR_TOKEN

# Only care about messages from last 3 days
./target/release/beeper-led-blinker --token YOUR_TOKEN --max-age-days 3 --interval 10

# Different LED path (find yours with: ls /sys/class/leds/)
./target/release/beeper-led-blinker --token YOUR_TOKEN --led-path /sys/class/leds/input6::capslock/brightness

# Debug logging
RUST_LOG=debug ./target/release/beeper-led-blinker --token YOUR_TOKEN
```

## Logs

```bash
# Check service status
systemctl --user status beeper-led-blinker.service

# Follow logs
journalctl --user -u beeper-led-blinker.service -f

# Recent logs
journalctl --user -u beeper-led-blinker.service -n 50
```

## Troubleshooting

**LED not blinking?**
```bash
# Test LED manually (the age-old "turn it off and on again" approach)
echo "1" | sudo tee /sys/class/leds/input3::capslock/brightness
echo "0" | sudo tee /sys/class/leds/input3::capslock/brightness

# Check sudoers syntax
sudo visudo -c
```

**Can't connect to API?**
```bash
# Check if Beeper is running
curl "http://localhost:23373/v0/search-chats?limit=1"

# Test with your token
curl -H "Authorization: Bearer YOUR_TOKEN" "http://localhost:23373/v0/search-chats?limit=1"
```

**Not detecting messages?**
- Default only checks last 7 days - use `--max-age-days 30` for older messages
- Archived chats are ignored
- Use `RUST_LOG=debug` to see what's happening

## How it works

1. Polls Beeper's local API (`http://localhost:23373`) every 5 seconds
2. Filters for unread messages within the configured time window
3. Writes `1` or `0` to `/sys/class/leds/input3::capslock/brightness` via sudo
4. Repeats until you kill it

The sudoers config (`/etc/sudoers.d/beeper-blinker`) allows passwordless `tee` to the LED path, and the `!syslog` flag prevents your logs from being spammed with LED state changes.

## Performance

~1-2MB RAM, negligible CPU usage. Binary is ~8MB.

Uses less resources than a Chrome tab (shocking, I know) and definitely less than Electron-based chat apps. Your laptop battery will thank you, unlike when you run Discord.

## Legal Disclaimer

Not responsible for:
- Addiction to blinking LEDs
- Neighbors thinking you're running a rave
- Sudden urge to add LEDs to everything you own
- Existential crisis about why we need our keyboards to tell us things