use crate::domain::{AnatomySlicerState, ShardCadEntity};
use bevy::prelude::*;
use bevy_egui::EguiContexts;
use node_editor::domain::BrainTopologyGraph;

fn intersect_shard(ray: Ray3d, w: f32, h: f32, d: f32) -> Option<(Vec3, u32)> {
    let ray_origin = ray.origin;
    let ray_dir: Vec3 = ray.direction.into();
    let inv_dir = 1.0 / ray_dir;

    let min = Vec3::new(-w / 2.0, -h / 2.0, -d / 2.0);
    let max = Vec3::new(w / 2.0, h / 2.0, d / 2.0);

    let t0 = (min - ray_origin) * inv_dir;
    let t1 = (max - ray_origin) * inv_dir;

    let tmin = t0.min(t1).max_element();
    let tmax = t0.max(t1).min_element();

    if tmax >= tmin && tmin >= 0.0 {
        let hit_pos = ray_origin + ray_dir * tmin;
        let voxel_z = (hit_pos.y + h / 2.0).floor().clamp(0.0, h - 1.0) as u32;
        Some((hit_pos, voxel_z))
    } else {
        None
    }
}

pub fn dnd_raycast_system(
    mut contexts: EguiContexts,
    mut ui_states: Query<(Entity, &mut AnatomySlicerState)>,
    cameras: Query<(&Camera, &GlobalTransform), With<ShardCadEntity>>,
    mut ctx_menu_events: EventWriter<layout_api::OpenContextMenuEvent>,
    graph: Res<BrainTopologyGraph>,
) {
    let Some((window_entity, mut state)) =
        ui_states.iter_mut().find(|(_, s)| s.active_zone.is_some())
    else {
        return;
    };
    let Ok((camera, cam_transform)) = cameras.get_single() else {
        return;
    };

    let mut h = 32.0;
    let mut w = 32.0;
    let mut d = 32.0;
    let mut target_zone = String::new();

    if let Some(ref shard_name) = state.active_zone {
        target_zone = shard_name.clone();
        if let Some(active_path) = &graph.active_path {
            if let Some(session) = graph.sessions.get(active_path) {
                if let Some(anatomy) = session.shard_anatomies.get(shard_name) {
                    h = anatomy.h;
                    w = anatomy.w;
                    d = anatomy.d;
                }
            }
        }
    }

    // 1. Process Connection Drop
    let payload_id = bevy_egui::egui::Id::new("io_wire_drag");
    let ctx = contexts.ctx_mut();

    if ctx.input(|i| i.pointer.any_released()) {
        if let Some(payload) =
            ctx.memory(|m| m.data.get_temp::<layout_api::IoWirePayload>(payload_id))
        {
            if let Some(mouse_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if let Some(ray) = camera.viewport_to_world(
                    cam_transform,
                    bevy::math::Vec2::new(mouse_pos.x, mouse_pos.y),
                ) {
                    if let Some((_, voxel_z)) = intersect_shard(ray, w, h, d) {
                        let (from_zone, from_port, to_zone, to_port) = if payload.is_input {
                            (
                                target_zone.clone(),
                                "out".to_string(),
                                payload.zone.clone(),
                                payload.port.clone(),
                            )
                        } else {
                            (
                                payload.zone.clone(),
                                payload.port.clone(),
                                target_zone.clone(),
                                "in".to_string(),
                            )
                        };

                        ctx_menu_events.send(layout_api::OpenContextMenuEvent {
                            target_window: window_entity,
                            position: mouse_pos,
                            actions: vec![
                                layout_api::MenuAction {
                                    action_id: format!(
                                        "node_editor.connect_matrix|{}|{}|{}|{}|{}",
                                        from_zone, from_port, to_zone, to_port, voxel_z
                                    ),
                                    label: format!(" Connect to Z-Voxel {}", voxel_z),
                                },
                                layout_api::MenuAction {
                                    action_id: format!(
                                        "node_editor.connect_global|{}|{}|{}",
                                        from_zone, from_port, to_zone
                                    ),
                                    label: " Map to Global UV Atlas".into(),
                                },
                            ],
                        });
                        info!("[DND] Cross-Plugin Raycast Hit Drop: Voxel Z={}", voxel_z);
                    }
                }
            }
        }
    }

    // 2. Process Hover State
    // DOD FIX: Slicer highlights target Z-index with a Hover Plane
    if let Some(payload) = ctx.memory(|m| m.data.get_temp::<layout_api::IoWirePayload>(payload_id))
    {
        if let Some(mouse_pos) = ctx.input(|i| i.pointer.latest_pos()) {
            if let Some(rect) = state.cad_viewport_rect {
                if rect.contains(mouse_pos) {
                    let local_pos = mouse_pos - rect.min;
                    if let Some(ray) = camera.viewport_to_world(
                        cam_transform,
                        bevy::math::Vec2::new(local_pos.x, local_pos.y),
                    ) {
                        if let Some((hit_pos, voxel_z)) = intersect_shard(ray, w, h, d) {
                            let center_y = -h / 2.0 + voxel_z as f32 + 0.5;
                            let snap_world_pos = Vec3::new(
                                if payload.is_input { -w / 2.0 } else { w / 2.0 },
                                center_y,
                                hit_pos.z,
                            );

                            if let Some(viewport_pos) =
                                camera.world_to_viewport(cam_transform, snap_world_pos)
                            {
                                let screen_snap = bevy_egui::egui::Pos2::new(
                                    rect.min.x + viewport_pos.x,
                                    rect.min.y + viewport_pos.y,
                                );
                                state.active_3d_hover = Some((screen_snap, voxel_z));
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    state.active_3d_hover = None;
}
