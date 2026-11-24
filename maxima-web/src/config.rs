//! Server configuration and CLI argument parsing for maxima-web.
//!
//! Complies with the workspace configuration rule: relies strictly on explicit
//! CLI flags and optional TOML configuration files; zero system environment variables.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Command-line arguments for maxima-web.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "maxima-web",
    about = "Offline-first standalone webUI and CAS API for maxima-rs",
    version
)]
pub struct Cli {
    /// Bind address (e.g. 127.0.0.1 or 0.0.0.0)
    #[arg(long, default_value = "127.0.0.1")]
    pub bind: String,

    /// TCP port to listen on
    #[arg(long, default_value_t = 3000)]
    pub port: u16,

    /// Maximum computation timeout in milliseconds
    #[arg(long, default_value_t = 8000)]
    pub timeout_ms: u64,

    /// Maximum concurrent compute executions (defaults to logical CPU cores)
    #[arg(long)]
    pub concurrency: Option<usize>,

    /// Maximum request body size in bytes
    #[arg(long, default_value_t = 65536)]
    pub max_body_size: usize,

    /// Maximum expression string length in characters
    #[arg(long, default_value_t = 10000)]
    pub max_expr_len: usize,

    /// Path to an optional TOML configuration file
    #[arg(long)]
    pub config: Option<PathBuf>,
}

/// Runtime configuration for the maxima-web service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
    pub timeout_ms: u64,
    pub concurrency: usize,
    pub max_body_size: usize,
    pub max_expr_len: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let default_concurrency = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            bind: "127.0.0.1".to_string(),
            port: 3000,
            timeout_ms: 8000,
            concurrency: default_concurrency,
            max_body_size: 64 * 1024,
            max_expr_len: 10_000,
        }
    }
}

impl ServerConfig {
    /// Load server configuration from CLI flags and an optional TOML file.
    pub fn from_cli(cli: &Cli) -> Result<Self, String> {
        let mut config = if let Some(config_path) = &cli.config {
            Self::load_from_file(config_path)?
        } else {
            Self::default()
        };

        // Override with explicit CLI flags
        config.bind = cli.bind.clone();
        config.port = cli.port;
        config.timeout_ms = cli.timeout_ms;
        if let Some(c) = cli.concurrency {
            config.concurrency = c;
        }
        config.max_body_size = cli.max_body_size;
        config.max_expr_len = cli.max_expr_len;

        Ok(config)
    }

    /// Load server configuration from a TOML file.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {}: {e}", path.display()))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse TOML config file {}: {e}", path.display()))
    }
}
