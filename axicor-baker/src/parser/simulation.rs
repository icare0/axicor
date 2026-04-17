//! Simulation parser (delegates to `axicor_core::config`).
use axicor_core::config::SimulationConfig as CoreSim;
pub type SimulationConfig = CoreSim;

/// Parses `simulation.toml` from a string, converting `String` error to `anyhow::Result`.
pub fn parse(content: &str) -> anyhow::Result<SimulationConfig> {
    SimulationConfig::parse(content).map_err(|e: String| anyhow::anyhow!(e))
}
