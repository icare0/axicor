use bevy::prelude::*;

use crate::{
    camera::{CameraMode, CameraSpeed, FpsCamera},
    AppState,
};

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Running), spawn_hud)
            .add_systems(
                Update,
                update_hud.run_if(in_state(AppState::Running)),
            );
    }
}

/// Currently selected / hovered neuron ID.
#[derive(Resource, Default)]
pub struct SelectedNeuron(pub Option<u32>);

/// UI text node markers.
#[derive(Component)]
struct HudPosText;

#[derive(Component)]
struct HudChunkText;

#[derive(Component)]
struct HudNeuronText;

#[derive(Component)]
struct HudSpeedText;

#[derive(Component)]
struct HudModeText;

fn spawn_hud(mut commands: Commands) {
    let text_style = TextFont {
        font_size: 14.0,
        ..default()
    };
    let dim_color = TextColor(Color::srgba(0.7, 0.8, 1.0, 0.85));

    // Root container — bottom-left overlay.
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|parent| {
            // Mode indicator.
            parent.spawn((
                Text::new("[ CAM OFF ]"),
                text_style.clone(),
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
                HudModeText,
            ));

            // Speed.
            parent.spawn((
                Text::new("Speed: 5.0"),
                text_style.clone(),
                dim_color.clone(),
                HudSpeedText,
            ));

            // Position.
            parent.spawn((
                Text::new("Pos: X: 0.0  Y: 0.0  Z: 0.0"),
                text_style.clone(),
                dim_color.clone(),
                HudPosText,
            ));

            // Chunk.
            parent.spawn((
                Text::new("Chunk: [0, 0, 0]"),
                text_style.clone(),
                dim_color.clone(),
                HudChunkText,
            ));

            // Selected neuron (hidden when none).
            parent.spawn((
                Text::new(""),
                text_style.clone(),
                TextColor(Color::srgba(1.0, 0.9, 0.4, 0.95)),
                HudNeuronText,
            ));
        });
}

const VOXEL_SCALE: f32 = 0.25;
const CHUNK_VOXELS: f32 = 10.0;

fn update_hud(
    cam_query: Query<&Transform, With<FpsCamera>>,
    speed: Res<CameraSpeed>,
    cam_mode: Res<CameraMode>,
    selected: Res<SelectedNeuron>,
    mut pos_text: Query<&mut Text, (With<HudPosText>, Without<HudChunkText>, Without<HudNeuronText>, Without<HudSpeedText>, Without<HudModeText>)>,
    mut chunk_text: Query<&mut Text, (With<HudChunkText>, Without<HudPosText>, Without<HudNeuronText>, Without<HudSpeedText>, Without<HudModeText>)>,
    mut neuron_text: Query<&mut Text, (With<HudNeuronText>, Without<HudPosText>, Without<HudChunkText>, Without<HudSpeedText>, Without<HudModeText>)>,
    mut speed_text: Query<&mut Text, (With<HudSpeedText>, Without<HudPosText>, Without<HudChunkText>, Without<HudNeuronText>, Without<HudModeText>)>,
    mut mode_text: Query<&mut Text, (With<HudModeText>, Without<HudPosText>, Without<HudChunkText>, Without<HudNeuronText>, Without<HudSpeedText>)>,
) {
    let Ok(cam_transform) = cam_query.get_single() else {
        return;
    };

    // Camera position in voxel-space (reverse VOXEL_SCALE).
    let pos = cam_transform.translation / VOXEL_SCALE;
    // In Bevy layout: Bevy Y = Genesis Z, Bevy X = Genesis X, Bevy Z = Genesis Y.
    let gx = pos.x;
    let gy = pos.z; // genesis Y
    let gz = pos.y; // genesis Z (up)

    // Chunk index.
    let cx = (gx / CHUNK_VOXELS).floor() as i32;
    let cy = (gy / CHUNK_VOXELS).floor() as i32;
    let cz = (gz / CHUNK_VOXELS).floor() as i32;

    if let Ok(mut t) = pos_text.get_single_mut() {
        *t = Text::new(format!("Pos: X:{:.1}  Y:{:.1}  Z:{:.1}", gx, gy, gz));
    }
    if let Ok(mut t) = chunk_text.get_single_mut() {
        *t = Text::new(format!("Chunk: [{}, {}, {}]", cx, cy, cz));
    }
    if let Ok(mut t) = neuron_text.get_single_mut() {
        *t = Text::new(match selected.0 {
            Some(id) => format!("Neuron ID: {:#010X}", id),
            None => String::new(),
        });
    }
    if let Ok(mut t) = speed_text.get_single_mut() {
        *t = Text::new(format!("Speed: {:.1}", speed.0));
    }
    if let Ok(mut t) = mode_text.get_single_mut() {
        *t = Text::new(if cam_mode.active {
            "[ CAM ON  ]  Alt to exit".to_string()
        } else {
            "[ CAM OFF ]  Alt to enter".to_string()
        });
    }
}
