//! Anatomy parser (delegates to `axicor_core::config`).
use axicor_core::config::anatomy::AnatomyConfig as CoreAnatomy;
pub type Anatomy = CoreAnatomy;

/// Parses `anatomy.toml` from a string.
pub fn parse(content: &str) -> anyhow::Result<Anatomy> {
    Anatomy::parse(content).map_err(|e: String| anyhow::anyhow!(e))
}
