/// Configuration loaded from environment variables.
pub struct Config {
    pub host: String,
    pub port: u16,
    pub bridge_url: String,
    pub bridge_secret: Option<String>,
    pub static_dir: String,
}

/// Helper to extract a string from a JSON object, with default.
fn json_str(obj: &serde_json::Map<String, serde_json::Value>, key: &str, default: &str) -> String {
    obj.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

/// Helper to extract an optional string from a JSON object.
fn json_opt_str(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    obj.get(key).and_then(|v| v.as_str()).map(String::from)
}

impl Config {
    /// Create a Config from a JSON value (used by the Plugin factory).
    pub fn from_json(value: &serde_json::Value) -> Self {
        let obj = value.as_object().cloned().unwrap_or_default();
        Self {
            host: json_str(&obj, "host", "0.0.0.0"),
            port: obj.get("port").and_then(|v| v.as_u64()).unwrap_or(8080) as u16,
            bridge_url: json_str(&obj, "bridge_url", "http://127.0.0.1:3100"),
            bridge_secret: json_opt_str(&obj, "bridge_secret"),
            static_dir: json_str(&obj, "static_dir", "./static"),
        }
    }

    pub fn from_env() -> Self {
        Self {
            host: std::env::var("CHAT_ECHO_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("CHAT_ECHO_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            bridge_url: std::env::var("CHAT_ECHO_BRIDGE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3100".into()),
            bridge_secret: std::env::var("CHAT_ECHO_BRIDGE_SECRET").ok(),
            static_dir: std::env::var("CHAT_ECHO_STATIC_DIR").unwrap_or_else(|_| "./static".into()),
        }
    }
}
