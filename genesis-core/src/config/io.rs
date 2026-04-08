use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysId {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoConfig {
    #[serde(default)]
    pub input: Vec<IoMatrix>,
    #[serde(default)]
    pub output: Vec<IoMatrix>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoMatrix {
    pub matrix_id_v1: SysId,
    pub name: String,
    pub entry_z: String, // "top", "mid", "bottom" или точное значение
    #[serde(default)]
    pub pin: Vec<IoPin>, // В TOML это будет [[input.pin]]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoPin {
    pub pin_id_v1: SysId,
    pub name: String,
    // VRAM Payload / C-ABI Size
    pub width: u32,
    pub height: u32,
    // Normalized UV Projection (0.0 .. 1.0)
    pub local_u: f32,
    pub local_v: f32,
    pub u_width: f32,
    pub v_height: f32,
    // Routing & Biology
    pub target_type: String,
    pub stride: u32,
    #[serde(default = "default_growth_steps")]
    pub growth_steps: u32,
    #[serde(default = "default_empty_pixel")]
    pub empty_pixel: String,
}

fn default_growth_steps() -> u32 { 1000 }
fn default_empty_pixel() -> String { "skip".to_string() }
