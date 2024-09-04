use axum::{
    routing::{get, post},
    Router,
    response::IntoResponse,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

mod server;
mod config;
mod setup;

async fn hello_world() -> impl IntoResponse {
    "Hello, World!"
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = config::load_config().expect("Failed to load configuration");

    // Initialize the model context
    let model_context = setup::ModelContext::new().expect("Failed to initialize model context");

    // Build our application with routes
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/transcribe", post(server::transcribe))
        .route("/load", post(server::load))
        .route("/list", get(server::list_models))
        .layer(CorsLayer::permissive())
        .with_state(model_context);

    // Run our application
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
