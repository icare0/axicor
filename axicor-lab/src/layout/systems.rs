use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowMode},
    app::AppExit,
    render::render_resource::{
        Extent3d, TextureDimension, TextureFormat, TextureUsages,
    },
    render::render_asset::RenderAssetUsages,
    winit::WinitWindows,
};
use bevy_egui::{egui, EguiContexts};
use egui_tiles::{LinearDir, Tile, Container, Linear, SimplificationOptions};
use std::collections::HashMap;
use crate::layout::data::*;
use crate::layout::behavior::PaneBehavior;

/// Allocates an Image for RTT usage.
pub fn create_plugin_render_target(images: &mut Assets<Image>, width: u32, height: u32) -> Handle<Image> {
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    images.add(image)
}

pub fn render_workspace_system(
    mut contexts: EguiContexts,
    tree_res: Option<ResMut<WorkspaceTree>>,
    mut drag_state: ResMut<WindowDragState>,
    mut topology: ResMut<TopologyCache>,
    fs_cache: Res<ProjectFsCache>,
    windows: Query<(Entity, &PluginWindow)>,
    mut input_query: Query<&mut PluginInput>,
    mut geometry_query: Query<&mut PluginGeometry>,
    mut zone_events: EventWriter<ZoneSelectedEvent>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut app_exit: EventWriter<AppExit>,
    mut drag_request: ResMut<layout_api::WindowDragRequest>, // ДОБАВЛЕНО
) {
    // Безопасный захват
    let Ok(mut window) = window_query.get_single_mut() else { return; };
    let mut tree_res = match tree_res {
        Some(res) => res,
        None => return,
    };

    let mut panes = HashMap::new();
    for (entity, plugin) in windows.iter() {
        let texture_id = plugin.texture.as_ref().map(|t| contexts.add_image(t.clone()));
        panes.insert(entity, PaneData { 
            domain: plugin.domain, 
            texture_id,
        });
    }

    let mut behavior = PaneBehavior { 
        panes: &panes, 
        drag_state: &mut drag_state,
        rects: &mut topology.rects,
        input_updates: Vec::new(),
        geometry_updates: Vec::new(),
        zone_events: Vec::new(),
        fs_cache: &fs_cache,
    };

    let ctx = contexts.ctx_mut().clone();

    // 1. Глобальный Top Bar (Borderless Control)
    egui::TopBottomPanel::top("axicor_top_bar")
        .frame(egui::Frame::default().fill(egui::Color32::from_rgb(20, 20, 20)).inner_margin(4.0))
        .show(&ctx, |ui| {
            ui.horizontal(|ui| {
                // --- МЕНЮ (Слева) ---
                ui.menu_button("File", |ui| {
                    if ui.button("Exit").clicked() { app_exit.send(AppExit); }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.button("Preferences").clicked() { /* TODO */ }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Reset Layout").clicked() { /* TODO */ }
                });

                // --- КНОПКИ ОС И DRAG AREA (Справа налево) ---
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let btn_size = egui::vec2(32.0, 24.0);

                    // Кнопка Закрыть
                    let close_btn = egui::Button::new(egui::RichText::new(" ✕ ").size(14.0)).fill(egui::Color32::TRANSPARENT);
                    if ui.add_sized(btn_size, close_btn).clicked() {
                        app_exit.send(AppExit);
                    }

                    // Кнопка Развернуть
                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new(" 🗖 ").size(14.0)).fill(egui::Color32::TRANSPARENT)).clicked() {
                        window.mode = if window.mode == WindowMode::Windowed {
                            WindowMode::BorderlessFullscreen
                        } else {
                            WindowMode::Windowed
                        };
                    }

                    // Кнопка Свернуть
                    if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new(" 🗕 ").size(14.0)).fill(egui::Color32::TRANSPARENT)).clicked() {
                        window.set_minimized(true);
                    }

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("Axicor Lab Alpha  ").color(egui::Color32::DARK_GRAY));

                    // DOD FIX: Мертвая зона между меню и кнопками — это наш Drag Area.
                    // Никакого перекрытия хитбоксов. Клик по кнопкам больше не проглатывается!
                    let drag_rect = ui.available_rect_before_wrap();
                    let drag_response = ui.interact(drag_rect, ui.id().with("title_bar_drag"), egui::Sense::drag());
                    
                    if drag_response.drag_started() {
                        drag_request.should_drag = true;
                    }
                });
            });
        });


    // 2. Workspace Area
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(egui::Color32::from_rgb(25, 25, 25)))
        .show(&ctx, |ui| {
            tree_res.tree.ui(&mut behavior, ui);
        });

    // Write back to ECS
    for (entity, input) in behavior.input_updates {
        if let Ok(mut e_input) = input_query.get_mut(entity) {
            *e_input = input;
        }
    }
    for (entity, geom) in behavior.geometry_updates {
        if let Ok(mut e_geom) = geometry_query.get_mut(entity) {
            if (e_geom.size - geom.size).length_squared() > 1.0 {
                *e_geom = geom;
            }
        }
    }

    // Process Intents
    for ev in behavior.zone_events {
        zone_events.send(ev);
    }
}

pub fn evaluate_drag_intents_system(
    mut contexts: EguiContexts,
    mut drag_state: ResMut<WindowDragState>,
    topology: Res<TopologyCache>,
    mut commands_queue: ResMut<TreeCommands>,
    tree_res: Option<Res<WorkspaceTree>>,
    windows_query: Query<&PluginWindow>,
) {
    if !drag_state.is_dragging { return; }
    let tree_res = match tree_res { Some(res) => res, None => return };

    let ctx = contexts.ctx_mut();
    let pointer_pos = ctx.pointer_interact_pos();
    let primary_released = ctx.input(|i| i.pointer.any_released());
    let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

    if escape_pressed {
        *drag_state = WindowDragState::default();
        return;
    }

    if let (Some(src_tile), Some(start_pos), Some(current_pos), Some(drag_axis), Some(drag_normal)) = (
        drag_state.source_tile, 
        drag_state.start_pos, 
        pointer_pos,
        drag_state.drag_axis,
        drag_state.drag_normal
    ) {
        if let Some(&src_rect) = topology.rects.get(&src_tile) {
            let delta = current_pos - start_pos;
            let component = if drag_axis == LinearDir::Horizontal { delta.x } else { delta.y };

            drag_state.intent = DragIntent::None;

            if component.abs() > 20.0 {
                let is_split = src_rect.contains(current_pos);
                if is_split {
                    if (if drag_axis == LinearDir::Horizontal { src_rect.width() } else { src_rect.height() }) > 200.0 {
                        let mut fraction = if drag_axis == LinearDir::Horizontal {
                            (current_pos.x - src_rect.min.x) / src_rect.width()
                        } else {
                            (current_pos.y - src_rect.min.y) / src_rect.height()
                        };
                        let min_f = 100.0 / if drag_axis == LinearDir::Horizontal { src_rect.width() } else { src_rect.height() };
                        fraction = fraction.clamp(min_f, 1.0 - min_f);

                        let insert_before = drag_normal < 0.0;
                        
                        let domain = if let Some(Tile::Pane(e)) = tree_res.tree.tiles.get(src_tile) {
                            if let Ok(plugin) = windows_query.get(*e) {
                                plugin.domain
                            } else {
                                PluginDomain::Viewport3D
                            }
                        } else {
                            PluginDomain::Viewport3D
                        };

                        drag_state.intent = DragIntent::Split { axis: drag_axis, fraction, insert_before, domain };
                        
                        let split_pos = if drag_axis == LinearDir::Horizontal {
                            src_rect.min.x + (src_rect.width() * fraction)
                        } else {
                            src_rect.min.y + (src_rect.height() * fraction)
                        };
                        
                        let painter = ctx.debug_painter();
                        if drag_axis == LinearDir::Horizontal {
                            painter.vline(split_pos, src_rect.y_range(), egui::Stroke::new(2.0, egui::Color32::WHITE));
                        } else {
                            painter.hline(src_rect.x_range(), split_pos, egui::Stroke::new(2.0, egui::Color32::WHITE));
                        }
                    }
                } else {
                    // Merge Check
                    if let Some((&victim_id, victim_rect)) = topology.rects.iter().find(|(id, r)| **id != src_tile && r.expand(8.0).contains(current_pos)) {
                        let mut valid = false;
                        let eps = 2.0;
                        if drag_axis == LinearDir::Horizontal {
                            if (src_rect.min.y - victim_rect.min.y).abs() < eps && (src_rect.max.y - victim_rect.max.y).abs() < eps { valid = true; }
                        } else {
                            if (src_rect.min.x - victim_rect.min.x).abs() < eps && (src_rect.max.x - victim_rect.max.x).abs() < eps { valid = true; }
                        }
                        if valid {
                            drag_state.intent = DragIntent::Merge { victim: victim_id };
                            ctx.debug_painter().rect_filled(*victim_rect, 0.0, egui::Color32::from_black_alpha(150));
                        }
                    }
                }
            }
        }
    }

    if primary_released {
        match drag_state.intent {
            DragIntent::Split { axis, fraction, insert_before, domain } => {
                if let Some(src) = drag_state.source_tile {
                    commands_queue.queue.push(TreeCommand::Split { target: src, axis, fraction, insert_before, domain });
                }
            }
            DragIntent::Merge { victim } => {
                if let Some(src) = drag_state.source_tile {
                    commands_queue.queue.push(TreeCommand::Merge { survivor: src, victim });
                }
            }
            _ => {}
        }
        *drag_state = WindowDragState::default();
    }
}

pub fn execute_window_commands_system(
    mut commands: Commands,
    mut tree_res: ResMut<WorkspaceTree>,
    mut commands_queue: ResMut<TreeCommands>,
    topology: Res<TopologyCache>,
    mut images: ResMut<Assets<Image>>,
) {
    for cmd in commands_queue.queue.drain(..) {
        match cmd {
            TreeCommand::Split { target, axis, fraction, insert_before, domain } => {
                let parent_rect = topology.rects.get(&target).copied().unwrap_or(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)));
                let (new_w, new_h) = if axis == LinearDir::Horizontal {
                    (parent_rect.width() * fraction, parent_rect.height())
                } else {
                    (parent_rect.width(), parent_rect.height() * fraction)
                };

                let new_entity = match domain {
                    PluginDomain::Viewport3D => {
                        let tex = create_plugin_render_target(&mut images, new_w.max(1.0) as u32, new_h.max(1.0) as u32);
                        let id = commands.spawn((
                            Camera3dBundle {
                                camera: Camera {
                                    target: bevy::render::camera::RenderTarget::Image(tex.clone()),
                                    clear_color: ClearColorConfig::Custom(Color::rgb(0.1, 0.1, 0.1)),
                                    ..default()
                                },
                                transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                                ..default()
                            },
                            ViewportCamera::default(),
                            PluginWindow { domain, texture: Some(tex) },
                            PluginInput::default(),
                            PluginGeometry { size: Vec2::new(new_w, new_h) },
                        )).id();
                        id
                    }
                    PluginDomain::ProjectExplorer => {
                        commands.spawn((
                            PluginWindow { domain, texture: None },
                            PluginInput::default(),
                            PluginGeometry { size: Vec2::new(new_w, new_h) },
                        )).id()
                    }
                    _ => {
                        commands.spawn((
                            PluginWindow { domain, texture: None },
                            PluginInput::default(),
                            PluginGeometry { size: Vec2::new(new_w, new_h) },
                        )).id()
                    }
                };

                if let Some(&Tile::Pane(old_entity)) = tree_res.tree.tiles.get(target) {
                    let old_id = tree_res.tree.tiles.insert_pane(old_entity);
                    let new_id = tree_res.tree.tiles.insert_pane(new_entity);
                    let (children, old_share, new_share) = if insert_before {
                        (vec![new_id, old_id], 1.0 - fraction, fraction)
                    } else {
                        (vec![old_id, new_id], fraction, 1.0 - fraction)
                    };
                    let mut linear = Linear { dir: axis, children, ..default() };
                    linear.shares.set_share(old_id, old_share);
                    linear.shares.set_share(new_id, new_share);
                    tree_res.tree.tiles.insert(target, Tile::Container(Container::Linear(linear)));
                }
            }
            TreeCommand::Merge { survivor, victim } => {
                if let Some(Tile::Pane(victim_entity)) = tree_res.tree.tiles.get(victim) {
                    commands.entity(*victim_entity).despawn_recursive();
                }
                let mut parent_linear = None;
                for (id, tile) in tree_res.tree.tiles.iter() {
                    if let Tile::Container(Container::Linear(linear)) = tile {
                        if linear.children.contains(&victim) && linear.children.contains(&survivor) {
                            parent_linear = Some(*id);
                            break;
                        }
                    }
                }
                if let Some(parent_id) = parent_linear {
                    let v_rect = topology.rects.get(&victim).copied().unwrap_or(egui::Rect::NOTHING);
                    let s_rect = topology.rects.get(&survivor).copied().unwrap_or(egui::Rect::NOTHING);
                    if let Some(Tile::Container(Container::Linear(linear))) = tree_res.tree.tiles.get_mut(parent_id) {
                        let is_horiz = linear.dir == LinearDir::Horizontal;
                        let v_size = if is_horiz { v_rect.width() } else { v_rect.height() };
                        let s_size = if is_horiz { s_rect.width() } else { s_rect.height() };
                        for child in linear.children.clone() {
                            let child_rect = topology.rects.get(&child).copied().unwrap_or(egui::Rect::NOTHING);
                            let mut new_weight = if is_horiz { child_rect.width() } else { child_rect.height() };
                            if child == survivor { new_weight = s_size + v_size; }
                            linear.shares.set_share(child, new_weight);
                        }
                    }
                }
                tree_res.tree.tiles.remove(victim);
                tree_res.tree.simplify(&SimplificationOptions { all_panes_must_have_tabs: false, ..default() });
            }
        }
    }
}

pub fn window_garbage_collector_system(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut tree_res: ResMut<WorkspaceTree>,
    topology: Res<TopologyCache>,
) {
    let primary_released = contexts.ctx_mut().input(|i| i.pointer.any_released());
    if !primary_released { return; }

    let pane_count = tree_res.tree.tiles.iter().filter(|(_, t)| matches!(t, Tile::Pane(_))).count();
    let mut tiles_to_remove = Vec::new();
    for (&tile_id, rect) in &topology.rects {
        if rect.width() < 100.0 || rect.height() < 100.0 { tiles_to_remove.push(tile_id); }
    }

    if !tiles_to_remove.is_empty() && pane_count > tiles_to_remove.len() {
        for tile_id in tiles_to_remove {
            if let Some(Tile::Pane(entity)) = tree_res.tree.tiles.get(tile_id) {
                commands.entity(*entity).despawn_recursive();
            }
            tree_res.tree.tiles.remove(tile_id);
        }
        tree_res.tree.simplify(&SimplificationOptions { all_panes_must_have_tabs: false, ..default() });
    }
}

pub fn window_drag_execution_system(
    mut drag_request: ResMut<layout_api::WindowDragRequest>,
    window_query: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    if drag_request.should_drag {
        drag_request.should_drag = false; // Сбрасываем триггер
        if let Ok(entity) = window_query.get_single() {
            if let Some(winit_window) = winit_windows.get_window(entity) {
                // Прямой системный вызов к оконному менеджеру ОС (X11/Wayland/Win32)
                let _ = winit_window.drag_window();
            }
        }
    }
}
