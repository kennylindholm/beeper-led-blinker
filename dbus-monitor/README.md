# DBus Notification LED Blinker

A Linux notification monitor that blinks an LED when notifications match specified text filters. This application monitors all DBus notifications (works with DBus Notifications, Dunst, Mako, or any notification daemon) and triggers LED blinking based on regex pattern matching.

## Features

- Monitor DBus Notifications notifications in real-time
- Multiple regex pattern filters (case-sensitive or insensitive)
- LED blink on notification match
- Automatic LED control via sudo (passwordless)
- Systemd service integration
- Similar architecture to the Beeper LED blinker

## Prerequisites

- Linux system with DBus Notifications installed
- Rust toolchain (for building)
- LED device accessible via sysfs (e.g., `/sys/class/leds/*/brightness`)
- sudo access for LED control setup
- `dbus-monitor` tool (usually pre-installed with dbus package)

## Installation

### Quick Install

Run the build and install script:

```bash
cd dbus-monitor
./buildandinstall.sh
```

The script will:
1. Build the release binary
2. Set up passwordless sudo for LED control
3. Install the binary to `~/.local/bin/`
4. Create and enable a systemd user service
5. Prompt for filter patterns

### Manual Build

```bash
cargo build --release --package dbus-notification-led-blinker
```

The binary will be at `target/release/dbus-notification-led-blinker`.

## Usage

### Command Line Options

```bash
dbus-notification-led-blinker \
  --filter "pattern1" \
  --filter "pattern2" \
  --led-path /sys/class/leds/input3::capslock/brightness \
  --blink-interval 500 \
  --case-insensitive
```

**Options:**

- `--filter <PATTERN>` (required, can be specified multiple times)  
  Regex pattern to match against notification text (app name, summary, body)
  
- `--led-path <PATH>` (default: `/sys/class/leds/input3::capslock/brightness`)  
  Path to LED brightness control file
  
- `--blink-interval <MS>` (default: `500`)  
  LED blink interval in milliseconds
  
- `--case-insensitive` (default: `false`)  
  Enable case-insensitive pattern matching
  
- `--interval <SECS>` (default: `30`)  
  Fallback polling interval for health checks

### Filter Pattern Examples

Match notifications containing "urgent":
```bash
--filter "urgent"
```

Match multiple words (OR):
```bash
--filter "error|critical|urgent"
```

Match notifications from specific app:
```bash
--filter "Signal"
```

Match multiple patterns:
```bash
--filter "urgent" --filter "Signal" --filter "alarm"
```

Case-insensitive matching:
```bash
--filter "ERROR" --case-insensitive
```

## Systemd Service

### Service Management

```bash
# Check status
systemctl --user status dbus-notification-led-blinker.service

# View logs
journalctl --user -u dbus-notification-led-blinker.service -f

# Restart service
systemctl --user restart dbus-notification-led-blinker.service

# Stop service
systemctl --user stop dbus-notification-led-blinker.service

# Disable service
systemctl --user disable dbus-notification-led-blinker.service
```

### Editing the Service

Edit the service file to change filters or options:

```bash
nano ~/.config/systemd/user/dbus-notification-led-blinker.service
```

After editing, reload and restart:

```bash
systemctl --user daemon-reload
systemctl --user restart dbus-notification-led-blinker.service
```

## How It Works

1. **Monitors Notifications**: Uses `dbus-monitor` to intercept all notification events on the session bus
2. **Parses DBus Messages**: Extracts app_name, summary, and body from Notify method calls
3. **Tracks Active Notifications**: Maintains a list of active notifications in memory
4. **Pattern Matching**: Checks each notification text (app name, summary, body) against your regex filters
5. **LED Control**: 
   - Starts blinking when a notification matches any filter
   - Continues blinking while matching notifications exist
   - Stops blinking when all matching notifications are cleared (via NotificationClosed signal)
6. **Auto-Recovery**: Restarts dbus-monitor if it crashes and monitors DBus Notifications availability

## Architecture

Similar to the Beeper LED blinker, this application uses:

- **Tokio**: Async runtime for event handling
- **LED Controller**: Shared library for LED control (via sudo tee)
- **Regex**: Pattern matching engine
- **dbus-monitor**: System tool for monitoring DBus messages
- **Notification Tracker**: In-memory HashMap tracking active notifications

## Troubleshooting

### LED not blinking

Check if sudo is configured correctly:
```bash
cat /etc/sudoers.d/dbus-notification-led-blinker
```

Should contain:
```
<your-username> ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/class/leds/*/brightness
```

Test LED control manually:
```bash
echo "1" | sudo tee /sys/class/leds/input3::capslock/brightness
echo "0" | sudo tee /sys/class/leds/input3::capslock/brightness
```

### Service not starting

Check logs:
```bash
journalctl --user -u dbus-notification-led-blinker.service -n 50
```

Verify DBus Notifications is running:
```bash
swaync-client --count
```

### Filters not matching

Enable debug logging:
```bash
# Edit service file
nano ~/.config/systemd/user/dbus-notification-led-blinker.service

# Change Environment line to:
Environment=RUST_LOG=debug

# Reload and restart
systemctl --user daemon-reload
systemctl --user restart dbus-notification-led-blinker.service

# Watch logs
journalctl --user -u dbus-notification-led-blinker.service -f
```

## Comparison with Beeper LED Blinker

| Feature | Beeper LED Blinker | DBus Notifications LED Blinker |
|---------|-------------------|-------------------|
| **Data Source** | Beeper Desktop API | DBus Notifications notifications |
| **Trigger** | Unread message count | Text pattern match |
| **Filtering** | Message age | Regex patterns |
| **Authentication** | API token | None (local) |
| **Architecture** | API polling | Event subscription |

## Contributing

This application follows the same structure as the Beeper LED blinker. Both share the `led-controller` library.

## License

Same as parent project.
