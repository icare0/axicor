use bevy_egui::egui::{self, Color32, Pos2, Rect, Vec2};
use crate::domain::{NodeGraphUiState, ProjectSession};

// [ARCH] Индустриальные flush-панели
const PANEL_WIDTH: f32 = 140.0;
const COLLAPSED_WIDTH: f32 = 4.0;

// Цвета панелей
const COLOR_PANEL_BG: Color32 = Color32::from_rgba_premultiplied(15, 16, 19, 230); // 0.90 alpha

// Цвета направлений
const COLOR_INPUT: Color32 = Color32::from_rgb(50, 140, 70);
const COLOR_OUTPUT: Color32 = Color32::from_rgb(200, 110, 40);

const COLOR_CAPSULE_BG: Color32 = Color32::from_rgb(25, 28, 32);
const COLOR_CAPSULE_HOV: Color32 = Color32::from_rgb(35, 38, 45);
const COLOR_CAPSULE_ACT: Color32 = Color32::from_rgb(45, 48, 55);

pub fn draw_shard_panels(
    ui: &mut egui::Ui,
    window_rect: Rect,
    state: &mut NodeGraphUiState,
    session: &ProjectSession,
    shard_name: &str,
    rtt_texture_id: Option<egui::TextureId>,
) {
    let ctx = ui.ctx().clone();
    let win_hash = window_rect.min.x.to_bits();

    let header_offset = 28.0;

    let slide_left = ctx.animate_bool(egui::Id::new("anim_inputs_panel").with(win_hash), state.show_inputs_panel);
    let slide_right = ctx.animate_bool(egui::Id::new("anim_outputs_panel").with(win_hash), state.show_outputs_panel);

    // =========================================================
    // 0. ЦЕНТРАЛЬНЫЙ 3D-ВИДЖЕТ (SHARD CAD INSPECTOR)
    // =========================================================
    // [DOD FIX] VRAM Freeze. 3D Вьюпорт всегда занимает 100% пространства окна.
    // Шторки рисуются в Order::Foreground и просто перекрывают края текстуры.
    let center_rect = Rect::from_min_max(
        Pos2::new(window_rect.min.x, window_rect.min.y + header_offset),
        Pos2::new(window_rect.max.x, window_rect.max.y),
    );

    // [DOD FIX] Избавляемся от плавающей Area. Рисуем прямо в UI окне поверх канваса.
    ui.allocate_ui_at_rect(center_rect, |ui| {
        ui.set_clip_rect(center_rect);
        let (rect, _resp) = ui.allocate_exact_size(center_rect.size(), egui::Sense::click_and_drag());
        
        state.cad_viewport_size = rect.size(); // [DOD FIX] Record dimensions for camera sync
        state.cad_viewport_rect = Some(rect);

        // [DOD FIX] Перехват DND-соединений над 3D-сценой
        state.dragging_over_3d = None; // Сброс каждый кадр
        if let Some((src_zone, src_port, _, is_input)) = state.dragging_pin.clone() {
            if ui.rect_contains_pointer(rect) {
                if let Some(mouse_pos) = ui.input(|i| i.pointer.interact_pos()) {
                    let local_pos = mouse_pos - rect.min; // Для 3D Raycasting
                    
                    if ui.input(|i| i.pointer.any_released()) {
                        state.pending_3d_drop = Some((src_zone, src_port, mouse_pos, local_pos.to_pos2(), is_input));
                        state.dragging_pin = None; // Блокируем глобальную обработку канваса
                        state.active_3d_hover = None;
                    } else {
                        state.dragging_over_3d = Some(local_pos.to_pos2());
                    }
                }
            } else {
                state.active_3d_hover = None;
            }
        } else {
            state.active_3d_hover = None;
        }

        // Зона 3D-рендера
        if let Some(tex_id) = rtt_texture_id {
            // [DOD FIX] Рисуем текстуру напрямую через painter в абсолютный rect.
            // Использование ui.add() сдвигало бы курсор за пределы clip_rect.
            let uv = egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            ui.painter().image(tex_id, rect, uv, Color32::WHITE);
            
            // Накидываем строгую техническую рамку поверх RTT
            ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 55)));
        } else {
            // Заглушка до инициализации RTT
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(12, 12, 14));
            ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(30, 30, 35)));
            
            // Рендерим техническую CAD-сетку в центре
            let grid_step = 40.0;
            let mut x = rect.min.x + (rect.width() % grid_step) / 2.0;
            while x < rect.max.x {
                ui.painter().line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], egui::Stroke::new(1.0, Color32::from_white_alpha(5)));
                x += grid_step;
            }
            let mut y = rect.min.y + (rect.height() % grid_step) / 2.0;
            while y < rect.max.y {
                ui.painter().line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], egui::Stroke::new(1.0, Color32::from_white_alpha(5)));
                y += grid_step;
            }
            
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "3D CAD VIEWPORT\n(Awaiting RTT Allocation)", egui::FontId::monospace(14.0), Color32::DARK_GRAY);
        }
    });

    // =========================================================
    // 1. ЛЕВАЯ ШТОРКА (INPUTS)
    // =========================================================
    let left_area_id = egui::Id::new("inputs_panel_area").with(win_hash);
    
    // Y-позиция: от хедера до самого низа (flush)
    let left_panel_y = window_rect.min.y + header_offset;
    let panel_height = window_rect.height() - header_offset;

    // Математика: closed = торчит только COLLAPSED_WIDTH
    let left_closed_x = window_rect.min.x - PANEL_WIDTH + COLLAPSED_WIDTH;
    let left_open_x = window_rect.min.x;
    let left_panel_x = left_closed_x * (1.0 - slide_left) + left_open_x * slide_left;

    egui::Area::new(left_area_id)
        .fixed_pos(Pos2::new(left_panel_x, left_panel_y))
        .order(egui::Order::Foreground)
        .show(&ctx, |ui| {
            ui.set_clip_rect(window_rect);

            let frame_resp = egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .show(ui, |ui| {
                    ui.set_width(PANEL_WIDTH);
                    ui.set_height(panel_height);
                    
                    // Контентная зона с отступом справа под акцентную линию
                    egui::Frame::none().inner_margin(egui::Margin { left: 8.0, right: 12.0, top: 12.0, bottom: 8.0 }).show(ui, |ui| {
                        if slide_left > 0.01 {
                            ui.label(egui::RichText::new("IN").color(COLOR_INPUT).strong());
                            ui.add_space(8.0);

                            if let Some(inputs) = session.node_inputs.get(shard_name) {
                                for port in inputs {
                                    draw_matrix_capsule(ui, port, true, state.active_3d_hover, shard_name, state);
                                    ui.add_space(2.0); // Компактный gap
                                }
                            }
                        }
                    });
                });

            let panel_rect = frame_resp.response.rect;

            // Акцентная линия на правом краю (всегда видна, служит триггером)
            let trigger_rect = Rect::from_min_max(
                Pos2::new(panel_rect.max.x - COLLAPSED_WIDTH, panel_rect.min.y),
                panel_rect.max,
            );

            let tab_resp = ui.interact(trigger_rect, ui.id().with("tab_inputs"), egui::Sense::click());
            if tab_resp.clicked() {
                state.show_inputs_panel = !state.show_inputs_panel;
            }

            let line_color = if tab_resp.hovered() { Color32::from_rgb(80, 180, 100) } else { COLOR_INPUT };
            ui.painter().rect_filled(trigger_rect, 0.0, line_color);
            
            // Если закрыта и ховер — показываем хинт
            if !state.show_inputs_panel && tab_resp.hovered() {
                egui::show_tooltip_at_pointer(ui.ctx(), ui.id().with("tt_in"), |ui| {
                    ui.label("Expand Inputs");
                });
            }
        });

    // =========================================================
    // 2. ПРАВАЯ ШТОРКА (OUTPUTS)
    // =========================================================
    let right_area_id = egui::Id::new("outputs_panel_area").with(win_hash);
    
    let right_panel_y = window_rect.min.y + header_offset;

    // Математика: closed = торчит только COLLAPSED_WIDTH
    let right_closed_x = window_rect.max.x - COLLAPSED_WIDTH;
    let right_open_x = window_rect.max.x - PANEL_WIDTH;
    let right_panel_x = right_closed_x * (1.0 - slide_right) + right_open_x * slide_right;

    egui::Area::new(right_area_id)
        .fixed_pos(Pos2::new(right_panel_x, right_panel_y))
        .order(egui::Order::Foreground)
        .show(&ctx, |ui| {
            ui.set_clip_rect(window_rect);

            let frame_resp = egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .show(ui, |ui| {
                    ui.set_width(PANEL_WIDTH);
                    ui.set_height(panel_height);

                    // Контентная зона с отступом слева под акцентную линию
                    egui::Frame::none().inner_margin(egui::Margin { left: 12.0, right: 8.0, top: 12.0, bottom: 8.0 }).show(ui, |ui| {
                        if slide_right > 0.01 {
                            // Выравнивание текста вправо
                            ui.with_layout(egui::Layout::top_down_justified(egui::Align::RIGHT), |ui| {
                                ui.label(egui::RichText::new("OUT").color(COLOR_OUTPUT).strong());
                            });
                            ui.add_space(8.0);

                            if let Some(outputs) = session.node_outputs.get(shard_name) {
                                for port in outputs {
                                    draw_matrix_capsule(ui, port, false, state.active_3d_hover, shard_name, state);
                                    ui.add_space(2.0);
                                }
                            }
                        }
                    });
                });

            let panel_rect = frame_resp.response.rect;

            // Акцентная линия на левом краю
            let trigger_rect = Rect::from_min_max(
                panel_rect.min,
                Pos2::new(panel_rect.min.x + COLLAPSED_WIDTH, panel_rect.max.y),
            );

            let tab_resp = ui.interact(trigger_rect, ui.id().with("tab_outputs"), egui::Sense::click());
            if tab_resp.clicked() {
                state.show_outputs_panel = !state.show_outputs_panel;
            }

            let line_color = if tab_resp.hovered() { Color32::from_rgb(255, 170, 70) } else { COLOR_OUTPUT };
            ui.painter().rect_filled(trigger_rect, 0.0, line_color);

            if !state.show_outputs_panel && tab_resp.hovered() {
                egui::show_tooltip_at_pointer(ui.ctx(), ui.id().with("tt_out"), |ui| {
                    ui.label("Expand Outputs");
                });
            }
        });
}

// ─────────────────────────────────────────────────────────────
// Капсула порта матрицы (вход / выход)
// ─────────────────────────────────────────────────────────────
fn draw_matrix_capsule(ui: &mut egui::Ui, name: &str, is_input: bool, active_hover: Option<(Pos2, u32)>, shard_name: &str, state: &mut crate::domain::NodeGraphUiState) {
    let height = 18.0; // Компактная высота
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(width, height),
        egui::Sense::click_and_drag(),
    );

    let accent_color = if is_input { COLOR_INPUT } else { COLOR_OUTPUT };
    let bg = if resp.dragged() { COLOR_CAPSULE_ACT } else if resp.hovered() { COLOR_CAPSULE_HOV } else { COLOR_CAPSULE_BG };

    if resp.drag_started() {
        let pin_center = if is_input { rect.right_center() } else { rect.left_center() };
        state.dragging_pin = Some((shard_name.to_string(), name.to_string(), pin_center, is_input));
    }

    // Минималистичный прямоугольник
    ui.painter().rect_filled(rect, 2.0, bg);

    // Цветной индикатор порта (зеленый для IN, справа; оранжевый для OUT, слева)
    let pin_radius = 2.5;
    let pin_center = if is_input { 
        rect.right_center() - Vec2::new(6.0, 0.0) 
    } else { 
        rect.left_center() + Vec2::new(6.0, 0.0) 
    };
    ui.painter().circle_filled(pin_center, pin_radius, accent_color);

    // Текст названия порта
    let text_pos = if is_input { 
        rect.left_center() + Vec2::new(6.0, 0.0) 
    } else { 
        // Если выход — выравниваем по правому краю
        rect.right_center() - Vec2::new(6.0, 0.0) 
    };

    let alignment = if is_input { egui::Align2::LEFT_CENTER } else { egui::Align2::RIGHT_CENTER };

    ui.painter().text(
        text_pos,
        alignment,
        name,
        egui::FontId::monospace(10.0), // Моноширный = индустриальный вид
        Color32::from_rgb(170, 180, 190),
    );

    if resp.dragged() {
        if let Some(mouse_pos) = ui.ctx().pointer_hover_pos() {
            let start_pos = if is_input { rect.right_center() } else { rect.left_center() };
            
            let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("wire_drag")));

            // [DOD FIX] Проекция якоря: если луч над 3D, прилипаем к слою
            let (target_pos, snap_text) = if let Some((snap_pos, z)) = active_hover {
                (snap_pos, Some(format!("Z-Voxel: {}", z)))
            } else {
                (mouse_pos, None)
            };

            let control_scale = (target_pos.x - start_pos.x).abs().max(50.0);
            let p1 = start_pos;
            let p2 = start_pos + if is_input { Vec2::new(control_scale, 0.0) } else { Vec2::new(-control_scale, 0.0) };
            let p3 = target_pos + if is_input { Vec2::new(-control_scale, 0.0) } else { Vec2::new(control_scale, 0.0) };
            let p4 = target_pos;

            painter.add(egui::epaint::CubicBezierShape {
                points: [p1, p2, p3, p4],
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: egui::Stroke::new(2.0, accent_color),
            });
            
            // Рисуем таргетный прицел
            if let Some(txt) = snap_text {
                painter.circle_filled(target_pos, 4.0, accent_color);
                painter.circle_stroke(target_pos, 8.0, egui::Stroke::new(1.0, accent_color));
                
                let text_pos = target_pos + Vec2::new(12.0, -12.0);
                painter.text(
                    text_pos,
                    egui::Align2::LEFT_BOTTOM,
                    txt,
                    egui::FontId::proportional(14.0),
                    Color32::WHITE,
                );
            }
        }
    }
}
