//! Парсер IO (делегируется к `axicor_core::config`).

pub use axicor_core::config::IoConfig;

/// Парсит `io.toml` из строки.
pub fn parse(src: &str) -> anyhow::Result<IoConfig> {
    toml::from_str(src).map_err(|e| anyhow::anyhow!(e))
}
