use bevy::prelude::*;

use crate::{loader::WorldData, AppState};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpikeFrame>()
            .add_systems(OnEnter(AppState::Running), (spawn_neurons, spawn_chunk_grid))
            .add_systems(
                Update,
                (apply_spike_glow, tick_glow_timers)
                    .chain()
                    .run_if(in_state(AppState::Running)),
            );
    }
}

/// Neuron entity marker.
#[derive(Component)]
pub struct NeuronMarker {
    pub id: u32,
}

/// Counts down each frame; when 0 emissive is cleared.
#[derive(Component)]
pub struct GlowTimer(pub u8);

/// Event emitted by telemetry thread.
#[derive(Event)]
pub struct SpikeFrame {
    pub tick: u64,
    pub spike_ids: Vec<u32>,
}

/// Palette for neuron types (4 types, 0..3).
const TYPE_COLORS: [Color; 4] = [
    Color::srgb(0.3, 0.6, 1.0),  // 0 — Vertical Excitatory — blue
    Color::srgb(1.0, 0.4, 0.4),  // 1 — Horizontal Inhibitory — red
    Color::srgb(0.4, 1.0, 0.5),  // 2 — variant 2 — green
    Color::srgb(1.0, 0.9, 0.3),  // 3 — variant 3 — yellow
];

/// World scale: 1 voxel = 0.25 Bevy units (so 140-voxel world ≈ 35 units wide).
const VOXEL_SCALE: f32 = 0.25;

fn spawn_neurons(
    mut commands: Commands,
    world_data: Res<WorldData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if world_data.neurons.is_empty() {
        println!("[world] No neurons to display.");
        return;
    }

    // One mesh shared across all neurons of same type via separate MaterialMeshBundle.
    // For MVP: individual entities. Instancing upgrade can come later.
    let sphere_mesh = meshes.add(Sphere::new(0.12).mesh().ico(1).unwrap());

    println!("[world] Spawning {} neurons...", world_data.neurons.len());

    for (idx, neuron) in world_data.neurons.iter().enumerate() {
        let type_idx = (neuron.type_mask & 0x3) as usize;
        let color = TYPE_COLORS[type_idx];

        let mat = materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::BLACK,
            perceptual_roughness: 0.6,
            metallic: 0.1,
            ..default()
        });

        commands.spawn((
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::from_xyz(
                neuron.x * VOXEL_SCALE,
                neuron.z * VOXEL_SCALE, // Z in genesis = up, Y in Bevy = up
                neuron.y * VOXEL_SCALE,
            ),
            NeuronMarker { id: idx as u32 },
        ));
    }

    println!("[world] Neurons spawned.");
}

/// Chunk grid: draw lines every 10 voxels in all 3 axes.
fn spawn_chunk_grid(
    world_data: Res<WorldData>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if world_data.neurons.is_empty() {
        return;
    }

    // Determine rough world bounds from neurons.
    let max_x = world_data.neurons.iter().map(|n| n.x as u32).max().unwrap_or(139);
    let max_y = world_data.neurons.iter().map(|n| n.y as u32).max().unwrap_or(139);
    let max_z = world_data.neurons.iter().map(|n| n.z as u32).max().unwrap_or(409);

    // Chunk size in voxels.
    const CHUNK: u32 = 10;

    let grid_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.5, 0.6, 0.8, 0.08),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let line_mesh = meshes.add(Cuboid::new(0.01, 0.01, 1.0));

    // Draw grid lines along Z axis (vertical pillars at chunk corners).
    let x_chunks = (max_x / CHUNK) + 2;
    let y_chunks = (max_y / CHUNK) + 2;
    let z_height = (max_z as f32 + CHUNK as f32) * VOXEL_SCALE;

    for cx in 0..=x_chunks {
        for cy in 0..=y_chunks {
            let wx = cx as f32 * CHUNK as f32 * VOXEL_SCALE;
            let wy = cy as f32 * CHUNK as f32 * VOXEL_SCALE;

            commands.spawn((
                Mesh3d(line_mesh.clone()),
                MeshMaterial3d(grid_mat.clone()),
                Transform::from_xyz(wx, z_height * 0.5, wy)
                    .with_scale(Vec3::new(1.0, 1.0, z_height)),
            ));
        }
    }
}

/// Apply emissive glow to neurons from spike frame.
fn apply_spike_glow(
    mut events: EventReader<SpikeFrame>,
    mut query: Query<(&NeuronMarker, &MeshMaterial3d<StandardMaterial>, &mut GlowTimer)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    neurons_no_timer: Query<(Entity, &NeuronMarker, &MeshMaterial3d<StandardMaterial>), Without<GlowTimer>>,
) {
    for frame in events.read() {
        // Collect spike set for fast lookup.
        let spike_set: std::collections::HashSet<u32> =
            frame.spike_ids.iter().copied().collect();

        // Neurons that already have GlowTimer: refresh if in spike set.
        for (marker, mat_handle, mut timer) in &mut query {
            if spike_set.contains(&marker.id) {
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.emissive = LinearRgba::new(2.0, 1.5, 0.3, 1.0);
                }
                timer.0 = 3; // 3 frames glow
            }
        }

        // Neurons without timer: spawn GlowTimer + set emissive.
        for (entity, marker, mat_handle) in &neurons_no_timer {
            if spike_set.contains(&marker.id) {
                if let Some(mat) = materials.get_mut(&mat_handle.0) {
                    mat.emissive = LinearRgba::new(2.0, 1.5, 0.3, 1.0);
                }
                commands.entity(entity).insert(GlowTimer(3));
            }
        }
    }
}

/// Decrement GlowTimer each frame; clear emissive when expired.
fn tick_glow_timers(
    mut commands: Commands,
    mut query: Query<(Entity, &MeshMaterial3d<StandardMaterial>, &mut GlowTimer)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mat_handle, mut timer) in &mut query {
        if timer.0 == 0 {
            if let Some(mat) = materials.get_mut(&mat_handle.0) {
                mat.emissive = LinearRgba::BLACK;
            }
            commands.entity(entity).remove::<GlowTimer>();
        } else {
            timer.0 -= 1;
        }
    }
}
