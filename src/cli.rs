use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "llm-usage")]
#[command(about = "Local-first personal LLM usage dashboard")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to config file
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the local web dashboard
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to bind to
        #[arg(long, default_value = "3766")]
        port: u16,

        /// Runner executable path
        #[arg(long)]
        runner: Option<String>,
    },

    /// Check runner and ccusage availability
    Health,

    /// Run one refresh and print normalized snapshot as JSON
    Refresh,

    /// Print resolved config file path
    Config,
}
