use anyhow::Result;
use clap::Parser;
use led_controller::LedController;
use regex::Regex;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, info, warn};

#[derive(Parser)]
#[command(name = "dbus-notification-led-blinker")]
#[command(version = "0.1.0")]
#[command(about = "Blinks LED when notifications match text filters")]
struct Args {
    /// Text filter patterns (regex) - LED blinks when any notification matches
    /// Can be specified multiple times: --filter "urgent" --filter "error"
    #[arg(long, required = true)]
    filter: Vec<String>,

    /// LED device path
    #[arg(long, default_value = "/sys/class/leds/input3::capslock/brightness")]
    led_path: String,

    /// Blink interval in milliseconds
    #[arg(long, default_value = "500")]
    blink_interval: u64,

    /// Case insensitive matching
    #[arg(long, default_value = "false")]
    case_insensitive: bool,

    /// Check interval in seconds (for periodic sync)
    #[arg(long, default_value = "3")]
    interval: u64,
}

#[derive(Debug, Clone)]
struct Notification {
    app_name: String,
    summary: String,
    body: String,
}

#[derive(Clone)]
struct NotificationTracker {
    notifications: Arc<RwLock<HashMap<u32, Notification>>>,
    filters: Arc<Vec<Regex>>,
}

impl NotificationTracker {
    fn new(filter_patterns: Vec<String>, case_insensitive: bool) -> Result<Self> {
        let mut filters = Vec::new();
        for pattern in &filter_patterns {
            let regex = if case_insensitive {
                Regex::new(&format!("(?i){}", pattern))?
            } else {
                Regex::new(pattern)?
            };
            filters.push(regex);
            info!("Added filter: {}", pattern);
        }

        Ok(Self {
            notifications: Arc::new(RwLock::new(HashMap::new())),
            filters: Arc::new(filters),
        })
    }

    async fn add_notification(&self, id: u32, app_name: String, summary: String, body: String) -> bool {
        let notification = Notification {
            app_name: app_name.clone(),
            summary: summary.clone(),
            body: body.clone(),
        };

        let matches = self.matches_filter(&notification);

        if matches {
            info!(
                "Notification {} MATCHES filter - app: '{}', summary: '{}', body: '{}'",
                id, app_name, summary, body
            );
        } else {
            debug!(
                "Notification {} added (no match) - app: '{}', summary: '{}'",
                id, app_name, summary
            );
        }

        let mut notifications = self.notifications.write().await;
        notifications.insert(id, notification);

        matches
    }

    async fn remove_notification(&self, id: u32) -> bool {
        let mut notifications = self.notifications.write().await;
        if let Some(notification) = notifications.remove(&id) {
            let was_matching = self.matches_filter(&notification);
            if was_matching {
                info!("Removed matching notification {}", id);
            } else {
                debug!("Removed non-matching notification {}", id);
            }
            return was_matching;
        }
        false
    }

    async fn has_matching_notifications(&self) -> bool {
        let notifications = self.notifications.read().await;
        notifications.values().any(|n| self.matches_filter(n))
    }

    async fn count_matching_notifications(&self) -> usize {
        let notifications = self.notifications.read().await;
        notifications.values().filter(|n| self.matches_filter(n)).count()
    }

    fn matches_filter(&self, notification: &Notification) -> bool {
        for regex in self.filters.iter() {
            if regex.is_match(&notification.app_name)
                || regex.is_match(&notification.summary)
                || regex.is_match(&notification.body)
            {
                return true;
            }
        }
        false
    }
}

async fn is_swaync_running() -> bool {
    let result = Command::new("swaync-client")
        .arg("--count")
        .arg("--skip-wait")
        .output()
        .await;

    result.is_ok() && result.unwrap().status.success()
}

async fn wait_for_swaync() -> Result<()> {
    info!("Waiting for SwayNC to be available...");
    loop {
        if is_swaync_running().await {
            info!("SwayNC is available");
            return Ok(());
        }
        info!("SwayNC not available - retrying in 10 seconds...");
        info!("Make sure swaync is running");
        sleep(StdDuration::from_secs(10)).await;
    }
}

fn parse_dbus_string(line: &str, prefix: &str) -> Option<String> {
    if let Some(idx) = line.find(prefix) {
        let after = &line[idx + prefix.len()..];
        if let Some(start) = after.find('"') {
            if let Some(end) = after[start + 1..].find('"') {
                return Some(after[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

fn parse_dbus_uint32(line: &str, prefix: &str) -> Option<u32> {
    if let Some(idx) = line.find(prefix) {
        let after = &line[idx + prefix.len()..].trim();
        if let Some(space_idx) = after.find(|c: char| c.is_whitespace()) {
            return after[..space_idx].parse().ok();
        } else {
            return after.parse().ok();
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Starting DBus Notification LED blinker");
    info!("LED path: {}", args.led_path);
    info!("Filters: {:?}", args.filter);
    info!("Case insensitive: {}", args.case_insensitive);

    // Initialize LED controller
    let mut led = LedController::new(args.led_path.clone(), args.blink_interval)?;

    // Initialize notification tracker
    let tracker = NotificationTracker::new(args.filter.clone(), args.case_insensitive)?;

    // Wait for SwayNC to be available
    wait_for_swaync().await?;

    info!("Starting dbus-monitor to track notifications...");

    // Start dbus-monitor as a subprocess
    let mut child = Command::new("dbus-monitor")
        .arg("--session")
        .arg("interface='org.freedesktop.Notifications'")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let stdout = child.stdout.take().ok_or_else(|| {
        anyhow::anyhow!("Failed to capture stdout from dbus-monitor")
    })?;

    let mut reader = BufReader::new(stdout).lines();

    info!("Monitoring notifications via dbus-monitor...");
    info!("Waiting for notifications that match filters...");

    let mut currently_blinking = false;
    let mut pending_notification: Option<(Option<String>, Option<String>, Option<String>)> = None;
    let mut in_notify_call = false;
    let mut in_notify_return = false;
    let mut in_close_signal = false;

    // Main loop
    loop {
        tokio::select! {
            // Read lines from dbus-monitor
            line = reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        debug!("DBus: {}", line);

                        // Detect Notify method call
                        if line.contains("method call") && line.contains("member=Notify") {
                            in_notify_call = true;
                            in_notify_return = false;
                            pending_notification = Some((None, None, None));
                            debug!("Started parsing Notify call");
                        }
                        // Detect NotificationClosed signal
                        else if line.contains("signal") && line.contains("member=NotificationClosed") {
                            in_close_signal = true;
                            debug!("Started parsing NotificationClosed signal");
                        }
                        // Parse notification fields from method call
                        else if in_notify_call {
                            if let Some((ref mut app, ref mut summary, ref mut body)) = pending_notification {
                                // Parse app name (first string after member=Notify)
                                if app.is_none() {
                                    if let Some(app_name) = parse_dbus_string(&line, "string ") {
                                        *app = Some(app_name);
                                        debug!("  App: {:?}", app);
                                    }
                                }
                                // Parse summary (should be 4th string)
                                else if summary.is_none() && line.trim().starts_with("string ") {
                                    // Skip icon (3rd string), get summary (4th)
                                    if let Some(s) = parse_dbus_string(&line, "string ") {
                                        if !s.is_empty() {
                                            *summary = Some(s);
                                            debug!("  Summary: {:?}", summary);
                                        }
                                    }
                                }
                                // Parse body (5th string)
                                else if body.is_none() && line.trim().starts_with("string ") {
                                    if let Some(b) = parse_dbus_string(&line, "string ") {
                                        *body = Some(b.clone());
                                        debug!("  Body: {:?}", body);
                                        
                                        // We have all the data, process it
                                        // Use a hash of the content as ID since we can't get the real ID
                                        let app_name = app.clone().unwrap_or_default();
                                        let summary_text = summary.clone().unwrap_or_default();
                                        let body_text = b;
                                        
                                        // Create a simple hash-based ID
                                        use std::collections::hash_map::DefaultHasher;
                                        use std::hash::{Hash, Hasher};
                                        let mut hasher = DefaultHasher::new();
                                        app_name.hash(&mut hasher);
                                        summary_text.hash(&mut hasher);
                                        body_text.hash(&mut hasher);
                                        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros().hash(&mut hasher);
                                        let id = (hasher.finish() as u32) % 1000000;

                                        info!("New notification #{}: app='{}', summary='{}', body='{}'",
                                            id, app_name, summary_text, body_text);

                                        let matched = tracker.add_notification(id, app_name, summary_text, body_text).await;

                                        if matched && !currently_blinking {
                                            info!("Starting LED blink - matching notification detected");
                                            if let Err(e) = led.start_blinking().await {
                                                warn!("Failed to start LED blinking: {}", e);
                                            } else {
                                                currently_blinking = true;
                                            }
                                        }
                                        
                                        // Reset state
                                        in_notify_call = false;
                                        pending_notification = None;
                                    }
                                }
                            }
                        }
                        // Parse NotificationClosed signal
                        else if in_close_signal {
                            if let Some(_id) = parse_dbus_uint32(&line, "uint32 ") {
                                info!("Notification closed - clearing all tracked notifications");
                                // Since we can't reliably track notification IDs, clear everything
                                // The periodic check will re-sync if there are still matching notifications
                                {
                                    let mut notifications = tracker.notifications.write().await;
                                    notifications.clear();
                                }
                                
                                if currently_blinking {
                                    info!("Stopping LED blink - will re-check in periodic sync");
                                    if let Err(e) = led.stop_blinking() {
                                        warn!("Failed to stop LED blinking: {}", e);
                                    } else {
                                        currently_blinking = false;
                                    }
                                }

                                in_close_signal = false;
                            }
                        }
                    }
                    Ok(None) => {
                        warn!("dbus-monitor ended, restarting...");
                        sleep(StdDuration::from_secs(5)).await;

                        child = Command::new("dbus-monitor")
                            .arg("--session")
                            .arg("interface='org.freedesktop.Notifications'")
                            .stdout(Stdio::piped())
                            .stderr(Stdio::null())
                            .spawn()?;

                        let stdout = child.stdout.take().ok_or_else(|| {
                            anyhow::anyhow!("Failed to capture stdout from dbus-monitor")
                        })?;
                        reader = BufReader::new(stdout).lines();
                    }
                    Err(e) => {
                        warn!("Error reading from dbus-monitor: {}", e);
                        sleep(StdDuration::from_secs(5)).await;
                    }
                }
            }

            // Periodic check
            _ = sleep(StdDuration::from_secs(args.interval)) => {
                debug!("Performing periodic check");

                if !is_swaync_running().await {
                    warn!("SwayNC became unavailable");
                    if currently_blinking {
                        info!("Stopping LED due to SwayNC unavailability");
                        if let Err(e) = led.stop_blinking() {
                            warn!("Failed to stop LED: {}", e);
                        } else {
                            currently_blinking = false;
                        }
                    }
                    wait_for_swaync().await?;
                }

                // Check if LED state matches notification state
                let has_matching = tracker.has_matching_notifications().await;
                let count = tracker.count_matching_notifications().await;

                if has_matching && !currently_blinking {
                    info!("Sync: Starting LED blink ({} matching notifications)", count);
                    if let Err(e) = led.start_blinking().await {
                        warn!("Failed to start LED: {}", e);
                    } else {
                        currently_blinking = true;
                    }
                } else if !has_matching && currently_blinking {
                    info!("Sync: Stopping LED blink (no matching notifications)");
                    if let Err(e) = led.stop_blinking() {
                        warn!("Failed to stop LED: {}", e);
                    } else {
                        currently_blinking = false;
                    }
                } else if has_matching {
                    debug!("Sync: {} matching notifications, LED blinking", count);
                }
            }
        }
    }
}
