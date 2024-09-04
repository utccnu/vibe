use axum::{
    extract::{State, Json},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use crate::setup::ModelContext;
use crate::cmd::{self, TranscribeOptions};

pub async fn transcribe(
    State(context): State<ModelContext>,
    Json(payload): Json<TranscribeOptions>,
) -> impl IntoResponse {
    // Implement transcription logic
}

pub async fn load(
    State(context): State<ModelContext>,
    Json(payload): Json<cmd::LoadPayload>,
) -> impl IntoResponse {
    // Implement model loading logic
}

pub async fn list_models(State(context): State<ModelContext>) -> impl IntoResponse {
    // Implement model listing logic
}
