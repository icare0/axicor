//! Парсер симуляции (делегируется к `axicor_core::config`).

pub use axicor_core::config::{SimulationConfig, SimulationParams, WorldConfig};

/// Парсит `simulation.toml` из строки, конвертируя `String` ошибку в `anyhow::Result`.
pub fn parse(src: &str) -> anyhow::Result<SimulationConfig> {
    SimulationConfig::parse(src).map_err(|e| anyhow::anyhow!(e))
}

