use axum::extract::ws::{Message, WebSocket};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::bridge::BridgeClient;

#[derive(Deserialize)]
struct IncomingMessage {
    message: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OutgoingMessage {
    Status { status: String },
    Response { response: String },
    Error { error: String },
}

/// Handle a WebSocket connection: receive user messages, relay to bridge-echo.
pub async fn handle_socket(mut socket: WebSocket, bridge: BridgeClient) {
    while let Some(Ok(msg)) = socket.next().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        let incoming: IncomingMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(_) => {
                let _ = send_json(
                    &mut socket,
                    &OutgoingMessage::Error {
                        error: "invalid message format".into(),
                    },
                )
                .await;
                continue;
            }
        };

        if incoming.message.trim().is_empty() {
            continue;
        }

        // Tell the client we're thinking
        let _ = send_json(
            &mut socket,
            &OutgoingMessage::Status {
                status: "thinking".into(),
            },
        )
        .await;

        // Relay to bridge-echo
        match bridge.send(&incoming.message).await {
            Ok(response) => {
                let _ = send_json(&mut socket, &OutgoingMessage::Response { response }).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "bridge-echo request failed");
                let _ = send_json(&mut socket, &OutgoingMessage::Error { error: e }).await;
            }
        }
    }

    tracing::info!("WebSocket connection closed");
}

async fn send_json(socket: &mut WebSocket, msg: &OutgoingMessage) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).unwrap_or_default();
    socket.send(Message::text(text)).await
}
