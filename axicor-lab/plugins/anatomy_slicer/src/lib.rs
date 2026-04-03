pub mod domain;
pub mod systems;
pub mod cad_glass_material;

use bevy::prelude::*;

pub struct AnatomySlicerPlugin;

impl Plugin for AnatomySlicerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(MaterialPlugin::<cad_glass_material::CadGlassMaterial>::default())
            .add_systems(Update, (
                systems::interaction::init_slicer_windows_system,
                systems::interaction::sync_active_zone_system,
                systems::cad_inspector::vram::allocate_vram_system,
                systems::cad_inspector::vram::sync_vram_system,
                systems::cad_inspector::camera::spawn_cad_camera_system,
                systems::cad_inspector::camera::sync_camera_aspect_system,
                systems::cad_inspector::camera::cad_camera_control_system,
                systems::cad_inspector::geometry::refresh_cad_geometry_on_change_system,
                systems::cad_inspector::geometry::spawn_cad_geometry_system,
                systems::render::render_anatomy_slicer_system,
                systems::cad_inspector::geometry::sync_hover_plane_system,
                systems::cad_inspector::raycast::dnd_raycast_system,
                systems::cad_inspector::cleanup::cleanup_cad_scene_system,
            ).chain());
    }
}
