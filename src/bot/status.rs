//! Provides system status utilities and slash command handlers for `/status` and `/health`.
//!
//! Includes a background task that posts live system metrics (CPU, RAM, disk usage)
//! to a dedicated Discord channel on a regular interval.

use serenity::{
    model::application::interaction::application_command::ApplicationCommandInteraction,
    model::prelude::*,
    prelude::*,
};
use std::{env, time::Duration};
use tokio::time::sleep;
use sysinfo::{CpuExt, DiskExt, System, SystemExt};

/// Constructs a human-readable system status message.
///
/// This includes:
/// - RAM usage (percentage and MiB)
/// - Average CPU usage and per-core breakdown
/// - Disk usage for each mounted disk (used/total GB and percentage)
///
/// This function is used both for the `/status` command and for
/// the automated background status updates.
pub fn build_status_message() -> String {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_count = sys.cpus().len();
    let avg_cpu = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_count as f32;

    let cpu_details = sys
        .cpus()
        .iter()
        .enumerate()
        .map(|(i, c)| format!("Core {}: {:.1}%", i, c.cpu_usage()))
        .collect::<Vec<_>>()
        .join("\n");

    let ram_used = sys.used_memory() / 1024;
    let ram_total = sys.total_memory() / 1024;
    let ram_percent = (ram_used as f32 / ram_total as f32) * 100.0;

    let disk_info = sys
        .disks()
        .iter()
        .map(|d| {
            let name = d.name().to_string_lossy();
            let mount = d.mount_point().display();
            let used = d.total_space() - d.available_space();
            let used_gb = used as f64 / 1e9;
            let total_gb = d.total_space() as f64 / 1e9;
            let percent = (used as f64 / d.total_space() as f64) * 100.0;
            format!(
                "**{}** (`{}`): `{:.1} GB / {:.1} GB` ({:.1}%)",
                name, mount, used_gb, total_gb, percent
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "**System Status**\n\
         **RAM Usage:** `{:.1}%` (`{} MiB / {} MiB`)\n\
         **CPU Usage:** `{:.1}% average` over {} cores\n\
         ```\n{}\n```\n\
         **Disks:**\n{}",
        ram_percent, ram_used, ram_total, avg_cpu, cpu_count, cpu_details, disk_info
    )
}

/// Slash command handler for `/status`.
///
/// This command fetches live system metrics and sends them as a one-time response
/// to the user who invoked the command.
pub async fn handle_status(ctx: &Context, command: &ApplicationCommandInteraction) {
    let content = build_status_message();

    let _ = command
        .create_interaction_response(&ctx.http, |res| {
            res.interaction_response_data(|msg| msg.content(content))
        })
        .await;
}

/// Slash command handler for `/health`.
///
/// Simply confirms that the bot is responsive and connected to Discord.
pub async fn handle_health(ctx: &Context, command: &ApplicationCommandInteraction) {
    let _ = command
        .create_interaction_response(&ctx.http, |res| {
            res.interaction_response_data(|msg| msg.content("✅ Bot is alive."))
        })
        .await;
}

/// Spawns a background task that posts a fresh system status message at regular intervals.
///
/// The task performs the following loop:
/// 1. Deletes the last 100 messages from the configured status channel
/// 2. Posts a new status message from `build_status_message()`
/// 3. Waits for the configured duration before repeating
///
/// Environment Variables:
/// - `DISCORD_STATUS_CHANNEL_ID`: ID of the target channel
/// - `STATUS_UPDATE_INTERVAL_SECS`: Interval between updates in seconds (default: 600)
///
/// This ensures the channel always shows the most recent status clearly, with no clutter.
pub async fn start_status_loop(ctx: Context) {
    let channel_id: u64 = env::var("DISCORD_STATUS_CHANNEL_ID")
        .expect("DISCORD_STATUS_CHANNEL_ID must be set")
        .parse()
        .expect("Invalid DISCORD_STATUS_CHANNEL_ID");

    let interval_secs: u64 = env::var("STATUS_UPDATE_INTERVAL_SECS")
        .unwrap_or_else(|_| "600".to_string())
        .parse()
        .unwrap_or(600);

    tokio::spawn(async move {
        loop {
            let channel = ChannelId(channel_id);
            let http = &ctx.http;

            // Delete previous messages in the status channel
            match channel.messages(http, |m| m.limit(100)).await {
                Ok(messages) => {
                    for msg in messages {
                        let _ = channel.delete_message(http, msg.id).await;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to delete status messages: {:?}", e);
                }
            }

            // Send a fresh status update
            let message = build_status_message();
            let _ = channel.send_message(http, |m| m.content(message)).await;

            // Wait for the configured interval before repeating
            sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}
