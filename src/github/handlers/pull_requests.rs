use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serenity::model::id::ChannelId;
use std::env;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub pull_request: PullRequest,
    pub repository: Repository,
    pub sender: Sender,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub html_url: String,
    pub title: String,
    pub head: BranchRef, // source branch
    pub base: BranchRef, // target branch
}

#[derive(Debug, Deserialize)]
pub struct BranchRef {
    #[serde(rename = "ref")]
    pub r#ref: String,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Debug, Deserialize)]
pub struct Sender {
    pub login: String,
}

pub async fn handle_pull_request_event(
    State(state): State<AppState>,
    Json(payload): Json<PullRequestEvent>,
) -> Response {
    if payload.action != "opened" {
        return StatusCode::OK.into_response();
    }

    let ctx = {
        let guard = state.discord_ctx.lock().unwrap();
        match &*guard {
            Some(ctx) => ctx.clone(),
            None => {
                eprintln!("Discord context not initialized yet.");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    };

    let channel_id: u64 = env::var("DISCORD_PR_CHANNEL_ID")
        .expect("DISCORD_PR_CHANNEL_ID not set")
        .parse()
        .unwrap();

    let role_id: u64 = env::var("DISCORD_DEV_ROLE_ID")
        .expect("DISCORD_DEV_ROLE_ID not set")
        .parse()
        .unwrap();

    let message = format!(
        "<@&{}> New PR in **{}** by `{}`:\n**{}**\n`{}` → `{}`\n{}",
        role_id,
        payload.repository.full_name,
        payload.sender.login,
        payload.pull_request.title,
        payload.pull_request.head.r#ref,
        payload.pull_request.base.r#ref,
        payload.pull_request.html_url
    );

    let _ = ChannelId(channel_id)
        .send_message(&ctx.http, |m| m.content(message))
        .await;

    StatusCode::OK.into_response()
}
