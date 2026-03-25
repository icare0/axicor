use bevy::prelude::*;
use bevy_egui::egui;
use egui_tiles::{Behavior, TileId, UiResponse, LinearDir};
use std::collections::HashMap;
use crate::layout::data::{PluginInput, PluginGeometry, WindowDragState, PaneData, PluginDomain, ProjectStatus, ProjectFsCache, ZoneSelectedEvent, LoadGraphEvent};

pub struct PaneBehavior<'a> {
    pub panes: &'a HashMap<Entity, PaneData>,
    pub rects: &'a mut HashMap<TileId, egui::Rect>,
    pub input_updates: Vec<(Entity, PluginInput)>,
    pub geometry_updates: Vec<(Entity, PluginGeometry)>,
    pub zone_events: Vec<ZoneSelectedEvent>,
    pub load_graph_events: Vec<LoadGraphEvent>,
    pub pane_swaps: Vec<(TileId, TileId)>,
    pub domain_switches: Vec<(TileId, PluginDomain)>,
    pub drag_state: &'a mut WindowDragState,
    pub fs_cache: &'a ProjectFsCache,
    pub brain_graph: &'a node_editor::BrainTopologyGraph,
    pub node_ui: &'a mut node_editor::NodeGraphUiState,
}

impl<'a> Behavior<Entity> for PaneBehavior<'a> {
    fn tab_bar_height(&self, _style: &egui::Style) -> f32 { 0.0 }

    fn pane_ui(&mut self, ui: &mut egui::Ui, tile_id: TileId, entity: &mut Entity) -> UiResponse {
        let rect = ui.max_rect().shrink(5.0);
        self.rects.insert(tile_id, rect);

        let mut input = PluginInput::default();

        if let Some(plugin) = self.panes.get(entity) {
            // 1. Единый фон и скругления для всех окон
            egui::Frame::none().fill(egui::Color32::from_rgb(25, 25, 28)).rounding(10.0).show(ui, |ui| {
                ui.set_min_size(rect.size());

                // 2. Стандартизированный Хедер Плагина (Высота 28px)
                let header_height = 28.0;
                let header_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), header_height));

                ui.allocate_ui_at_rect(header_rect, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(4.0);

                        let (icon, title) = match plugin.domain {
                            PluginDomain::Viewport3D => ("👁", "Connectome Viewer"),
                            PluginDomain::NodeEditor => ("🕸", "Shard Assembler"),
                            PluginDomain::ProjectExplorer => ("📁", "Project Explorer"),
                        };

                        // DOD FIX 1: Blender-like переключатель плагинов
                        ui.menu_button(egui::RichText::new(icon).color(egui::Color32::WHITE).size(14.0), |ui| {
                            if ui.button("👁 Connectome Viewer").clicked() {
                                self.domain_switches.push((tile_id, PluginDomain::Viewport3D));
                                ui.close_menu();
                            }
                            if ui.button("🕸 Shard Assembler").clicked() {
                                self.domain_switches.push((tile_id, PluginDomain::NodeEditor));
                                ui.close_menu();
                            }
                            if ui.button("📁 Project Explorer").clicked() {
                                self.domain_switches.push((tile_id, PluginDomain::ProjectExplorer));
                                ui.close_menu();
                            }
                        });

                        ui.add_space(4.0);

                        // DOD FIX 2: Draggable Title для свопа окон
                        let title_resp = ui.add(egui::Label::new(
                            egui::RichText::new(title).color(egui::Color32::LIGHT_GRAY).strong().size(14.0)
                        ).sense(egui::Sense::drag()));

                        // Память намерений через egui memory (Zero-Cost state)
                        if title_resp.drag_started() {
                            ui.memory_mut(|mem| mem.data.insert_temp(egui::Id::new("pane_swap_src"), tile_id));
                        }

                        if title_resp.dragged() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                        }

                        // Перехват Drop: отпустили мышь над ЭТИМ тайлом, а тащили ДРУГОЙ
                        if title_resp.hovered() && ui.input(|i| i.pointer.any_released()) {
                            if let Some(src_tile) = ui.memory_mut(|mem| mem.data.get_temp::<TileId>(egui::Id::new("pane_swap_src"))) {
                                if src_tile != tile_id {
                                    self.pane_swaps.push((src_tile, tile_id));
                                }
                                ui.memory_mut(|mem| mem.data.remove::<TileId>(egui::Id::new("pane_swap_src")));
                            }
                        }
                    });
                });

                // 3. Линия разделителя (1px)
                let separator_y = header_rect.bottom();
                ui.painter().line_segment(
                    [egui::pos2(rect.left(), separator_y), egui::pos2(rect.right(), separator_y)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 50))
                );

                // 4. PAYLOAD AREA: Вычисляем остаток места для самого плагина
                let payload_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), separator_y + 1.0),
                    rect.right_bottom()
                );

                // 5. Маршрутизация в домены (рендерим контент строго внутри payload_rect)
                ui.allocate_ui_at_rect(payload_rect, |ui| {
                    match &plugin.domain {
                        PluginDomain::Viewport3D => {
                            if let Some(texture_id) = plugin.texture_id {
                                let resp = ui.put(
                                    payload_rect,
                                    egui::Image::new(egui::load::SizedTexture::new(texture_id, payload_rect.size()))
                                );

                                // Изолированный сбор инпутов для 3D
                                ui.ctx().input(|i| {
                                    if resp.hovered() { input.scroll_delta = i.raw_scroll_delta.y; }
                                    if let Some(pos) = i.pointer.interact_pos() {
                                        if payload_rect.contains(pos) {
                                            let rmb_down = i.pointer.button_down(egui::PointerButton::Secondary);
                                            let mmb_down = i.pointer.button_down(egui::PointerButton::Middle);
                                            let shift_down = i.modifiers.shift;
                                            if rmb_down || mmb_down {
                                                let d = i.pointer.delta();
                                                if mmb_down || (rmb_down && shift_down) {
                                                    input.is_middle_pressed = true;
                                                    input.cursor_delta = Vec2::new(d.x, d.y);
                                                } else if rmb_down {
                                                    input.is_secondary_pressed = true;
                                                    input.cursor_delta = Vec2::new(d.x, d.y);
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }
                        PluginDomain::ProjectExplorer => {
                            egui::Frame::none().inner_margin(8.0).show(ui, |ui| {
                                if !self.fs_cache.projects.is_empty() {
                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        for proj in &self.fs_cache.projects {
                                            let status_icon = if proj.status == ProjectStatus::Ready { "🟢 Ready" } else { "🔴 Needs Bake" };
                                            
                                            ui.collapsing(egui::RichText::new(format!("🧠 {} [{}]", proj.name, status_icon)).color(egui::Color32::WHITE), |ui| {
                                                ui.collapsing(egui::RichText::new("🧬 DNA (Config)").color(egui::Color32::from_rgb(100, 200, 255)), |ui| {
                                                    for dna in &proj.dna_files { 
                                                        if dna == "brain.toml" {
                                                            if ui.selectable_label(false, format!("📄 {}", dna)).clicked() {
                                                                self.load_graph_events.push(LoadGraphEvent { project_name: proj.name.clone() });
                                                            }
                                                        } else {
                                                            ui.label(format!("📄 {}", dna)); 
                                                        }
                                                    }
                                                });

                                                ui.collapsing(egui::RichText::new("⚙️ Shards (Compiled)").color(egui::Color32::from_rgb(255, 150, 150)), |ui| {
                                                    if proj.shards.is_empty() {
                                                        ui.label(egui::RichText::new("Missing or Stale").color(egui::Color32::DARK_GRAY));
                                                    } else {
                                                        for shard in &proj.shards {
                                                            if ui.selectable_label(false, format!("🖧 {}", shard)).clicked() {
                                                                self.zone_events.push(ZoneSelectedEvent {
                                                                    project_name: proj.name.clone(),
                                                                    shard_name: shard.clone(),
                                                                });
                                                            }
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
                        }
                        PluginDomain::NodeEditor => {
                            if let Some(config) = &self.brain_graph.config {
                                // 1. Выделяем весь экран под кастомный Painter
                                let (response, painter) = ui.allocate_painter(payload_rect.size(), egui::Sense::click_and_drag());
                                
                                // 2. Обработка Pan & Zoom
                                if response.dragged_by(egui::PointerButton::Middle) || response.dragged_by(egui::PointerButton::Secondary) {
                                    self.node_ui.pan += response.drag_delta();
                                }
                                if response.hovered() {
                                    let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                                    if zoom_delta != 1.0 {
                                        self.node_ui.zoom *= zoom_delta;
                                        self.node_ui.zoom = self.node_ui.zoom.clamp(0.1, 5.0);
                                    }
                                }

                                // Трансформация из мировых координат графа в экранные
                                let to_screen = |pos: egui::Pos2| -> egui::Pos2 {
                                    (pos.to_vec2() * self.node_ui.zoom + self.node_ui.pan).to_pos2() + payload_rect.min.to_vec2()
                                };

                                // 1. Фаза калькуляции
                                let mut node_rects = HashMap::new();
                                let mut pin_positions = HashMap::new();

                                let node_header_height = 30.0 * self.node_ui.zoom;
                                let row_height = 22.0 * self.node_ui.zoom;
                                let base_width = 200.0 * self.node_ui.zoom;

                                for zone in &config.zones {
                                    if let Some(&pos) = self.node_ui.node_positions.get(&zone.name) {
                                        let screen_pos = to_screen(pos);
                                        let io_cfg = self.brain_graph.io_configs.get(&zone.name);
                                        
                                        let in_count = io_cfg.map(|i| i.inputs.len()).unwrap_or(0);
                                        let out_count = io_cfg.map(|i| i.outputs.len()).unwrap_or(0);
                                        let rows = in_count.max(out_count).max(1) as f32;

                                        let height = node_header_height + rows * row_height + 10.0 * self.node_ui.zoom;
                                        let rect = egui::Rect::from_min_size(screen_pos, egui::vec2(base_width, height));
                                        node_rects.insert(zone.name.clone(), rect);

                                        if let Some(io) = io_cfg {
                                            for (i, inp) in io.inputs.iter().enumerate() {
                                                let cy = rect.top() + node_header_height + (i as f32 + 0.5) * row_height;
                                                pin_positions.insert(format!("{}_in_{}", zone.name, inp.name), egui::pos2(rect.left(), cy));
                                            }
                                            for (i, out) in io.outputs.iter().enumerate() {
                                                let cy = rect.top() + node_header_height + (i as f32 + 0.5) * row_height;
                                                pin_positions.insert(format!("{}_out_{}", zone.name, out.name), egui::pos2(rect.right(), cy));
                                            }
                                        }
                                        pin_positions.insert(format!("{}_ghost_in", zone.name), egui::pos2(rect.left(), rect.center().y));
                                        pin_positions.insert(format!("{}_ghost_out", zone.name), egui::pos2(rect.right(), rect.center().y));
                                    }
                                }

                                // 2. Фаза отрисовки связей
                                for conn in &config.connections {
                                    let io_cfg = self.brain_graph.io_configs.get(&conn.from);
                                    let from_pin = if let Some(io) = io_cfg {
                                        if let Some(out) = io.outputs.first() {
                                            format!("{}_out_{}", conn.from, out.name)
                                        } else {
                                            format!("{}_ghost_out", conn.from)
                                        }
                                    } else {
                                        format!("{}_ghost_out", conn.from)
                                    };
                                    
                                    let to_pin = format!("{}_ghost_in", conn.to);

                                    if let (Some(&p1), Some(&p2)) = (pin_positions.get(&from_pin), pin_positions.get(&to_pin)) {
                                        let control_scale = (p2.x - p1.x).abs().max(50.0) * 0.5;
                                        let cp1 = p1 + egui::vec2(control_scale, 0.0);
                                        let cp2 = p2 - egui::vec2(control_scale, 0.0);

                                        painter.add(egui::epaint::CubicBezierShape {
                                            points: [p1, cp1, cp2, p2],
                                            closed: false,
                                            fill: egui::Color32::TRANSPARENT,
                                            stroke: egui::Stroke::new(3.0 * self.node_ui.zoom, egui::Color32::from_rgb(90, 140, 255)),
                                        });
                                    }
                                }

                                // 3. Фаза отрисовки тел нод
                                for zone in &config.zones {
                                    if let (Some(rect), Some(pos)) = (node_rects.get(&zone.name), self.node_ui.node_positions.get_mut(&zone.name)) {
                                        let node_response = ui.interact(*rect, ui.id().with(&zone.name), egui::Sense::drag());
                                        if node_response.dragged_by(egui::PointerButton::Primary) {
                                            *pos += node_response.drag_delta() / self.node_ui.zoom;
                                        }

                                        painter.rect_filled(*rect, 6.0 * self.node_ui.zoom, egui::Color32::from_rgb(30, 30, 35));
                                        let border_color = if node_response.hovered() { egui::Color32::from_rgb(100, 150, 255) } else { egui::Color32::from_rgb(60, 60, 65) };
                                        painter.rect_stroke(*rect, 6.0 * self.node_ui.zoom, egui::Stroke::new(1.0, border_color));

                                        painter.text(rect.left_top() + egui::vec2(10.0, 15.0) * self.node_ui.zoom, egui::Align2::LEFT_CENTER, &zone.name, egui::FontId::proportional(14.0 * self.node_ui.zoom), egui::Color32::WHITE);
                                        painter.line_segment([rect.left_top() + egui::vec2(0.0, node_header_height), rect.right_top() + egui::vec2(0.0, node_header_height)], egui::Stroke::new(1.0, border_color));

                                        if let Some(io_cfg) = self.brain_graph.io_configs.get(&zone.name) {
                                            for (i, inp) in io_cfg.inputs.iter().enumerate() {
                                                let cy = rect.top() + node_header_height + (i as f32 + 0.5) * row_height;
                                                let p = egui::pos2(rect.left(), cy);
                                                painter.circle_filled(p, 5.0 * self.node_ui.zoom, egui::Color32::from_rgb(80, 200, 120));
                                                painter.text(p + egui::vec2(10.0, 0.0) * self.node_ui.zoom, egui::Align2::LEFT_CENTER, &inp.name, egui::FontId::proportional(12.0 * self.node_ui.zoom), egui::Color32::LIGHT_GRAY);
                                            }
                                            for (i, out) in io_cfg.outputs.iter().enumerate() {
                                                let cy = rect.top() + node_header_height + (i as f32 + 0.5) * row_height;
                                                let p = egui::pos2(rect.right(), cy);
                                                painter.circle_filled(p, 5.0 * self.node_ui.zoom, egui::Color32::from_rgb(255, 100, 100));
                                                painter.text(p - egui::vec2(10.0, 0.0) * self.node_ui.zoom, egui::Align2::RIGHT_CENTER, &out.name, egui::FontId::proportional(12.0 * self.node_ui.zoom), egui::Color32::LIGHT_GRAY);
                                            }
                                        }
                                    }
                                }
                            } else {
                                ui.centered_and_justified(|ui| {
                                    ui.label(egui::RichText::new("No Brain Topology Loaded").color(egui::Color32::DARK_GRAY));
                                });
                            }
                        }
                    }
                });
            });

            // DOD: Death Mark превью
            let is_squashed = rect.width() < 100.0 || rect.height() < 100.0;
            if is_squashed {
                ui.painter().rect_filled(rect, 10.0, egui::Color32::from_black_alpha(150));
            }

            // Хитбоксы окон
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

            // Отправляем чистый PluginInput и геометрию в ECS
            self.input_updates.push((*entity, input));
            self.geometry_updates.push((*entity, PluginGeometry { size: Vec2::new(rect.width(), rect.height()) }));
        }
        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, _entity: &Entity) -> egui::WidgetText { "".into() }
}
