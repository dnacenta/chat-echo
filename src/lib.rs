//! chat-echo — Web chat interface for AI entities.
//!
//! Provides a WebSocket-based chat relay between a browser frontend and
//! a backend LLM service (via bridge-echo). Includes a Nord-themed
//! responsive web UI served as static assets.
//!
//! # Usage as a library
//!
//! ```no_run
//! use chat_echo::{ChatEcho, config::Config};
//!
//! # fn run() {
//! let config = Config::from_env();
//! let mut chat = ChatEcho::new(config);
//! // chat.start().await.expect("server");
//! # }
//! ```

pub mod bridge;
pub mod config;
pub mod ws;

use std::net::SocketAddr;

use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use echo_system_types::{HealthStatus, SetupPrompt};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use bridge::BridgeClient;
use config::Config;

/// Shared application state accessible from all handlers.
#[derive(Clone)]
pub struct AppState {
    pub bridge: BridgeClient,
}

/// The chat-echo plugin. Manages the web chat interface lifecycle.
pub struct ChatEcho {
    config: Config,
    started: bool,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ChatEcho {
    /// Create a new ChatEcho instance from config.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            started: false,
            shutdown_tx: None,
        }
    }

    /// Start the chat server. Builds state, binds the listener, and serves.
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let state = AppState {
            bridge: BridgeClient::new(&self.config.bridge_url, self.config.bridge_secret.clone()),
        };

        let app = self.build_router(state);

        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| format!("Invalid address: {e}"))?;

        tracing::info!(%addr, "Listening");

        let listener = tokio::net::TcpListener::bind(addr).await?;

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);
        self.started = true;

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await?;

        Ok(())
    }

    /// Stop the chat server gracefully.
    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.started = false;
        Ok(())
    }

    /// Report health status.
    pub fn health(&self) -> HealthStatus {
        if self.started {
            HealthStatus::Healthy
        } else {
            HealthStatus::Down("not started".into())
        }
    }

    /// Return the Axum router with all chat-echo routes.
    pub fn routes(&self) -> Router<()> {
        let state = AppState {
            bridge: BridgeClient::new(&self.config.bridge_url, self.config.bridge_secret.clone()),
        };
        self.build_router(state)
    }

    /// Configuration prompts for the echo-system init wizard.
    pub fn setup_prompts() -> Vec<SetupPrompt> {
        vec![
            SetupPrompt {
                key: "bridge_url".into(),
                question: "Bridge URL (backend LLM endpoint):".into(),
                required: true,
                secret: false,
                default: Some("http://127.0.0.1:3100".into()),
            },
            SetupPrompt {
                key: "bridge_secret".into(),
                question: "Bridge secret (optional auth header):".into(),
                required: false,
                secret: true,
                default: None,
            },
            SetupPrompt {
                key: "static_dir".into(),
                question: "Static assets directory:".into(),
                required: false,
                secret: false,
                default: Some("./static".into()),
            },
        ]
    }

    fn build_router(&self, state: AppState) -> Router<()> {
        Router::new()
            .route("/ws", get(ws_handler))
            .route("/health", get(health_handler))
            .fallback_service(ServeDir::new(&self.config.static_dir))
            .layer(TraceLayer::new_for_http())
            .with_state(state)
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state.bridge))
}

async fn health_handler() -> &'static str {
    "ok"
}
