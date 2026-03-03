use chat_echo::config::Config;
use chat_echo::ChatEcho;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

    let mut chat = ChatEcho::new(config);

    if let Err(e) = chat.start().await {
        tracing::error!("Server error: {e}");
        std::process::exit(1);
    }
}
