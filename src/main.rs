mod bridge;
mod config;
mod ws;

use std::net::SocketAddr;

use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use bridge::BridgeClient;
use config::Config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
struct AppState {
    bridge: BridgeClient,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "chat_echo=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env();

    tracing::info!(
        version = VERSION,
        bridge_url = %config.bridge_url,
        static_dir = %config.static_dir,
        "Starting chat-echo"
    );

    let state = AppState {
        bridge: BridgeClient::new(&config.bridge_url, config.bridge_secret),
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health))
        .fallback_service(ServeDir::new(&config.static_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid address");

    tracing::info!(%addr, "Listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app).await.expect("Server error");
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state.bridge))
}

async fn health() -> &'static str {
    "ok"
}
