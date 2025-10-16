# LED Controller Library

A Rust library for controlling LEDs on Linux systems via sysfs.

## Overview

This library provides a simple interface to control LEDs through the Linux sysfs interface. It supports turning LEDs on/off and blinking them at configurable intervals.

## Features

- Turn LEDs on and off
- Asynchronous LED blinking with configurable intervals
- Graceful start/stop of blinking
- Uses `sudo tee` for privileged access to sysfs

## Requirements

- Linux system with sysfs LED interface
- `sudo` access for writing to LED brightness files
- Tokio async runtime

## Usage

### Adding to Your Project

Add this to your `Cargo.toml`:

```toml
[dependencies]
led-controller = { path = "../led-controller" }
tokio = { version = "1", features = ["full"] }
anyhow = "1"
tracing = "0.1"
```

### Basic Example

```rust
use led_controller::LedController;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    // Create a controller for your LED device
    let led_path = "/sys/class/leds/input3::capslock/brightness";
    let mut controller = LedController::new(led_path.to_string(), 500)?;

    // Turn LED on
    controller.set_led_state(true)?;

    // Turn LED off
    controller.set_led_state(false)?;

    Ok(())
}
```

### Blinking Example

```rust
use led_controller::LedController;
use anyhow::Result;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let led_path = "/sys/class/leds/input3::capslock/brightness";
    let mut controller = LedController::new(led_path.to_string(), 500)?;

    // Start blinking (500ms interval)
    controller.start_blinking().await?;

    // Let it blink for 5 seconds
    sleep(Duration::from_secs(5)).await;

    // Stop blinking
    controller.stop_blinking()?;

    Ok(())
}
```

### Changing Blink Interval

```rust
use led_controller::LedController;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let led_path = "/sys/class/leds/input3::capslock/brightness";
    let mut controller = LedController::new(led_path.to_string(), 500)?;

    // Start with 500ms interval
    controller.start_blinking().await?;

    // Change interval (requires restart to take effect)
    controller.stop_blinking()?;
    controller.set_blink_interval(200); // 200ms interval
    controller.start_blinking().await?;

    Ok(())
}
```

## API Reference

### `LedController::new(led_path: String, blink_interval: u64) -> Result<Self>`

Creates a new LED controller.

- `led_path`: Path to the LED brightness file (e.g., `/sys/class/leds/input3::capslock/brightness`)
- `blink_interval`: Default blink interval in milliseconds

### `set_led_state(&self, on: bool) -> Result<()>`

Sets the LED to on or off.

- `on`: `true` to turn LED on, `false` to turn it off

### `start_blinking(&mut self) -> Result<()>`

Starts blinking the LED using the configured blink interval. Spawns a background task that continues until `stop_blinking()` is called.

### `stop_blinking(&mut self) -> Result<()>`

Stops the LED from blinking and turns it off.

### `is_blinking(&self) -> bool`

Returns whether the LED is currently blinking.

### `blink_interval(&self) -> u64`

Gets the current blink interval in milliseconds.

### `set_blink_interval(&mut self, interval: u64)`

Sets a new blink interval in milliseconds. Note: If the LED is currently blinking, you need to stop and restart it for the new interval to take effect.

## Finding Your LED Path

To find available LEDs on your system:

```bash
ls /sys/class/leds/
```

Common LED paths:
- Keyboard LEDs: `/sys/class/leds/input*::capslock/brightness`
- Laptop indicator LEDs: `/sys/class/leds/platform::*/brightness`
- Custom hardware LEDs: `/sys/class/leds/<device-name>/brightness`

## Permissions

The library uses `sudo tee` to write to the LED brightness files, which typically require root permissions. Ensure your user has appropriate sudo privileges.

For passwordless operation, you can add a sudoers entry:
```
username ALL=(ALL) NOPASSWD: /usr/bin/tee /sys/class/leds/*/brightness
```

## Error Handling

All public methods return `Result<T>` from the `anyhow` crate. Handle errors appropriately:

```rust
match controller.set_led_state(true) {
    Ok(_) => println!("LED turned on"),
    Err(e) => eprintln!("Failed to turn on LED: {}", e),
}
```

## License

See the parent project for license information.
