mod handlers;

use axum::{
    extract::{Json, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use crate::AppState;
use handlers::handle_pull_request_event;

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
            // Deserialize manually to expected type
            match serde_json::from_value(payload.0) {
                Ok(data) => handle_pull_request_event(State(state.0.clone()), Json(data)).await,
                Err(_) => StatusCode::BAD_REQUEST.into_response(),
            }
        }
        _ => StatusCode::NOT_IMPLEMENTED.into_response(), // For unhandled or missing event types
    }
}
