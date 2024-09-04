use axum::{
    routing::get,
    Router,
    Server,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run our app with hyper, listening on all interfaces
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
