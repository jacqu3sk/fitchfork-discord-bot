mod handlers;

use axum::{
    extract::{Json, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use crate::AppState;
use handlers::{handle_pull_request_event, handle_review_requested_event, handle_workflow_run_event};

pub fn routes(shared_state: AppState) -> Router {
    Router::new().route("/github-webhook", post(dispatch_event).with_state(shared_state))
}

/// Main entry point for the GitHub webhook route.
/// In future this can match `X-GitHub-Event` header to dispatch different handlers.
async fn dispatch_event(
    headers: HeaderMap,
    state: State<AppState>,
    payload: Json<serde_json::Value>,
) -> Response {
    match headers.get("X-GitHub-Event") {
        Some(event_type) if event_type == HeaderValue::from_static("pull_request") => {
            let action = payload
                .get("action")
                .and_then(|a| a.as_str())
                .unwrap_or_default();

            match action {
                "opened" => match serde_json::from_value(payload.0) {
                    Ok(data) => handle_pull_request_event(State(state.0.clone()), Json(data)).await,
                    Err(_) => StatusCode::BAD_REQUEST.into_response(),
                },
                "review_requested" => match serde_json::from_value(payload.0) {
                    Ok(data) => handle_review_requested_event(State(state.0.clone()), Json(data)).await,
                    Err(_) => StatusCode::BAD_REQUEST.into_response(),
                },
                _ => StatusCode::OK.into_response(),
            }
        }
        Some(event_type) if event_type == HeaderValue::from_static("workflow_run") => {
            match serde_json::from_value(payload.0) {
                Ok(data) => handle_workflow_run_event(State(state.0.clone()), Json(data)).await,
                Err(_) => StatusCode::BAD_REQUEST.into_response(),
            }
        }
        _ => StatusCode::NOT_IMPLEMENTED.into_response(),
    }
}