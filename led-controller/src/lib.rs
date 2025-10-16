use anyhow::Result;
use std::process::Command;
use std::time::Duration as StdDuration;
use tokio::sync::watch;
use tokio::time::sleep;
use tracing::{debug, error, info};

/// Controls an LED by writing to a sysfs brightness file
pub struct LedController {
    led_path: String,
    blink_interval: u64,
    is_blinking: bool,
    stop_tx: Option<watch::Sender<bool>>,
}

impl LedController {
    /// Creates a new LED controller for the given LED device path
    ///
    /// # Arguments
    /// * `led_path` - Path to the LED brightness file (e.g., "/sys/class/leds/input3::capslock/brightness")
    /// * `blink_interval` - Default blink interval in milliseconds (default: 500ms)
    ///
    /// # Returns
    /// * `Result<Self>` - A new LedController instance if LED access is successful
    ///
    /// # Errors
    /// Returns an error if the LED device cannot be accessed or written to
    pub fn new(led_path: String, blink_interval: u64) -> Result<Self> {
        let controller = Self {
            led_path,
            blink_interval,
            is_blinking: false,
            stop_tx: None,
        };

        // Test LED access
        controller.set_led_state(false)?;
        info!("LED control permissions verified");

        Ok(controller)
    }

    /// Sets the LED to on or off
    ///
    /// # Arguments
    /// * `on` - true to turn LED on, false to turn it off
    ///
    /// # Errors
    /// Returns an error if the LED state cannot be set
    pub fn set_led_state(&self, on: bool) -> Result<()> {
        Self::set_led_state_static(&self.led_path, on)
    }

    /// Starts blinking the LED using the configured blink interval
    ///
    /// # Returns
    /// * `Result<()>` - Ok if blinking started successfully
    ///
    /// # Notes
    /// - If already blinking, this is a no-op
    /// - Spawns a background task that continues until `stop_blinking()` is called
    /// - The LED will be turned off when blinking stops
    /// - Uses the blink interval set in the constructor
    pub async fn start_blinking(&mut self) -> Result<()> {
        if self.is_blinking {
            return Ok(());
        }

        self.is_blinking = true;
        info!("Starting LED blinking");

        let led_path = self.led_path.clone();
        let interval = StdDuration::from_millis(self.blink_interval);

        // Create a channel to signal the task to stop
        let (stop_tx, mut stop_rx) = watch::channel(false);
        self.stop_tx = Some(stop_tx);

        tokio::spawn(async move {
            let mut state = true;
            loop {
                // Check if we should stop
                if *stop_rx.borrow() {
                    debug!("Blink task received stop signal");
                    break;
                }

                if let Err(e) = Self::set_led_state_static(&led_path, state) {
                    error!("Failed to set LED state: {}", e);
                }
                state = !state;

                // Use tokio::select to wait for either the interval or stop signal
                tokio::select! {
                    _ = sleep(interval) => {},
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            debug!("Blink task stopping");
                            break;
                        }
                    }
                }
            }

            // Turn off LED when stopping
            let _ = Self::set_led_state_static(&led_path, false);
        });

        Ok(())
    }

    /// Stops the LED from blinking and turns it off
    ///
    /// # Returns
    /// * `Result<()>` - Ok if LED stopped successfully
    ///
    /// # Notes
    /// - If not currently blinking, this is a no-op
    /// - Signals the background blinking task to stop
    /// - Ensures LED is turned off
    pub fn stop_blinking(&mut self) -> Result<()> {
        if !self.is_blinking {
            return Ok(());
        }

        self.is_blinking = false;
        info!("Stopping LED blinking");

        // Send stop signal to the blinking task
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(true);
        }

        self.set_led_state(false)
    }

    /// Returns whether the LED is currently blinking
    pub fn is_blinking(&self) -> bool {
        self.is_blinking
    }

    /// Gets the current blink interval in milliseconds
    pub fn blink_interval(&self) -> u64 {
        self.blink_interval
    }

    /// Sets a new blink interval in milliseconds
    ///
    /// # Arguments
    /// * `interval` - New blink interval in milliseconds
    ///
    /// # Notes
    /// - If the LED is currently blinking, you need to stop and restart it for the new interval to take effect
    pub fn set_blink_interval(&mut self, interval: u64) {
        self.blink_interval = interval;
    }

    /// Internal helper to set LED state using sudo tee
    fn set_led_state_static(led_path: &str, on: bool) -> Result<()> {
        let state = if on { "1" } else { "0" };

        let mut child = Command::new("sudo")
            .arg("tee")
            .arg(led_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        if let Some(stdin) = child.stdin.take() {
            use std::io::Write;
            let mut stdin = stdin;
            stdin.write_all(state.as_bytes())?;
        }

        child.wait()?;
        Ok(())
    }
}
