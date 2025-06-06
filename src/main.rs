mod bot;
mod github;
mod commands;

use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok(); // Load .env file

    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN not set in .env");

    println!("Loaded token: {}", &token[..5]); // Just show first few chars for debug

    // We'll call the bot's startup next
    bot::start(token).await;
}
