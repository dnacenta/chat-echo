//! chat-echo — Web chat interface for AI entities.
//!
//! Provides a WebSocket-based chat relay between a browser frontend and
//! a backend LLM service (via bridge-echo). Includes a Nord-themed
//! responsive web UI with assets embedded in the binary.
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

use std::any::Any;
use std::future::Future;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;

use axum::extract::{State, WebSocketUpgrade};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use echo_system_types::plugin::{Plugin, PluginContext, PluginResult, PluginRole};
use echo_system_types::{HealthStatus, PluginMeta, SetupPrompt};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use bridge::BridgeClient;
use config::Config;

// Embed static assets at compile time so the binary is self-contained.
const EMBEDDED_HTML: &str = include_str!("../static/index.html");
const EMBEDDED_JS: &str = include_str!("../static/chat.js");
const EMBEDDED_CSS: &str = include_str!("../static/style.css");
const EMBEDDED_FONT_REGULAR: &[u8] = include_bytes!("../static/fonts/0xProto-Regular.woff2");
const EMBEDDED_FONT_BOLD: &[u8] = include_bytes!("../static/fonts/0xProto-Bold.woff2");
const EMBEDDED_FONT_ITALIC: &[u8] = include_bytes!("../static/fonts/0xProto-Italic.woff2");

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

    /// Report health status.
    fn health_check(&self) -> HealthStatus {
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
    fn get_setup_prompts() -> Vec<SetupPrompt> {
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
        ]
    }

    fn build_router(&self, state: AppState) -> Router<()> {
        let router = Router::new()
            .route("/ws", get(ws_handler))
            .route("/health", get(health_handler))
            .route("/api/dashboard", get(dashboard_proxy))
            .route("/chat.js", get(serve_js))
            .route("/style.css", get(serve_css))
            .route("/fonts/0xProto-Regular.woff2", get(serve_font_regular))
            .route("/fonts/0xProto-Bold.woff2", get(serve_font_bold))
            .route("/fonts/0xProto-Italic.woff2", get(serve_font_italic));

        // If static_dir exists on disk, use it as fallback (allows overrides).
        // Otherwise, serve embedded index.html for all unmatched routes.
        let router = if Path::new(&self.config.static_dir).is_dir() {
            router.fallback_service(ServeDir::new(&self.config.static_dir))
        } else {
            router.fallback(serve_index)
        };

        router.layer(TraceLayer::new_for_http()).with_state(state)
    }
}

/// Factory function — creates a fully initialized chat-echo plugin.
pub async fn create(
    config: &serde_json::Value,
    _ctx: &PluginContext,
) -> Result<Box<dyn Plugin>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Box::new(ChatEcho::new(Config::from_json(config))))
}

impl Plugin for ChatEcho {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            name: "chat-echo".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "Web chat UI for pulse-null".into(),
        }
    }

    fn role(&self) -> PluginRole {
        PluginRole::Interface
    }

    fn start(&mut self) -> PluginResult<'_> {
        Box::pin(async move {
            let state = AppState {
                bridge: BridgeClient::new(
                    &self.config.bridge_url,
                    self.config.bridge_secret.clone(),
                ),
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
        })
    }

    fn stop(&mut self) -> PluginResult<'_> {
        Box::pin(async move {
            if let Some(tx) = self.shutdown_tx.take() {
                let _ = tx.send(());
            }
            self.started = false;
            Ok(())
        })
    }

    fn health(&self) -> Pin<Box<dyn Future<Output = HealthStatus> + Send + '_>> {
        Box::pin(async move { self.health_check() })
    }

    fn setup_prompts(&self) -> Vec<SetupPrompt> {
        Self::get_setup_prompts()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws::handle_socket(socket, state.bridge))
}

async fn health_handler() -> &'static str {
    "ok"
}

async fn dashboard_proxy(State(state): State<AppState>) -> Response {
    match state.bridge.dashboard().await {
        Ok(json) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            json,
        )
            .into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn serve_index() -> Html<&'static str> {
    Html(EMBEDDED_HTML)
}

async fn serve_js() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/javascript")],
        EMBEDDED_JS,
    )
        .into_response()
}

async fn serve_css() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/css")],
        EMBEDDED_CSS,
    )
        .into_response()
}

async fn serve_font_regular() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "font/woff2")],
        EMBEDDED_FONT_REGULAR,
    )
        .into_response()
}

async fn serve_font_bold() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "font/woff2")],
        EMBEDDED_FONT_BOLD,
    )
        .into_response()
}

async fn serve_font_italic() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "font/woff2")],
        EMBEDDED_FONT_ITALIC,
    )
        .into_response()
}
