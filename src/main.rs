mod cli;
mod config;
mod core;
#[cfg(feature = "desktop")]
mod desktop;
mod server;

use clap::Parser;
use cli::{Cli, Commands};
use config::AppConfig;
use core::provider::{ProviderHealth, UsageProvider};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

struct CcUsageProvider {
    config: config::CcUsageConfig,
}

impl CcUsageProvider {
    fn new(config: config::CcUsageConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl UsageProvider for CcUsageProvider {
    async fn execute_cell(
        &self,
        cell: core::model::Cell,
    ) -> Result<String, core::error::ProviderError> {
        let planner = core::planner::CommandPlanner::new(self.config.clone());
        let cmd = planner.plan_cell(cell);

        let output = tokio::process::Command::new(&cmd.program)
            .args(&cmd.args)
            .output()
            .await?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(core::error::ProviderError::CommandFailed { exit_code, stderr });
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if stdout.len() > 10 * 1024 * 1024 {
            return Err(core::error::ProviderError::InvalidJson(
                "Output exceeds 10MB limit".to_string(),
            ));
        }

        serde_json::from_str::<serde_json::Value>(&stdout)
            .map_err(|e| core::error::ProviderError::InvalidJson(e.to_string()))?;

        Ok(stdout)
    }

    async fn health_check(&self) -> ProviderHealth {
        let runner_output = tokio::process::Command::new(&self.config.runner)
            .arg("--version")
            .output()
            .await;

        let (runner_found, runner_path, runner_version) = match runner_output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                (true, Some(self.config.runner.clone()), Some(version))
            }
            _ => (false, None, None),
        };

        let ccusage_output = tokio::process::Command::new(&self.config.runner)
            .arg(&self.config.package_spec)
            .arg("--version")
            .output()
            .await;

        let (ccusage_available, ccusage_version) = match ccusage_output {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                (true, Some(version))
            }
            _ => (false, None),
        };

        ProviderHealth {
            runner_found,
            runner_path,
            runner_version,
            ccusage_available,
            ccusage_version,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let config = AppConfig::load(cli.config.as_ref())?;

    match cli.command {
        Commands::Serve { host, port, runner } => {
            let mut server_config = config.server.clone();
            server_config.host = host;
            server_config.port = port;

            let mut ccusage_config = config.ccusage.clone();
            if let Some(r) = runner {
                ccusage_config.runner = r;
            }

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

            let provider = Arc::new(CcUsageProvider::new(ccusage_config));
            let refresh_manager = Arc::new(core::refresh::RefreshManager::new(config, provider));
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
            let provider = Arc::new(CcUsageProvider::new(config.ccusage.clone()));
            let refresh_manager = core::refresh::RefreshManager::new(config, provider);
            let health = refresh_manager.health_check().await;
            println!("{}", serde_json::to_string_pretty(&health)?);
        }

        Commands::Refresh => {
            let provider = Arc::new(CcUsageProvider::new(config.ccusage.clone()));
            let refresh_manager = core::refresh::RefreshManager::new(config, provider);
            let response = refresh_manager.refresh().await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        Commands::Config => {
            println!("{}", AppConfig::config_path_display());
        }
    }

    Ok(())
}
