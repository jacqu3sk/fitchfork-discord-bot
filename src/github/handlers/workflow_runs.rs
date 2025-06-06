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
pub struct WorkflowRunEvent {
    pub action: String,
    pub workflow_run: WorkflowRun,
    pub repository: Repository,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRun {
    pub html_url: String,
    pub name: String,
    pub status: Option<String>,
    pub conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub full_name: String,
}

pub async fn handle_workflow_run_event(
    State(state): State<AppState>,
    Json(payload): Json<WorkflowRunEvent>,
) -> Response {
    // Only notify on completed workflow runs
    if payload.action != "completed" {
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

    let channel_id: u64 = env::var("DISCORD_WORKFLOW_CHANNEL_ID")
        .expect("DISCORD_WORKFLOW_CHANNEL_ID not set")
        .parse()
        .unwrap();

    let message = format!(
        "Workflow run **{}** in **{}** completed with status `{}` and result `{}`:\n{}",
        payload.workflow_run.name,
        payload.repository.full_name,
        payload.workflow_run.status.as_deref().unwrap_or("unknown"),
        payload.workflow_run.conclusion.as_deref().unwrap_or("unknown"),
        payload.workflow_run.html_url
    );

    let _ = ChannelId(channel_id)
        .send_message(&ctx.http, |m| m.content(message))
        .await;

    StatusCode::OK.into_response()
}
