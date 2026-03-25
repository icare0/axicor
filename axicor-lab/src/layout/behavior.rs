use bevy::prelude::*;
use bevy_egui::egui;
use egui_tiles::{Behavior, TileId, UiResponse, LinearDir};
use std::collections::HashMap;
use crate::layout::data::{PluginInput, PluginGeometry, WindowDragState, PaneData, PluginDomain, ProjectStatus, ProjectFsCache};

pub struct PaneBehavior<'a> {
    pub panes: &'a HashMap<Entity, PaneData>,
    pub rects: &'a mut HashMap<TileId, egui::Rect>,
    pub input_updates: Vec<(Entity, PluginInput)>,
    pub geometry_updates: Vec<(Entity, PluginGeometry)>,
    pub zone_events: Vec<String>,
    pub drag_state: &'a mut WindowDragState,
    pub fs_cache: &'a ProjectFsCache,
}

impl<'a> Behavior<Entity> for PaneBehavior<'a> {
    fn tab_bar_height(&self, _style: &egui::Style) -> f32 { 0.0 }

    fn pane_ui(&mut self, ui: &mut egui::Ui, tile_id: TileId, entity: &mut Entity) -> UiResponse {
        let rect = ui.max_rect().shrink(5.0);
        self.rects.insert(tile_id, rect);

        if let Some(plugin) = self.panes.get(entity) {
            // DOD: Hybrid Render (VRAM vs Native UI)
            match &plugin.domain {
                PluginDomain::Viewport3D => {
                    if let Some(texture_id) = plugin.texture_id {
                        ui.put(
                            rect,
                            egui::Image::new(egui::load::SizedTexture::new(texture_id, rect.size()))
                                .rounding(10.0),
                        );
                    }
                }
                PluginDomain::ProjectExplorer => {
                    ui.allocate_ui_at_rect(rect, |ui| {
                        egui::Frame::none().fill(egui::Color32::from_rgb(30, 30, 30)).rounding(10.0).inner_margin(8.0).show(ui, |ui| {
                            ui.set_min_size(rect.size());
                            ui.heading(egui::RichText::new("📁 Project Explorer").strong().color(egui::Color32::WHITE));
                            ui.separator();
                            
                            if !self.fs_cache.projects.is_empty() {
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    for proj in &self.fs_cache.projects {
                                        let status_icon = if proj.status == ProjectStatus::Ready { "🟢 Ready" } else { "🔴 Needs Bake" };
                                        
                                        ui.collapsing(egui::RichText::new(format!("🧠 {} [{}]", proj.name, status_icon)).color(egui::Color32::WHITE), |ui| {
                                            
                                            // DNA Block
                                            ui.collapsing(egui::RichText::new("🧬 DNA (Config)").color(egui::Color32::from_rgb(100, 200, 255)), |ui| {
                                                for dna in &proj.dna_files {
                                                    ui.label(format!("📄 {}", dna));
                                                }
                                            });

                                            // Shards Block
                                            ui.collapsing(egui::RichText::new("⚙️ Shards (Compiled)").color(egui::Color32::from_rgb(255, 150, 150)), |ui| {
                                                if proj.shards.is_empty() {
                                                    ui.label(egui::RichText::new("Missing or Stale").color(egui::Color32::DARK_GRAY));
                                                } else {
                                                    for shard in &proj.shards {
                                                        ui.label(format!("🖧 {}", shard));
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(4.0);
                                    }
                                });
                            } else {
                                ui.spinner();
                            }
                        });
                    });
                }
                PluginDomain::NodeEditor => {
                    ui.allocate_ui_at_rect(rect, |ui| {
                        egui::Frame::none().fill(egui::Color32::from_rgb(40, 40, 45)).rounding(10.0).show(ui, |ui| {
                            ui.set_min_size(rect.size());
                            ui.heading(" 🕸 Node Graph");
                            ui.separator();
                            ui.label("Waiting for graph backend...");
                        });
                    });
                }
            }

            // Death Mark
            let is_squashed = rect.width() < 100.0 || rect.height() < 100.0;
            if is_squashed {
                ui.painter().rect_filled(rect, 10.0, egui::Color32::from_black_alpha(150));
            }

            // Hitboxes (Initiation only)
            if !self.drag_state.is_dragging {
                let thickness = 10.0;
                let edges = [
                    (egui::Rect::from_min_max(rect.left_top(), rect.right_top() + egui::vec2(0.0, thickness)), LinearDir::Vertical, -1.0),
                    (egui::Rect::from_min_max(rect.left_bottom() - egui::vec2(0.0, thickness), rect.right_bottom()), LinearDir::Vertical, 1.0),
                    (egui::Rect::from_min_max(rect.left_top(), rect.left_bottom() + egui::vec2(thickness, 0.0)), LinearDir::Horizontal, -1.0),
                    (egui::Rect::from_min_max(rect.right_top() - egui::vec2(thickness, 0.0), rect.right_bottom()), LinearDir::Horizontal, 1.0),
                ];

                for (edge_rect, axis, normal_sign) in edges {
                    let resp = ui.interact(edge_rect, ui.id().with(axis).with(normal_sign as i32), egui::Sense::drag());
                    
                    if resp.hovered() {
                        ui.painter().rect_filled(edge_rect, 5.0, egui::Color32::from_white_alpha(13));
                    }

                    if resp.drag_started() {
                        self.drag_state.is_dragging = true;
                        self.drag_state.source_tile = Some(tile_id);
                        self.drag_state.start_pos = ui.ctx().pointer_interact_pos();
                        self.drag_state.drag_axis = Some(axis);
                        self.drag_state.drag_normal = Some(normal_sign);
                        break;
                    }
                }
            }

            // Local Inputs
            let pointer_pos = ui.ctx().pointer_hover_pos();
            let mut input = PluginInput::default();
            if let Some(pos) = pointer_pos {
                if rect.contains(pos) {
                    let local = pos - rect.min;
                    input.local_cursor = Vec2::new(local.x, local.y);
                    input.is_pressed = ui.ctx().input(|i| i.pointer.primary_down());
                }
            }
            self.input_updates.push((*entity, input));

            let new_size = Vec2::new(rect.width(), rect.height());
            self.geometry_updates.push((*entity, PluginGeometry { size: new_size }));
        }
        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, _entity: &Entity) -> egui::WidgetText { "".into() }
}
