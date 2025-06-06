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
pub struct PullRequestReviewRequestedEvent {
    pub action: String,
    pub pull_request: PullRequest,
    pub repository: Repository,
    pub requested_reviewer: Option<User>,
    pub sender: User,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub html_url: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub full_name: String,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
}

/// Tries to map a GitHub username to a Discord mention via env var like GITHUB_NOTIFY_username
fn discord_mention_for_github_user(username: &str) -> Option<String> {
    let key = format!("GITHUB_NOTIFY_{}", username);
    env::var(key).ok()
}

pub async fn handle_review_requested_event(
    State(state): State<AppState>,
    Json(payload): Json<PullRequestReviewRequestedEvent>,
) -> Response {
    if payload.action != "review_requested" {
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

    let requester = payload.sender.login;
    let reviewer_login = payload
        .requested_reviewer
        .as_ref()
        .map(|r| r.login.clone())
        .unwrap_or_else(|| "(unknown)".to_string());

    let reviewer_display = discord_mention_for_github_user(&reviewer_login)
        .unwrap_or_else(|| format!("`{}`", reviewer_login));

    let message = format!(
        "`{}` requested a review from {} on PR in **{}**:\n**{}**\n{}",
        requester,
        reviewer_display,
        payload.repository.full_name,
        payload.pull_request.title,
        payload.pull_request.html_url
    );

    let _ = ChannelId(channel_id)
        .send_message(&ctx.http, |m| m.content(message))
        .await;

    StatusCode::OK.into_response()
}
