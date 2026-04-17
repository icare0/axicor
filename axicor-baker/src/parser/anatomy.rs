//! Парсер анатомии (делегируется к `axicor_core::config`).

pub use axicor_core::config::AnatomyConfig as Anatomy;

/// Парсит `anatomy.toml` из строки.
pub fn parse(src: &str) -> anyhow::Result<Anatomy> {
    Anatomy::parse(src).map_err(|e| anyhow::anyhow!(e))
}

