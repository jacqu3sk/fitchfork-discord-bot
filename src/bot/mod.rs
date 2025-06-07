//! Main bot module.
//!
//! This module defines the Discord bot's startup routine, event handlers, and
//! command registration logic. It initializes the bot, listens for interactions,
//! manages shared state, and spawns background workers like the system status loop.

use serenity::{
    async_trait,
    model::prelude::*,
    model::application::interaction::{Interaction},
    model::application::command::Command,
    prelude::*,
    Client,
};

use crate::AppState;
use crate::commands::{
    clean, fresh, migrate, reboot,
    restart_api, restart_service,
    start_api, stop_api,
    tail_logs, uptime,
};

mod status;
use status::{handle_health, handle_status, start_status_loop};

/// Starts the Discord bot client.
///
/// This function initializes the bot with the given token and app state,
/// sets up the event handler, and connects to the Discord gateway.
/// Any runtime errors will be logged to stderr.
///
/// # Arguments
/// - `token`: Discord bot token.
/// - `state`: Shared application state used across modules.
pub async fn start(token: String, state: AppState) {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let handler = Handler {
        shared_state: state.clone(),
    };

    let mut client = Client::builder(&token, intents)
        .event_handler(handler)
        .await
        .expect("Error creating Discord client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}

/// Serenity event handler for managing Discord gateway events.
///
/// This handler processes slash command interactions and takes action when the bot becomes ready.
struct Handler {
    shared_state: AppState,
}

#[async_trait]
impl EventHandler for Handler {
    /// Handles all incoming application command interactions (slash commands).
    ///
    /// Routes commands to the appropriate async handler function.
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "status" => handle_status(&ctx, &command).await,
                "health" => handle_health(&ctx, &command).await,
                "uptime" => uptime(&ctx, &command).await,
                "restart" => restart_service(&ctx, &command).await,
                "clean" => clean(&ctx, &command).await,
                "fresh" => fresh(&ctx, &command).await,
                "migrate" => migrate(&ctx, &command).await,
                "restart_api" => restart_api(&ctx, &command).await,
                "start_api" => start_api(&ctx, &command).await,
                "stop_api" => stop_api(&ctx, &command).await,
                "tail_logs" => tail_logs(&ctx, &command).await,
                "reboot" => reboot(&ctx, &command).await,
                _ => {}
            }
        }
    }

    /// Called when the bot is fully connected and ready.
    ///
    /// - Stores the Discord context globally so other modules (like system commands) can access it.
    /// - Launches a background status update loop that periodically posts system metrics.
    /// - Registers all slash commands globally with Discord.
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        // Store context for later use in background tasks or manual command sending.
        {
            let mut lock = self.shared_state.discord_ctx.lock().unwrap();
            *lock = Some(ctx.clone());
        }

        // Start the repeating system status updater task in a separate async thread.
        start_status_loop(ctx.clone()).await;

        // Register slash commands available to users
        register_command(&ctx, "status", "Show system status (CPU, RAM, Disk)").await;
        register_command(&ctx, "health", "Simple health check to see if the bot is responsive").await;
        register_command(&ctx, "uptime", "Show system uptime").await;

        register_command_with_option(
            &ctx,
            "restart",
            "Restart a systemd service",
            "service",
            "The name of the systemd service to restart"
        ).await;

        // Register additional predefined bot actions
        for (name, description) in &[
            ("clean", "Run cargo make clean"),
            ("fresh", "Run cargo make fresh"),
            ("migrate", "Run cargo make migrate"),
            ("restart_api", "Restart the FitchFork API"),
            ("start_api", "Start the FitchFork API"),
            ("stop_api", "Stop the FitchFork API"),
            ("tail_logs", "Tail the FitchFork log file"),
            ("reboot", "Reboot the server"),
        ] {
            register_command(&ctx, name, description).await;
        }
    }
}

/// Registers a simple slash command with no parameters.
///
/// # Arguments
/// - `ctx`: Discord context to register the command against.
/// - `name`: Name of the command (e.g., "health").
/// - `description`: Description shown in the Discord UI.
async fn register_command(ctx: &Context, name: &str, description: &str) {
    let _ = Command::create_global_application_command(&ctx.http, |cmd| {
        cmd.name(name).description(description)
    })
    .await;
}

/// Registers a slash command that requires a string parameter.
///
/// Useful for commands like `/restart` that accept a service name.
///
/// # Arguments
/// - `ctx`: Discord context.
/// - `name`: Name of the command.
/// - `description`: Overall command description.
/// - `option`: Name of the parameter (e.g., "service").
/// - `option_desc`: Description of the parameter shown to the user.
async fn register_command_with_option(
    ctx: &Context,
    name: &str,
    description: &str,
    option: &str,
    option_desc: &str,
) {
    let _ = Command::create_global_application_command(&ctx.http, |cmd| {
        cmd.name(name)
            .description(description)
            .create_option(|opt| {
                opt.name(option)
                    .description(option_desc)
                    .kind(serenity::model::application::command::CommandOptionType::String)
                    .required(true)
            })
    })
    .await;
}
