use bevy_egui::egui;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation, EditorLevel};

pub fn render_editor_ui<FTopo, FSave, FBake>(
    ui: &mut egui::Ui,
    payload_rect: egui::Rect,
    brain_graph: &BrainTopologyGraph,
    node_ui: &mut NodeGraphUiState,
    mut on_topo_mutation: FTopo,
    mut on_save: FSave,
    mut on_bake: FBake,
) where
    FTopo: FnMut(TopologyMutation),
    FSave: FnMut(),
    FBake: FnMut(),
{
    // Рисуем системный хедер. Получаем rect для канваса и rect для кнопок.
    let (canvas_rect, toolbar_rect) = layout_api::draw_unified_header(ui, payload_rect, "Topology Editor");

    // 1. TOOLBAR (Save, Compile, Breadcrumbs)
    ui.allocate_ui_at_rect(toolbar_rect, |ui| {
        // DOD FIX: Локальная мутация стилей кнопок (плоский фон, светлый текст при ховере)
        ui.visuals_mut().widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(60, 60, 65);
        ui.visuals_mut().widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(200, 200, 200);

        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            // DOD FIX: Кнопки тулбара с 5px отступом
            if ui.add(egui::Button::new("💾 Save").frame(false)).clicked() { on_save(); }
            ui.add_space(5.0);
            if ui.add(egui::Button::new("⚙ Compile").frame(false)).clicked() { on_save(); on_bake(); }

            ui.add_space(10.0);
            
            // DOD FIX: Аппаратный сепаратор строго 14px высотой
            let (sep_rect, _) = ui.allocate_exact_size(egui::vec2(1.0, 14.0), egui::Sense::hover());
            ui.painter().vline(sep_rect.center().x, sep_rect.y_range(), egui::Stroke::new(1.0, layout_api::COLOR_HEADER_LINE));
            
            ui.add_space(10.0);

            // BREADCRUMBS
            let model_name = brain_graph.active_project.as_deref().unwrap_or("No .axic Model");
            ui.menu_button(egui::RichText::new(model_name).strong(), |ui| {
                if ui.button("Root (All Models)").clicked() {
                    node_ui.level = EditorLevel::Model;
                    ui.close_menu();
                }
            });

            ui.label(egui::RichText::new("›").color(egui::Color32::DARK_GRAY));

            // DOD FIX: Никаких .to_string() и .clone() в горячем цикле рендера. Только &str.
            let current_zone: &str = match &node_ui.level {
                EditorLevel::Model => "Select Zone...",
                EditorLevel::Zone(z) => z.as_str(),
                EditorLevel::Shard { zone_name, .. } => zone_name.as_str(),
            };

            let mut next_zone = None;
            ui.menu_button(current_zone, |ui| {
                if ui.button("Main Network").clicked() {
                    // Аллокация легальна, так как выполняется только в обработчике клика
                    next_zone = Some(EditorLevel::Zone("Main Network".to_string()));
                    ui.close_menu();
                }
            });
            if let Some(lvl) = next_zone {
                node_ui.level = lvl;
            }

            let mut next_shard_lvl = None;
            if let EditorLevel::Shard { shard_name, zone_name } = &node_ui.level {
                ui.label(egui::RichText::new("›").color(egui::Color32::DARK_GRAY));

                ui.menu_button(shard_name, |ui| {
                    if let Some(config) = &brain_graph.config {
                        for z in &config.zones {
                            if ui.button(format!("Shard: {}", z.name)).clicked() {
                                next_shard_lvl = Some(EditorLevel::Shard {
                                    zone_name: zone_name.clone(),
                                    shard_name: z.name.clone(),
                                });
                                ui.close_menu();
                            }
                        }
                    }
                });
            }
            if let Some(lvl) = next_shard_lvl {
                node_ui.level = lvl;
            }
        });
    });

    // 2. NODE CANVAS
    let mut next_lvl_canvas = None;
    ui.allocate_ui_at_rect(canvas_rect, |ui: &mut egui::Ui| {
        // DOD FIX: Скругляем ТОЛЬКО нижние углы канваса
        egui::Frame::none().fill(egui::Color32::from_rgb(25, 25, 28))
            .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 10.0, se: 10.0 })
            .show(ui, |ui: &mut egui::Ui| {
            ui.set_clip_rect(canvas_rect);
            
            match &node_ui.level {
                EditorLevel::Model => {
                    ui.centered_and_justified(|ui: &mut egui::Ui| {
                        ui.label(egui::RichText::new("Model Level: .axic Transformer Topography\n(Showing Zones/Logical Groups)").color(egui::Color32::LIGHT_GRAY));
                    });
                },
                EditorLevel::Zone(_zone_name) => {
                    if let Some(config) = &brain_graph.config {
                        let (response, painter) = ui.allocate_painter(canvas_rect.size(), egui::Sense::click_and_drag());
                        
                        // Состояние канваса (зум/пан)
                        let to_screen = egui::emath::RectTransform::from_to(
                            egui::Rect::from_center_size(egui::Pos2::ZERO, response.rect.size() / node_ui.zoom),
                            response.rect.translate(node_ui.pan),
                        );

                        // Сетка
                        let grid_spacing = 50.0 * node_ui.zoom;
                        for i in 0..=(canvas_rect.width() / grid_spacing) as i32 {
                            let x = canvas_rect.left() + i as f32 * grid_spacing + (node_ui.pan.x % grid_spacing);
                            painter.line_segment([egui::pos2(x, canvas_rect.top()), egui::pos2(x, canvas_rect.bottom())], egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 35, 38)));
                        }
                        for i in 0..=(canvas_rect.height() / grid_spacing) as i32 {
                            let y = canvas_rect.top() + i as f32 * grid_spacing + (node_ui.pan.y % grid_spacing);
                            painter.line_segment([egui::pos2(canvas_rect.left(), y), egui::pos2(canvas_rect.right(), y)], egui::Stroke::new(1.0, egui::Color32::from_rgb(35, 35, 38)));
                        }

                        // Отрисовка Зон
                        for zone in &config.zones {
                            let pos = node_ui.node_positions.get(&zone.name).cloned().unwrap_or(egui::pos2(100.0, 100.0));
                            let screen_pos = to_screen.transform_pos(pos);
                            let node_rect = egui::Rect::from_center_size(screen_pos, egui::vec2(200.0, 120.0) * node_ui.zoom);
                            
                            let node_res = ui.interact(node_rect, ui.id().with(&zone.name), egui::Sense::click_and_drag());
                            
                            // Фон ноды
                            painter.rect_filled(node_rect, 8.0, egui::Color32::from_rgb(45, 45, 50));
                            if node_res.hovered() {
                                painter.rect_stroke(node_rect, 8.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)));
                            }

                            // Заголовок
                            painter.text(node_rect.center_top() + egui::vec2(0.0, 20.0 * node_ui.zoom), egui::Align2::CENTER_TOP, &zone.name, egui::FontId::proportional(18.0 * node_ui.zoom), egui::Color32::WHITE);
                            
                            // Drill-down клик
                            if node_res.double_clicked() {
                                next_lvl_canvas = Some(EditorLevel::Shard {
                                    zone_name: zone.name.clone(),
                                    shard_name: zone.name.clone(),
                                });
                            }
                        }

                        // Управление камерой (Pan/Zoom)
                        if response.dragged_by(egui::PointerButton::Middle) {
                            node_ui.pan += response.drag_delta();
                        }
                        if response.hovered() {
                            let zoom_delta = ui.input(|i| i.raw_scroll_delta.y);
                            if zoom_delta != 0.0 {
                                let old_zoom = node_ui.zoom;
                                node_ui.zoom = (node_ui.zoom * (1.0 + zoom_delta * 0.001)).clamp(0.1, 5.0);
                                let zoom_ratio = node_ui.zoom / old_zoom;
                                if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                                    let delta = pointer - response.rect.center();
                                    node_ui.pan -= delta * (zoom_ratio - 1.0);
                                }
                            }
                        }

                        // Контекстное меню (ПКМ)
                        response.context_menu(|ui| {
                            if ui.button("Add Shard").clicked() {
                                let spawn_pos = to_screen.inverse().transform_pos(ui.ctx().pointer_interact_pos().unwrap_or(egui::Pos2::ZERO));
                                on_topo_mutation(TopologyMutation::AddZone { 
                                    name: format!("Shard_{}", config.zones.len()),
                                    pos: spawn_pos
                                });
                                ui.close_menu();
                            }
                        });
                    }
                },
                EditorLevel::Shard { shard_name, .. } => {
                    // Рендер внутренностей Шарда (Слои, I/O)
                    if let Some(anatomy) = brain_graph.anatomy_configs.get(shard_name) {
                        let mut cursor_y = canvas_rect.top() + 20.0;
                        for layer in &anatomy.layers {
                            let layer_h = 100.0 * node_ui.zoom;
                            let rect = egui::Rect::from_min_max(
                                egui::pos2(canvas_rect.left() + 20.0, cursor_y),
                                egui::pos2(canvas_rect.right() - 20.0, cursor_y + layer_h)
                            );
                            
                            ui.painter().rect_filled(rect, 5.0, egui::Color32::from_rgb(30, 30, 35));
                            
                            // Заголовок слоя
                            ui.painter().text(
                                rect.left_top() + egui::vec2(10.0, 10.0) * node_ui.zoom,
                                egui::Align2::LEFT_TOP,
                                format!("{}  [{}% Z-Height | {}% Density]", layer.name, (layer.height_pct * 100.0).round(), (layer.density * 100.0).round()),
                                egui::FontId::proportional(16.0 * node_ui.zoom),
                                egui::Color32::WHITE,
                            );

                            // Квоты популяций (Blueprints)
                            let mut comp_y = rect.top() + 35.0 * node_ui.zoom;
                            for (type_name, quota) in &layer.composition {
                                let type_name: &String = type_name;
                                let is_inh = type_name.contains("Inh") || type_name.contains("GABA");
                                let color = if is_inh { 
                                    egui::Color32::from_rgb(255, 100, 100) 
                                } else { 
                                    egui::Color32::from_rgb(100, 200, 255) 
                                };
                                ui.painter().text(
                                    egui::pos2(rect.left() + 20.0 * node_ui.zoom, comp_y),
                                    egui::Align2::LEFT_TOP,
                                    format!("▶ {}: {}%", type_name, (*quota * 100.0).round() as i32),
                                    egui::FontId::proportional(13.0 * node_ui.zoom),
                                    color,
                                );
                                comp_y += 18.0 * node_ui.zoom;
                            }
                            cursor_y += layer_h + 10.0;
                        }
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.label(egui::RichText::new(format!("Shard {} Anatomy data not found", shard_name)).color(egui::Color32::RED));
                        });
                    }
                }
            }
        });
    });

    if let Some(lvl) = next_lvl_canvas {
        node_ui.level = lvl;
    }
}
