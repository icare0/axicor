//! IO parser (delegates to `axicor_core::config`).
use axicor_core::config::io::IoConfig as CoreIo;
pub type IoConfig = CoreIo;

/// Parses `io.toml` from a string.
pub fn parse(content: &str) -> anyhow::Result<IoConfig> {
    IoConfig::parse(content).map_err(|e: String| anyhow::anyhow!(e))
}
