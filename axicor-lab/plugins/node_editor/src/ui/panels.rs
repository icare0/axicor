use bevy_egui::egui::{self, Color32, Pos2, Rect, Vec2};
use crate::domain::{NodeGraphUiState, ProjectSession};

// [ARCH] Шторка теперь вдвое уже, квадратная по умолчанию, растёт с контентом
const PANEL_WIDTH: f32 = 120.0;
const TAB_WIDTH: f32 = 26.0;
const TAB_HEIGHT: f32 = 56.0;
const INNER_ROUNDING: f32 = 10.0;

// Цвета панелей
const COLOR_PANEL_BG: Color32 = Color32::from_rgb(20, 21, 24);
const COLOR_PANEL_BORDER: Color32 = Color32::from_rgb(50, 53, 60);
const COLOR_LABEL: Color32 = Color32::from_rgb(100, 110, 125);

// Цвета бирок
const COLOR_TAB_INPUT: Color32 = Color32::from_rgb(32, 62, 42);
const COLOR_TAB_INPUT_HOV: Color32 = Color32::from_rgb(44, 82, 56);
const COLOR_TAB_INPUT_BORDER: Color32 = Color32::from_rgb(45, 150, 75);

const COLOR_TAB_OUTPUT: Color32 = Color32::from_rgb(62, 28, 28);
const COLOR_TAB_OUTPUT_HOV: Color32 = Color32::from_rgb(82, 40, 40);
const COLOR_TAB_OUTPUT_BORDER: Color32 = Color32::from_rgb(170, 55, 55);

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
        if let Some((src_zone, src_port, _)) = state.dragging_pin.clone() {
            if ui.rect_contains_pointer(rect) {
                if let Some(mouse_pos) = ui.input(|i| i.pointer.interact_pos()) {
                    let local_pos = mouse_pos - rect.min; // Для 3D Raycasting
                    
                    if ui.input(|i| i.pointer.any_released()) {
                        state.pending_3d_drop = Some((src_zone, src_port, mouse_pos, local_pos.to_pos2()));
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
    
    // Читаем реальную высоту панели с предыдущего кадра для идеального центрирования
    let left_height = ctx.memory(|mem| mem.area_rect(left_area_id).map_or(PANEL_WIDTH, |r| r.height()));
    let left_panel_y = window_rect.center().y - left_height / 2.0;

    // slide_left уже вычислен выше для математики центрального виджета

    // Математика: в закрытом состоянии панель уходит за левый край ровно так, чтобы торчала только бирка
    let left_closed_x = window_rect.min.x + (TAB_WIDTH / 2.0) - PANEL_WIDTH;
    let left_open_x = window_rect.min.x;
    let left_panel_x = left_closed_x * (1.0 - slide_left) + left_open_x * slide_left;

    egui::Area::new(left_area_id)
        .fixed_pos(Pos2::new(left_panel_x, left_panel_y))
        .order(egui::Order::Foreground)
        .show(&ctx, |ui| {
            ui.set_clip_rect(window_rect);

            let frame_resp = egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .stroke(egui::Stroke::new(1.0, COLOR_PANEL_BORDER))
                .rounding(egui::Rounding { nw: 0.0, sw: 0.0, ne: INNER_ROUNDING, se: INNER_ROUNDING })
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    // [DOD FIX] Жестко ограничиваем ширину, чтобы капсулы не растягивались
                    ui.set_width(PANEL_WIDTH);
                    ui.set_min_height(PANEL_WIDTH);

                    if slide_left > 0.01 {
                        ui.label(egui::RichText::new("INPUT MATRICES").size(9.0).color(COLOR_LABEL));
                        ui.add_space(6.0);

                        if let Some(inputs) = session.node_inputs.get(shard_name) {
                            for port in inputs {
                                draw_matrix_capsule(ui, port, true, state.active_3d_hover);
                                ui.add_space(3.0);
                            }
                        }
                    }
                });

            let panel_rect = frame_resp.response.rect;

            // Бирка INPUTS
            let tab_center = Pos2::new(
                panel_rect.max.x,
                panel_rect.min.y + panel_rect.height() * 0.5,
            );
            let tab_rect = Rect::from_center_size(tab_center, Vec2::new(TAB_WIDTH, TAB_HEIGHT));

            let tab_resp = ui.interact(tab_rect, ui.id().with("tab_inputs"), egui::Sense::click());
            if tab_resp.clicked() {
                state.show_inputs_panel = !state.show_inputs_panel;
            }

            let tab_fill = if tab_resp.hovered() { COLOR_TAB_INPUT_HOV } else { COLOR_TAB_INPUT };
            let tab_rounding = egui::Rounding::same(TAB_WIDTH / 2.0);

            ui.painter().rect_filled(tab_rect, tab_rounding, tab_fill);
            ui.painter().rect_stroke(tab_rect, tab_rounding, egui::Stroke::new(1.0, COLOR_TAB_INPUT_BORDER));
            draw_rotated_label(ui, tab_rect.center(), "INPUTS", 10.5, Color32::WHITE);
        });

    // =========================================================
    // 2. ПРАВАЯ ШТОРКА (OUTPUTS)
    // =========================================================
    let right_area_id = egui::Id::new("outputs_panel_area").with(win_hash);
    
    let right_height = ctx.memory(|mem| mem.area_rect(right_area_id).map_or(PANEL_WIDTH, |r| r.height()));
    let right_panel_y = window_rect.center().y - right_height / 2.0;

    // slide_right уже вычислен выше

    // Математика: в закрытом состоянии панель уходит вправо так, чтобы левая грань + центр бирки ровно касались края
    let right_closed_x = window_rect.max.x - (TAB_WIDTH / 2.0);
    let right_open_x = window_rect.max.x - PANEL_WIDTH;
    let right_panel_x = right_closed_x * (1.0 - slide_right) + right_open_x * slide_right;

    egui::Area::new(right_area_id)
        .fixed_pos(Pos2::new(right_panel_x, right_panel_y))
        .order(egui::Order::Foreground)
        .show(&ctx, |ui| {
            ui.set_clip_rect(window_rect);

            let frame_resp = egui::Frame::none()
                .fill(COLOR_PANEL_BG)
                .stroke(egui::Stroke::new(1.0, COLOR_PANEL_BORDER))
                .rounding(egui::Rounding { ne: 0.0, se: 0.0, nw: INNER_ROUNDING, sw: INNER_ROUNDING })
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    // [DOD FIX] Жестко ограничиваем ширину
                    ui.set_width(PANEL_WIDTH);
                    ui.set_min_height(PANEL_WIDTH);

                    if slide_right > 0.01 {
                        ui.label(egui::RichText::new("OUTPUT MATRICES").size(9.0).color(COLOR_LABEL));
                        ui.add_space(6.0);

                        if let Some(outputs) = session.node_outputs.get(shard_name) {
                            for port in outputs {
                                draw_matrix_capsule(ui, port, false, state.active_3d_hover);
                                ui.add_space(3.0);
                            }
                        }
                    }
                });

            let panel_rect = frame_resp.response.rect;

            // Бирка OUTPUTS
            let tab_center = Pos2::new(
                panel_rect.min.x,
                panel_rect.min.y + panel_rect.height() * 0.5,
            );
            let tab_rect = Rect::from_center_size(tab_center, Vec2::new(TAB_WIDTH, TAB_HEIGHT));

            let tab_resp = ui.interact(tab_rect, ui.id().with("tab_outputs"), egui::Sense::click());
            if tab_resp.clicked() {
                state.show_outputs_panel = !state.show_outputs_panel;
            }

            let tab_fill = if tab_resp.hovered() { COLOR_TAB_OUTPUT_HOV } else { COLOR_TAB_OUTPUT };
            let tab_rounding = egui::Rounding::same(TAB_WIDTH / 2.0);

            ui.painter().rect_filled(tab_rect, tab_rounding, tab_fill);
            ui.painter().rect_stroke(tab_rect, tab_rounding, egui::Stroke::new(1.0, COLOR_TAB_OUTPUT_BORDER));
            draw_rotated_label(ui, tab_rect.center(), "OUTPUTS", 10.5, Color32::WHITE);
        });
}

// ─────────────────────────────────────────────────────────────
// Хелпер: горизонтальный текст, повёрнутый на -90° (снизу вверх)
// ─────────────────────────────────────────────────────────────
fn draw_rotated_label(ui: &mut egui::Ui, center: Pos2, text: &str, size: f32, color: Color32) {
    let galley = ui.ctx().fonts(|f| {
        f.layout_no_wrap(
            text.to_string(),
            egui::FontId::proportional(size),
            color,
        )
    });

    let w = galley.size().x;
    let h = galley.size().y;

    let angle = -std::f32::consts::FRAC_PI_2;
    let pos = Pos2::new(center.x - h / 2.0, center.y + w / 2.0);

    ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
        pos,
        galley,
        underline: egui::Stroke::NONE,
        override_text_color: Some(color),
        fallback_color: color,
        angle,
        opacity_factor: 1.0,
    }));
}

// ─────────────────────────────────────────────────────────────
// Капсула порта матрицы (вход / выход)
// ─────────────────────────────────────────────────────────────
fn draw_matrix_capsule(ui: &mut egui::Ui, name: &str, is_input: bool, active_hover: Option<(Pos2, u32)>) {
    let height = 24.0;
    // Берем ширину, которую нам жестко задал родительский контейнер
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(width, height),
        egui::Sense::click_and_drag(),
    );

    // [DOD FIX] Унифицированный дизайн: макро-нода + оранжевый акцент
    let base = Color32::from_rgb(30, 30, 35);
    let hov = Color32::from_rgb(45, 45, 50);
    let active = Color32::from_rgb(60, 60, 65);
    let border = Color32::from_rgb(255, 150, 50); // CLR_PIN_OUT (Оранжевый)

    let bg = if resp.dragged() { active } else if resp.hovered() { hov } else { base };

    ui.painter().rect_filled(rect, 4.0, bg);
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, border));

    // Отрисовка физического "порта" (кружка) внутри капсулы
    let pin_radius = 3.5;
    let pin_center = if is_input { 
        rect.right_center() - Vec2::new(8.0, 0.0) 
    } else { 
        rect.left_center() + Vec2::new(8.0, 0.0) 
    };
    ui.painter().circle_filled(pin_center, pin_radius, border);

    // Текст смещается так, чтобы не наезжать на пин
    let text_pos = if is_input { 
        rect.left_center() + Vec2::new(8.0, 0.0) 
    } else { 
        rect.left_center() + Vec2::new(16.0, 0.0) 
    };

    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(12.0),
        Color32::from_rgb(210, 215, 220),
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
                stroke: egui::Stroke::new(2.0, border),
            });
            
            // Рисуем таргетный прицел
            if let Some(txt) = snap_text {
                painter.circle_filled(target_pos, 4.0, border);
                painter.circle_stroke(target_pos, 8.0, egui::Stroke::new(1.0, border));
                
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
