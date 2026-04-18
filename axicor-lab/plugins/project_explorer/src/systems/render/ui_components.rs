use crate::domain::{ProjectModel, ProjectNode, ProjectNodeType};
use bevy_egui::egui;

const EXPLORER_MIN_WIDTH: f32 = 200.0;
const EXPLORER_MAX_WIDTH: f32 = 300.0;
const SECTION_SPACING: f32 = 3.0;
const SEPARATOR_FRACTION: f32 = 0.9;

pub fn draw_explorer_tree<FLoad, FZone>(
    ui: &mut egui::Ui,
    bundles: &[&ProjectModel],
    sources: &[&ProjectModel],
    open_file_ev: &mut bevy::prelude::EventWriter<layout_api::OpenFileEvent>,
    ctx_menu_events: &mut bevy::prelude::EventWriter<layout_api::OpenContextMenuEvent>,
    target_window: bevy::prelude::Entity,
    mut active_file: Option<&mut Option<std::path::PathBuf>>,
    mut on_load: FLoad,
    mut on_zone: FZone,
) where
    FLoad: FnMut(String),
    FZone: FnMut(String, String),
{
    if ui.input(|i| i.pointer.any_released()) {
        ui.memory_mut(|mem| {
            mem.data
                .remove_temp::<std::path::PathBuf>(egui::Id::new("dnd_path"))
        });
    }
    // 2. Project List
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.set_min_width(EXPLORER_MIN_WIDTH);
        ui.set_max_width(EXPLORER_MAX_WIDTH);
        ui.add_space(3.0);

        for project in bundles {
            draw_project_root(
                ui,
                project,
                open_file_ev,
                ctx_menu_events,
                target_window,
                active_file.as_deref_mut(),
                &mut on_load,
                &mut on_zone,
            );
            ui.add_space(SECTION_SPACING);
        }

        if !bundles.is_empty() && !sources.is_empty() {
            draw_separator(ui);
        }

        for project in sources {
            draw_project_root(
                ui,
                project,
                open_file_ev,
                ctx_menu_events,
                target_window,
                active_file.as_deref_mut(),
                &mut on_load,
                &mut on_zone,
            );
            ui.add_space(SECTION_SPACING);
        }
    });
}

fn draw_project_root<FLoad, FZone>(
    ui: &mut egui::Ui,
    project: &ProjectModel,
    open_file_ev: &mut bevy::prelude::EventWriter<layout_api::OpenFileEvent>,
    ctx_menu_events: &mut bevy::prelude::EventWriter<layout_api::OpenContextMenuEvent>,
    target_window: bevy::prelude::Entity,
    mut active_file: Option<&mut Option<std::path::PathBuf>>,
    on_load: &mut FLoad,
    on_zone: &mut FZone,
) where
    FLoad: FnMut(String),
    FZone: FnMut(String, String),
{
    let is_project_active = active_file
        .as_ref()
        .and_then(|af| af.as_ref())
        .map_or(false, |p| {
            p.to_string_lossy()
                .contains(&project.name.replace(" (Source)", ""))
        });

    let label = if project.is_bundle {
        format!("[Model] {}", project.name)
    } else {
        project.name.clone()
    };

    let header_text = if is_project_active {
        egui::RichText::new(label)
            .color(egui::Color32::LIGHT_BLUE)
            .strong()
    } else {
        egui::RichText::new(label)
    };

    let header = egui::CollapsingHeader::new(header_text)
        .id_source(&project.name)
        .default_open(is_project_active)
        .show(ui, |ui| {
            // Model-wide simulation.toml
            let parent_path = find_simulation_node(&project.root_nodes)
                .map(|n| n.path.clone())
                .unwrap_or_default();
            for node in &project.root_nodes {
                draw_node_recursive(
                    ui,
                    &project.name,
                    node,
                    &parent_path,
                    open_file_ev,
                    ctx_menu_events,
                    target_window,
                    active_file.as_deref_mut(),
                    on_zone,
                );
            }
        });

    if header.header_response.clicked() {
        if let Some(sim_node) = find_simulation_node(&project.root_nodes) {
            if let Some(ref mut af) = active_file {
                **af = Some(sim_node.path.clone());
            }
            open_file_ev.send(layout_api::OpenFileEvent {
                path: sim_node.path.clone(),
            });
        }
        on_load(project.name.clone());
    }

    if header.header_response.secondary_clicked() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            ctx_menu_events.send(layout_api::OpenContextMenuEvent {
                target_window,
                position: pos,
                actions: vec![layout_api::MenuAction {
                    action_id: format!("explorer.delete_model|{}", project.name),
                    label: " Delete Model".into(),
                }],
            });
        }
    }
}

fn draw_node_recursive<FZone>(
    ui: &mut egui::Ui,
    project_name: &str,
    node: &ProjectNode,
    parent_path: &std::path::PathBuf,
    open_file_ev: &mut bevy::prelude::EventWriter<layout_api::OpenFileEvent>,
    ctx_menu_events: &mut bevy::prelude::EventWriter<layout_api::OpenContextMenuEvent>,
    target_window: bevy::prelude::Entity,
    mut active_file: Option<&mut Option<std::path::PathBuf>>,
    on_zone: &mut FZone,
) where
    FZone: FnMut(String, String),
{
    let is_active = active_file.as_ref().and_then(|af| af.as_ref()) == Some(&node.path);

    let (label_color, is_strikethrough) = match node.git_status {
        crate::domain::GitStatus::Added => (egui::Color32::from_rgb(100, 255, 100), false),
        crate::domain::GitStatus::Deleted => (egui::Color32::from_rgb(255, 100, 100), true),
        crate::domain::GitStatus::Unmodified => {
            if is_active {
                (egui::Color32::WHITE, false)
            } else {
                (egui::Color32::GRAY, false)
            }
        }
    };

    let mut label_text = egui::RichText::new(&node.name).color(label_color);
    if is_active {
        label_text = label_text.strong();
    }
    if is_strikethrough {
        label_text = label_text.strikethrough();
    }

    // WARN: Use persistent node.id for egui!
    let id = ui.make_persistent_id(&node.id);
    let mut collapsing =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);

    let mut response = None;
    ui.horizontal(|ui| {
        if !node.children.is_empty() {
            // DOD FIX: Manual toggle button
            if collapsing
                .show_toggle_button(ui, egui::collapsing_header::paint_default_icon)
                .clicked()
            {
                // UI interaction only, show_toggle_button mutates collapsing state
            }
        } else {
            ui.add_space(14.0);
        }

        if node.git_status != crate::domain::GitStatus::Unmodified {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 14.0), egui::Sense::hover());
            ui.painter().vline(
                rect.center().x,
                rect.y_range(),
                egui::Stroke::new(2.0, label_color),
            );
        }

        let resp = ui.selectable_label(is_active, label_text);
        if resp.clicked() && node.git_status != crate::domain::GitStatus::Deleted {
            if let Some(ref mut af) = active_file {
                **af = Some(node.path.clone());
            }
            open_file_ev.send(layout_api::OpenFileEvent {
                path: node.path.clone(),
            });

            if node.node_type == ProjectNodeType::Shard {
                let zone_name = node.name.replace("Zone: ", "").replace("Shard: ", "");
                on_zone(project_name.to_string(), zone_name);
            }
        }

        if resp.dragged() && node.git_status != crate::domain::GitStatus::Deleted {
            ui.memory_mut(|mem| {
                mem.data
                    .insert_temp(egui::Id::new("dnd_path"), node.path.clone())
            });
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        response = Some(resp);
    });

    if let Some(resp) = response {
        if resp.secondary_clicked() {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let mut actions = vec![];
                match node.node_type {
                    ProjectNodeType::Brain => {
                        let name = node.name.replace(".toml", "");
                        if node.git_status != crate::domain::GitStatus::Deleted {
                            actions.push(layout_api::MenuAction {
                                action_id: format!(
                                    "explorer.delete_dept|{}|{}|{}",
                                    name,
                                    node.id,
                                    parent_path.display()
                                ),
                                label: " Delete Department".into(),
                            });
                        }
                    }
                    ProjectNodeType::Shard => {
                        let name = node.name.replace(".toml", "");
                        if node.git_status != crate::domain::GitStatus::Deleted {
                            actions.push(layout_api::MenuAction {
                                action_id: format!(
                                    "explorer.delete_shard|{}|{}|{}",
                                    name,
                                    node.id,
                                    parent_path.display()
                                ),
                                label: " Delete Shard".into(),
                            });
                        }
                    }
                    _ => {}
                }

                if !actions.is_empty() {
                    ctx_menu_events.send(layout_api::OpenContextMenuEvent {
                        target_window,
                        position: pos,
                        actions,
                    });
                }
            }
        }
    }

    // DOD FIX: Flush state, mandatory!
    collapsing.store(ui.ctx());

    if collapsing.is_open() {
        ui.indent(id, |ui| {
            for child in &node.children {
                // Recursively draw subnodes
                draw_node_recursive(
                    ui,
                    project_name,
                    child,
                    &node.path,
                    open_file_ev,
                    ctx_menu_events,
                    target_window,
                    active_file.as_deref_mut(),
                    on_zone,
                );
            }
        });
    }
}

fn find_simulation_node(nodes: &[ProjectNode]) -> Option<&ProjectNode> {
    nodes
        .iter()
        .find(|n| n.node_type == ProjectNodeType::Simulation)
        .or_else(|| {
            nodes
                .iter()
                .find(|n| !n.children.is_empty())
                .and_then(|n| find_simulation_node(&n.children))
        })
}

fn draw_separator(ui: &mut egui::Ui) {
    let total_w = ui.available_width();
    let line_w = total_w * SEPARATOR_FRACTION;
    let pad = (total_w - line_w) / 2.0;

    ui.horizontal(|ui| {
        ui.add_space(pad);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(line_w, 1.0), egui::Sense::hover());
        ui.painter().hline(
            rect.x_range(),
            rect.center().y,
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
        );
    });
    ui.add_space(2.0);
}
