mod cli;
mod config;
mod core;
#[cfg(feature = "desktop")]
mod desktop;
mod server;

use clap::Parser;
use cli::{Cli, Commands};
use config::AppConfig;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let config = AppConfig::load(cli.config.as_ref())?;

    match cli.command {
        Commands::Serve { host, port, .. } => {
            let mut server_config = config.server.clone();
            server_config.host = host;
            server_config.port = port;

            if server_config.host != "127.0.0.1" && server_config.host != "localhost" {
                tracing::warn!(
                    "WARNING: Binding to {} is not designed for public/team deployment. \
                     This is a local personal tool.",
                    server_config.host
                );
                eprintln!(
                    "WARNING: Binding to {} is not designed for public/team deployment. \
                     This is a local personal tool.",
                    server_config.host
                );
            }

            let refresh_manager = Arc::new(core::refresh::RefreshManager::new(config));
            let state = server::AppState { refresh_manager };

            let frontend_path = std::env::current_dir()
                .unwrap_or_default()
                .join("frontend")
                .join("dist");

            let router = if frontend_path.exists() {
                server::create_router_with_frontend(state, frontend_path.to_str().unwrap_or("frontend/dist"))
            } else {
                server::create_router(state)
            };

            let addr = format!("{}:{}", server_config.host, server_config.port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;

            tracing::info!("Starting llm-usage dashboard on http://{}", addr);
            println!("llm-usage dashboard running at http://{}", addr);
            println!("Press Ctrl+C to stop");

            axum::serve(listener, router).await?;
        }

        Commands::Health => {
            let refresh_manager = core::refresh::RefreshManager::new(config);
            let health = refresh_manager.health_check().await;
            println!("{}", serde_json::to_string_pretty(&health)?);
        }

        Commands::Refresh => {
            let refresh_manager = core::refresh::RefreshManager::new(config);
            let response = refresh_manager.refresh().await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        Commands::Config => {
            println!("{}", AppConfig::config_path_display());
        }
    }

    Ok(())
}
