use bevy::prelude::*;
use std::path::PathBuf;

use crate::AppState;

pub struct LoaderPlugin;

impl Plugin for LoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldData>()
            .add_systems(OnEnter(AppState::Loading), start_loading)
            .add_systems(Update, check_load_done.run_if(in_state(AppState::Loading)));
    }
}

/// Decoded neuron position and type.
#[derive(Clone, Copy, Debug)]
pub struct NeuronEntry {
    /// World position in voxel units.
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// 4-bit type mask (0..15).
    pub type_mask: u8,
}

impl NeuronEntry {
    /// Decode from packed_pos u32: [type(4b)|z(8b)|y(10b)|x(10b)]
    pub fn from_packed(packed: u32) -> Self {
        let x = (packed & 0x3FF) as f32;
        let y = ((packed >> 10) & 0x3FF) as f32;
        let z = ((packed >> 20) & 0xFF) as f32;
        let type_mask = (packed >> 28) as u8;
        NeuronEntry { x, y, z, type_mask }
    }
}

/// Loaded world geometry — available once Loading is complete.
#[derive(Resource, Default)]
pub struct WorldData {
    pub neurons: Vec<NeuronEntry>,
    pub loaded: bool,
}

/// Path to baked positions. Could be CLI arg — hardcoded for MVP.
fn positions_path() -> PathBuf {
    PathBuf::from("baked/shard.positions")
}

fn start_loading(mut world_data: ResMut<WorldData>) {
    match std::fs::read(positions_path()) {
        Ok(bytes) => {
            if bytes.len() % 4 != 0 {
                eprintln!("[loader] shard.positions size not multiple of 4 — corrupt file");
                return;
            }
            let neurons: Vec<NeuronEntry> = bytes
                .chunks_exact(4)
                .map(|b| {
                    let packed = u32::from_le_bytes(b.try_into().unwrap());
                    NeuronEntry::from_packed(packed)
                })
                .collect();
            println!("[loader] Loaded {} neurons from shard.positions", neurons.len());
            world_data.neurons = neurons;
            world_data.loaded = true;
        }
        Err(e) => {
            eprintln!(
                "[loader] Cannot read {}: {e}. Starting empty world.",
                positions_path().display()
            );
            // Start with empty world so IDE is usable without baked data.
            world_data.loaded = true;
        }
    }
}

fn check_load_done(
    world_data: Res<WorldData>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if world_data.loaded {
        next_state.set(AppState::Running);
    }
}
