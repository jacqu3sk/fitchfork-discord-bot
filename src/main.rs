mod bot;
mod github;
mod commands;

use std::{env, net::SocketAddr, sync::{Arc, Mutex}};
use axum::{Router};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use tower_http::cors::CorsLayer;
use dotenvy::dotenv;

#[derive(Clone)]
pub struct AppState {
    pub discord_ctx: Arc<Mutex<Option<serenity::prelude::Context>>>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // Load env vars
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse::<u16>()
        .expect("PORT must be a valid number");
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set");

    // Shared bot/app state
    let shared_state = AppState {
        discord_ctx: Arc::new(Mutex::new(None)),
    };

    // Start Discord bot in background
    let bot_state = shared_state.clone();
    tokio::spawn(async move {
        bot::start(token, bot_state).await;
    });

    // Build Axum app
    let cors = CorsLayer::very_permissive()
        .expose_headers([CONTENT_DISPOSITION, CONTENT_TYPE]);

    let app = Router::new()
        .nest("/webhook", github::routes(shared_state.clone()))
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid address");
    println!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .expect("Server crashed");
}
