use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    message: String,
    channel: String,
    sender: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    response: String,
}

/// HTTP client for bridge-echo's `/chat` endpoint.
#[derive(Clone)]
pub struct BridgeClient {
    client: Client,
    url: String,
}

impl BridgeClient {
    pub fn new(bridge_url: &str) -> Self {
        Self {
            client: Client::new(),
            url: format!("{}/chat", bridge_url.trim_end_matches('/')),
        }
    }

    /// Send a message to bridge-echo and return the response text.
    pub async fn send(&self, message: &str) -> Result<String, String> {
        let req = ChatRequest {
            message: message.to_string(),
            channel: "web".to_string(),
            sender: "user".to_string(),
        };

        let resp = self
            .client
            .post(&self.url)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("bridge-echo unreachable: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("bridge-echo returned {}", resp.status()));
        }

        let body: ChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("invalid response from bridge-echo: {e}"))?;

        Ok(body.response)
    }
}
