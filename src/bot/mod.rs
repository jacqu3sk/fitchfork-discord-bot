use serenity::{
    async_trait,
    model::prelude::*,
    prelude::*,
    Client,
};
use serenity::model::application::interaction::{Interaction, application_command::ApplicationCommandInteraction};
use serenity::model::application::command::Command;

use crate::AppState;
use crate::commands::{
    uptime, restart_service,
    ff_clean, ff_fresh, ff_migrate,
    ff_restart_api, ff_start_api, ff_stop_api,
    ff_tail_logs, ff_reboot,
};
use sysinfo::{System, SystemExt, CpuExt, DiskExt};

pub async fn start(token: String, state: AppState) {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let handler = Handler {
        shared_state: state.clone(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}

struct Handler {
    shared_state: AppState,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "status" => handle_status(&ctx, &command).await,
                "uptime" => uptime(&ctx, &command).await,
                "restart" => restart_service(&ctx, &command).await,
                "ff_clean" => ff_clean(&ctx, &command).await,
                "ff_fresh" => ff_fresh(&ctx, &command).await,
                "ff_migrate" => ff_migrate(&ctx, &command).await,
                "ff_restart_api" => ff_restart_api(&ctx, &command).await,
                "ff_start_api" => ff_start_api(&ctx, &command).await,
                "ff_stop_api" => ff_stop_api(&ctx, &command).await,
                "ff_tail_logs" => ff_tail_logs(&ctx, &command).await,
                "ff_reboot" => ff_reboot(&ctx, &command).await,
                _ => {}
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        // clone before lock+set to avoid async Send issues
        let ctx_clone = ctx.clone();
        {
            let mut lock = self.shared_state.discord_ctx.lock().unwrap();
            *lock = Some(ctx_clone);
        }

        let _ = Command::create_global_application_command(&ctx.http, |cmd| {
            cmd.name("status").description("Show system status (CPU, RAM, Disk)")
        }).await;

        let _ = Command::create_global_application_command(&ctx.http, |cmd| {
            cmd.name("uptime").description("Show system uptime")
        }).await;

        let _ = Command::create_global_application_command(&ctx.http, |cmd| {
            cmd.name("restart")
                .description("Restart a systemd service")
                .create_option(|opt| {
                    opt.name("service")
                        .description("The name of the systemd service to restart")
                        .kind(serenity::model::application::command::CommandOptionType::String)
                        .required(true)
                })
        }).await;

        for (name, description) in &[
            ("ff_clean", "Run cargo make clean"),
            ("ff_fresh", "Run cargo make fresh"),
            ("ff_migrate", "Run cargo make migrate"),
            ("ff_restart_api", "Restart the FitchFork API"),
            ("ff_start_api", "Start the FitchFork API"),
            ("ff_stop_api", "Stop the FitchFork API"),
            ("ff_tail_logs", "Tail the FitchFork log file"),
            ("ff_reboot", "Reboot the server"),
        ] {
            let _ = Command::create_global_application_command(&ctx.http, |cmd| {
                cmd.name(name).description(description)
            }).await;
        }
    }
}

async fn handle_status(ctx: &Context, command: &ApplicationCommandInteraction) {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu_count = sys.cpus().len();
    let avg_cpu = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_count as f32;
    let cpu_details = sys.cpus().iter().enumerate().map(|(i, c)| {
        format!("Core {}: {:.1}%", i, c.cpu_usage())
    }).collect::<Vec<_>>().join("\n");
    let ram_used = sys.used_memory() / 1024;
    let ram_total = sys.total_memory() / 1024;
    let ram_percent = (ram_used as f32 / ram_total as f32) * 100.0;
    let disk_info = sys.disks().iter().map(|d| {
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
    }).collect::<Vec<_>>().join("\n");
    let content = format!(
        "**System Status**\n\
        **RAM Usage:** `{:.1}%` (`{} MiB / {} MiB`)\n\
        **CPU Usage:** `{:.1}% average` over {} cores\n\
        ```\n{}\n```\n\
        **Disks:**\n{}",
        ram_percent, ram_used, ram_total,
        avg_cpu, cpu_count, cpu_details,
        disk_info
    );
    let _ = command
        .create_interaction_response(&ctx.http, |res| {
            res.interaction_response_data(|msg| msg.content(content))
        })
        .await;
}
