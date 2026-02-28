/// Configuration loaded from environment variables.
pub struct Config {
    pub host: String,
    pub port: u16,
    pub bridge_url: String,
    pub static_dir: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("CHAT_ECHO_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("CHAT_ECHO_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            bridge_url: std::env::var("CHAT_ECHO_BRIDGE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:3100".into()),
            static_dir: std::env::var("CHAT_ECHO_STATIC_DIR").unwrap_or_else(|_| "./static".into()),
        }
    }
}
