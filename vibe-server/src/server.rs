use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use crate::setup::ModelContext;

pub async fn transcribe(State(_context): State<ModelContext>) -> impl IntoResponse {
    Json(json!({"message": "Hello from transcribe!"}))
}

pub async fn load(State(_context): State<ModelContext>) -> impl IntoResponse {
    Json(json!({"message": "Hello from load!"}))
}

pub async fn list_models(State(_context): State<ModelContext>) -> impl IntoResponse {
    Json(json!({"message": "Hello from list_models!"}))
}
