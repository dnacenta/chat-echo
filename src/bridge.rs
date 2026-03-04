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

/// HTTP client for the backend `/chat` endpoint.
#[derive(Clone)]
pub struct BridgeClient {
    client: Client,
    base_url: String,
    url: String,
    secret: Option<String>,
}

impl BridgeClient {
    pub fn new(bridge_url: &str, secret: Option<String>) -> Self {
        let base = bridge_url.trim_end_matches('/').to_string();
        Self {
            client: Client::new(),
            url: format!("{}/chat", base),
            base_url: base,
            secret,
        }
    }

    /// Fetch the dashboard JSON from the backend.
    pub async fn dashboard(&self) -> Result<String, String> {
        let url = format!("{}/api/dashboard", self.base_url);
        let mut http_req = self.client.get(&url);

        if let Some(secret) = &self.secret {
            http_req = http_req.header("X-Echo-Secret", secret);
        }

        let resp = http_req
            .send()
            .await
            .map_err(|e| format!("backend unreachable: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("backend returned {}", resp.status()));
        }

        resp.text()
            .await
            .map_err(|e| format!("invalid response: {e}"))
    }

    /// Send a message to the backend and return the response text.
    pub async fn send(&self, message: &str) -> Result<String, String> {
        let req = ChatRequest {
            message: message.to_string(),
            channel: "web".to_string(),
            sender: "user".to_string(),
        };

        let mut http_req = self.client.post(&self.url).json(&req);

        if let Some(secret) = &self.secret {
            http_req = http_req.header("X-Echo-Secret", secret);
        }

        let resp = http_req
            .send()
            .await
            .map_err(|e| format!("backend unreachable: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("backend returned {}", resp.status()));
        }

        let body: ChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("invalid response from backend: {e}"))?;

        Ok(body.response)
    }
}
