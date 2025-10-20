use led_controller::LedController;
use clap::Parser;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration as StdDuration;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

#[derive(Parser)]
#[command(name = "beeper-led-blinker")]
#[command(version = "0.1.0")]
#[command(about = "Blinks LED when you have unread Beeper messages")]
struct Args {
    /// Beeper API token
    #[arg(long, env)]
    token: String,

    /// LED device path
    #[arg(long, default_value = "/sys/class/leds/input3::capslock/brightness")]
    led_path: String,

    /// API base URL
    #[arg(long, default_value = "http://localhost:23373")]
    api_url: String,

    /// Check interval in seconds
    #[arg(long, default_value = "5")]
    interval: u64,

    /// Blink interval in milliseconds
    #[arg(long, default_value = "500")]
    blink_interval: u64,

    /// Only check messages newer than this many days (0 = all history)
    #[arg(long, default_value = "7")]
    max_age_days: i64,

    /// Filter out messages from archived chats
    #[arg(long, default_value = "true")]
    exclude_archived: bool,

    /// Filter out messages from muted chats
    #[arg(long, default_value = "true")]
    exclude_muted: bool,
}

#[derive(Debug, Deserialize)]
struct SearchMessagesResponse {
    items: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    #[allow(dead_code)]
    id: String,
    #[serde(rename = "chatID")]
    #[allow(dead_code)]
    chat_id: String,
    #[allow(dead_code)]
    timestamp: DateTime<Utc>,
    #[serde(rename = "isUnread", default)]
    is_unread: bool,
}

struct BeeperClient {
    client: Client,
    api_url: String,
    token: String,
}

impl BeeperClient {
    fn new(api_url: String, token: String) -> Self {
        let client = Client::new();
        Self {
            client,
            api_url,
            token,
        }
    }

    async fn is_api_available(&self) -> bool {
        let url = format!("{}/v0/search-messages", self.api_url);

        match self.client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("limit", "1")])
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    async fn get_recent_unread_count(&self, max_age_days: i64, exclude_archived: bool, exclude_muted: bool) -> Result<u32> {
        let url = format!("{}/v0/search-messages", self.api_url);

        let mut query_params = vec![
            ("unreadOnly", "true".to_string()),
            ("limit", "500".to_string()),
        ];

        // Only add date filter if max_age_days > 0
        if max_age_days > 0 {
            let cutoff_date = Utc::now() - Duration::days(max_age_days);
            let cutoff_date_str = cutoff_date.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            debug!("Checking for unread messages newer than {}", cutoff_date.format("%Y-%m-%d %H:%M:%S"));
            query_params.push(("after", cutoff_date_str));
        } else {
            debug!("Checking for unread messages (all history)");
        }

        // Add archive filter
        if exclude_archived {
            query_params.push(("excludeArchived", "true".to_string()));
        }

        // Add muted filter
        if exclude_muted {
            query_params.push(("excludeMuted", "true".to_string()));
        }

        let response = self.client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&query_params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("API request failed: {}", response.status()));
        }

        let messages: SearchMessagesResponse = response.json().await?;

        let total_unread = messages.items
            .iter()
            .filter(|msg| msg.is_unread)
            .count() as u32;

        if total_unread > 0 {
            debug!("Found {} unread messages", total_unread);

            // Group by chat for better logging
            use std::collections::HashMap;
            let mut chat_counts: HashMap<&str, u32> = HashMap::new();
            for msg in &messages.items {
                if msg.is_unread {
                    *chat_counts.entry(&msg.chat_id).or_insert(0) += 1;
                }
            }

            debug!("  Unread messages across {} chats", chat_counts.len());
        }

        Ok(total_unread)
    }
}

async fn wait_for_api(client: &BeeperClient) -> Result<()> {
    info!("Waiting for Beeper Desktop API to be available...");

    loop {
        if client.is_api_available().await {
            info!("Beeper Desktop API is available at {}", client.api_url);
            return Ok(());
        }

        info!("Beeper Desktop API not available - retrying in 10 seconds...");
        info!("Make sure Beeper Desktop is running and API is enabled in Settings > Developers");
        sleep(StdDuration::from_secs(10)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Starting Beeper LED blinker");
    info!("LED path: {}", args.led_path);
    info!("API URL: {}", args.api_url);
    info!("Check interval: {}s", args.interval);
    if args.max_age_days > 0 {
        info!("Max message age: {} days", args.max_age_days);
    } else {
        info!("Max message age: all history");
    }
    info!("Exclude archived chats: {}", args.exclude_archived);
    info!("Exclude muted chats: {}", args.exclude_muted);

    // Initialize LED controller
    let mut led = LedController::new(args.led_path, args.blink_interval)?;

    // Initialize Beeper client
    let beeper = BeeperClient::new(args.api_url, args.token);

    // Wait for API to be available
    wait_for_api(&beeper).await?;

    // Check initial state
    let initial_unread = beeper.get_recent_unread_count(args.max_age_days, args.exclude_archived, args.exclude_muted).await?;
    let mut currently_blinking = false;

    if initial_unread > 0 {
        info!("Starting with {} unread messages - enabling LED", initial_unread);
        led.start_blinking().await?;
        currently_blinking = true;
    } else {
        info!("No recent unread messages - LED off");
        led.set_led_state(false)?;
    }

    info!("Monitoring Beeper Desktop API for unread messages...");

    // Main monitoring loop
    loop {
        sleep(StdDuration::from_secs(args.interval)).await;

        // Check if API is still available
        if !beeper.is_api_available().await {
            warn!("Beeper Desktop API became unavailable");
            if currently_blinking {
                info!("Stopping LED blink due to API unavailability");
                led.stop_blinking()?;
                currently_blinking = false;
            }

            warn!("Waiting for API to reconnect...");
            wait_for_api(&beeper).await?;
            continue;
        }

        // Get current unread count
        match beeper.get_recent_unread_count(args.max_age_days, args.exclude_archived, args.exclude_muted).await {
            Ok(unread_count) => {
                if unread_count > 0 && !currently_blinking {
                    info!("Found {} unread messages - starting LED blink", unread_count);
                    led.start_blinking().await?;
                    currently_blinking = true;
                } else if unread_count == 0 && currently_blinking {
                    info!("No unread messages - stopping LED blink");
                    led.stop_blinking()?;
                    currently_blinking = false;
                } else if unread_count > 0 {
                    debug!("Still have {} unread messages - LED continues blinking", unread_count);
                }
            }
            Err(e) => {
                error!("Failed to get unread count: {}", e);
                // Don't change LED state on API errors
            }
        }
    }
}
