use axum::{
    routing::{get, post},
    Router,
    Server,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use std::path::Path;

mod server;
mod config;
mod setup;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = config::load_config().expect("Failed to load configuration");

    // Check if model directory exists
    let model_dir = Path::new(&config.models.model_directory);
    if !model_dir.exists() || !model_dir.is_dir() {
        return Err(format!("Model directory does not exist: {:?}", model_dir).into());
    }

    // Check if default model file exists
    let default_model_file = model_dir.join(&config.models.mappings[&config.models.default_model]);
    if !default_model_file.exists() || !default_model_file.is_file() {
        return Err(format!("Default model file does not exist: {:?}", default_model_file).into());
    }

    // Initialize the model context
    let model_context = setup::ModelContext::new(config.transcribe_module, config.models)
        .expect("Failed to initialize model context");

    // Build our application with routes
    let app = Router::new()
        .route("/transcribe", post(server::transcribe))
        .route("/transcription_status", post(server::get_transcription_status))
        .route("/transcription_result", post(server::get_transcription_result))
        .route("/load", post(server::load))
        .route("/list", get(server::list_models))
        .layer(CorsLayer::permissive())
        .with_state(model_context);

    // Run our application
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    tracing::info!("listening on {}", addr);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
