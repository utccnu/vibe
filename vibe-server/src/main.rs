use axum::{
    routing::{get, post},
    Router,
};
use axum::extract::DefaultBodyLimit;  // Add this line
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::path::{Path, PathBuf};  // Add PathBuf here
use tower_http::cors::CorsLayer;
use std::env;

mod server;
mod config;
mod setup;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get config file path from command line argument or use default
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("config.toml"));

    // Load configuration
    let config = config::load_config(&config_path).expect("Failed to load configuration");

    // Check if model directory exists
    let model_dir = Path::new(&config.models.model_directory);
    if !model_dir.exists() || !model_dir.is_dir() {
        return Err(eyre::eyre!("Model directory does not exist: {:?}", model_dir));
    }

    // Check if default model file exists
    let default_model_file = model_dir.join(&config.models.mappings[&config.models.default_model]);
    if !default_model_file.exists() || !default_model_file.is_file() {
        return Err(eyre::eyre!("Default model file does not exist: {:?}", default_model_file));
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
		.layer(DefaultBodyLimit::max(1024 * 1024 * 100)) // Set to 100MB or adjust as needed
        .with_state(model_context);

    // Run our application
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
