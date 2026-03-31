use bevy_egui::egui::{self, Color32, Pos2, Rect, Vec2};
use crate::domain::{NodeGraphUiState, ProjectSession};

const PANEL_WIDTH: f32 = 240.0;
const TAB_WIDTH: f32 = 28.0;

pub fn draw_shard_panels(
    ctx: &egui::Context,
    window_rect: Rect,
    state: &mut NodeGraphUiState,
    _session: &ProjectSession,
    _shard_name: &str,
) {
    let header_offset = 28.0;
    let content_height = window_rect.height() - header_offset;
    // DOD FIX: Уникальный ID окна, чтобы тайлы плагинов не шарили стейт анимации
    let win_hash = window_rect.min.x.to_bits(); 

    // 1. Левая шторка (INPUTS)
    let slide_left = ctx.animate_bool(egui::Id::new("anim_inputs_panel").with(win_hash), state.show_inputs_panel);
    let left_offset = -PANEL_WIDTH + (PANEL_WIDTH * slide_left);

    egui::Area::new(egui::Id::new("inputs_panel_area").with(win_hash))
        .fixed_pos(window_rect.min + Vec2::new(left_offset, header_offset))
        .order(egui::Order::Foreground) // [DOD FIX] Подняли над слоем интерактивного канваса
        .show(ctx, |ui| {
            ui.set_clip_rect(window_rect);
            let panel_rect = Rect::from_min_size(ui.min_rect().min, Vec2::new(PANEL_WIDTH, content_height));
            
            ui.painter().rect_filled(panel_rect, 0.0, Color32::from_rgb(20, 20, 22));
            ui.painter().rect_stroke(panel_rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)));

            let tab_rect = Rect::from_min_size(Pos2::new(panel_rect.max.x, panel_rect.min.y + 40.0), Vec2::new(TAB_WIDTH, 120.0));
            let tab_resp = ui.interact(tab_rect, ui.id().with("tab_inputs"), egui::Sense::click());
            if tab_resp.clicked() { state.show_inputs_panel = !state.show_inputs_panel; }
            
            ui.painter().rect_filled(tab_rect, egui::Rounding { ne: 6.0, se: 6.0, nw: 0.0, sw: 0.0 }, Color32::from_rgb(35, 65, 45));
            ui.painter().text(tab_rect.center(), egui::Align2::CENTER_CENTER, "I\nN\nP\nU\nT\nS", egui::FontId::monospace(14.0), Color32::WHITE);

            if slide_left > 0.0 {
                ui.allocate_ui_at_rect(panel_rect.shrink(12.0), |ui| {
                    ui.label(egui::RichText::new("INPUT MATRICES").strong().color(Color32::WHITE));
                    ui.add_space(8.0);
                    
                    if let Some(inputs) = _session.node_inputs.get(_shard_name) {
                        for port in inputs {
                            draw_matrix_capsule(ui, port, true);
                            ui.add_space(4.0);
                        }
                    }
                });
            }
        });

    // 2. Правая шторка (OUTPUTS)
    let slide_right = ctx.animate_bool(egui::Id::new("anim_outputs_panel").with(win_hash), state.show_outputs_panel);
    let right_offset = window_rect.max.x - (PANEL_WIDTH * slide_right);

    egui::Area::new(egui::Id::new("outputs_panel_area").with(win_hash))
        .fixed_pos(Pos2::new(right_offset, window_rect.min.y + header_offset))
        .order(egui::Order::Foreground) // [DOD FIX] Подняли над холстом
        .show(ctx, |ui| {
            ui.set_clip_rect(window_rect);
            let panel_rect = Rect::from_min_size(ui.min_rect().min, Vec2::new(PANEL_WIDTH, content_height));
            
            ui.painter().rect_filled(panel_rect, 0.0, Color32::from_rgb(20, 20, 22));
            ui.painter().rect_stroke(panel_rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)));

            let tab_rect = Rect::from_min_size(Pos2::new(panel_rect.min.x - TAB_WIDTH, panel_rect.min.y + 40.0), Vec2::new(TAB_WIDTH, 120.0));
            let tab_resp = ui.interact(tab_rect, ui.id().with("tab_outputs"), egui::Sense::click());
            if tab_resp.clicked() { state.show_outputs_panel = !state.show_outputs_panel; }
            
            ui.painter().rect_filled(tab_rect, egui::Rounding { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 }, Color32::from_rgb(65, 35, 35));
            ui.painter().text(tab_rect.center(), egui::Align2::CENTER_CENTER, "O\nU\nT\nP\nU\nT\nS", egui::FontId::monospace(14.0), Color32::WHITE);

            // Бирка UV-MAP
            let tab_uv_rect = Rect::from_min_size(Pos2::new(panel_rect.min.x - TAB_WIDTH, panel_rect.max.y - 140.0), Vec2::new(TAB_WIDTH, 100.0));
            let tab_uv_resp = ui.interact(tab_uv_rect, ui.id().with("tab_uv"), egui::Sense::click());
            if tab_uv_resp.clicked() { state.show_uv_panel = !state.show_uv_panel; }
            
            ui.painter().rect_filled(tab_uv_rect, egui::Rounding { nw: 6.0, sw: 6.0, ne: 0.0, se: 0.0 }, Color32::from_rgb(50, 50, 70));
            ui.painter().text(tab_uv_rect.center(), egui::Align2::CENTER_CENTER, "U\nV\n-\nM\nA\nP", egui::FontId::monospace(14.0), Color32::WHITE);

            if slide_right > 0.0 {
                ui.allocate_ui_at_rect(panel_rect.shrink(12.0), |ui| {
                    ui.label(egui::RichText::new("OUTPUT MATRICES").strong().color(Color32::WHITE));
                    ui.add_space(8.0);

                    if let Some(outputs) = _session.node_outputs.get(_shard_name) {
                        for port in outputs {
                            draw_matrix_capsule(ui, port, false);
                            ui.add_space(4.0);
                        }
                    }
                });
            }
        });

    // 3. Выдвижная панель UV-MAP
    let slide_uv = ctx.animate_bool(egui::Id::new("anim_uv_panel").with(win_hash), state.show_uv_panel && state.show_outputs_panel);
    if slide_uv > 0.0 {
        let uv_panel_size = Vec2::new(300.0, 300.0);
        let uv_offset = right_offset - (uv_panel_size.x * slide_uv);
        
        egui::Area::new(egui::Id::new("uv_panel_area").with(win_hash))
            .fixed_pos(Pos2::new(uv_offset, window_rect.max.y - uv_panel_size.y - 20.0))
            .order(egui::Order::Foreground) // [DOD FIX]
            .show(ctx, |ui| {
                ui.set_clip_rect(window_rect);
                let uv_rect = Rect::from_min_size(ui.min_rect().min, uv_panel_size);
                ui.painter().rect_filled(uv_rect, 0.0, Color32::from_rgb(15, 15, 18));
                ui.painter().rect_stroke(uv_rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(80, 80, 100)));

                ui.allocate_ui_at_rect(uv_rect.shrink(10.0), |ui| {
                    ui.label(egui::RichText::new("UV MAP EDITOR").strong().color(Color32::WHITE));
                });
            });
    }
}

fn draw_matrix_capsule(ui: &mut egui::Ui, name: &str, is_input: bool) {
    let height = 28.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::click_and_drag());
    
    let base_color = if is_input { Color32::from_rgb(35, 65, 45) } else { Color32::from_rgb(65, 35, 35) };
    let border_color = if is_input { Color32::from_rgb(50, 160, 80) } else { Color32::from_rgb(180, 60, 60) };
    
    let bg_color = if resp.dragged() { 
        if is_input { Color32::from_rgb(50, 90, 60) } else { Color32::from_rgb(90, 50, 50) }
    } else if resp.hovered() { 
        if is_input { Color32::from_rgb(45, 80, 55) } else { Color32::from_rgb(80, 45, 45) }
    } else { 
        base_color 
    };

    ui.painter().rect_filled(rect, 4.0, bg_color);
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, border_color));
    
    let text_pos = rect.left_center() + Vec2::new(8.0, 0.0);
    ui.painter().text(text_pos, egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(14.0), Color32::WHITE);
}
