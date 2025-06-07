//! Provides system status utilities and slash command handlers for `/status` and `/health`.
//!
//! Includes:
//! - A reusable function to format system metrics (RAM, CPU, disks)
//! - Slash command handlers (`/status`, `/health`)
//! - A background task that posts or edits a single status message on an interval.

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
/// Includes:
/// - RAM usage (percent + MiB)
/// - Average CPU usage and per-core details
/// - Disk usage by mount path (used/total GB and %)
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
/// Replies to the command invoker with the current system resource usage.
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
/// Confirms the bot is responsive and connected.
pub async fn handle_health(ctx: &Context, command: &ApplicationCommandInteraction) {
    let _ = command
        .create_interaction_response(&ctx.http, |res| {
            res.interaction_response_data(|msg| msg.content("✅ Bot is alive."))
        })
        .await;
}

/// Spawns a background task that posts or edits a pinned status message in a Discord channel.
///
/// Behavior:
/// - On first run, posts a new status message and pins it.
/// - On subsequent runs, **edits that message** with fresh data.
///
/// Environment Variables:
/// - `DISCORD_STATUS_CHANNEL_ID`: Channel to post the status
/// - `STATUS_UPDATE_INTERVAL_SECS`: Seconds between updates (default: 600)
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
        let channel = ChannelId(channel_id);
        let http = &ctx.http;
        let mut status_message_id: Option<MessageId> = None;

        loop {
            let content = build_status_message();

            match status_message_id {
                Some(message_id) => {
                    // Try to edit the previously sent message
                    if let Err(e) = channel
                        .edit_message(http, message_id, |m| m.content(content.clone()))
                        .await
                    {
                        eprintln!("Failed to edit status message: {:?}", e);
                        status_message_id = None; // fallback to re-sending
                    }
                }
                None => {
                    // Send a new message and pin it
                    match channel.send_message(http, |m| m.content(content)).await {
                        Ok(msg) => {
                            status_message_id = Some(msg.id);
                            let _ = msg.pin(http).await;
                        }
                        Err(e) => {
                            eprintln!("Failed to send status message: {:?}", e);
                        }
                    }
                }
            }

            sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}
