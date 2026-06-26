mod agent;
mod config;
mod error;

use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "anti-gav-ty-agent")]
#[command(version = "0.1.0")]
#[command(about = "Gateway agent for anti-gav-ty platform")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "agent.yaml")]
    config: String,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with(
            fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true),
        )
        .init();

    let cli = Cli::parse();

    tracing::info!("starting anti-gav-ty agent v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let cfg = match config::load(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // Build and run agent
    let mut agent = match agent::Agent::new(cfg).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("failed to create agent: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = agent.run().await {
        tracing::error!("agent error: {}", e);
        std::process::exit(1);
    }
}
