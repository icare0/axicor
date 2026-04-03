use bevy_egui::egui::{self, Color32, Pos2, Rect};
use crate::domain::{NodeGraphUiState, ProjectSession, ShardPopout};

pub fn draw_shard_panels(
    ui: &mut egui::Ui,
    window_rect: Rect,
    state: &mut NodeGraphUiState,
    _session: &ProjectSession, // Пока не используем, пригодится позже
    shard_name: &str,
    rtt_texture_id: Option<egui::TextureId>,
) {
    let ctx = ui.ctx().clone();
    let win_hash = window_rect.min.x.to_bits();
    let header_offset = 28.0;

    // 1. ЦЕНТРАЛЬНЫЙ 3D-ВИДЖЕТ (Занимает всё доступное пространство тайла)
    let center_rect = Rect::from_min_max(
        Pos2::new(window_rect.min.x, window_rect.min.y + header_offset),
        Pos2::new(window_rect.max.x, window_rect.max.y),
    );

    ui.allocate_ui_at_rect(center_rect, |ui| {
        ui.set_clip_rect(center_rect);
        let (rect, _resp) = ui.allocate_exact_size(center_rect.size(), egui::Sense::click_and_drag());

        state.cad_viewport_size = rect.size();
        state.cad_viewport_rect = Some(rect);

        // Перехват DND над 3D-сценой
        state.dragging_over_3d = None;
        if let Some((src_zone, src_port, _, is_input)) = state.dragging_pin.clone() {
            if ui.rect_contains_pointer(rect) {
                if let Some(mouse_pos) = ui.input(|i| i.pointer.interact_pos()) {
                    let local_pos = mouse_pos - rect.min;
                    if ui.input(|i| i.pointer.any_released()) {
                        state.pending_3d_drop = Some((src_zone, src_port, mouse_pos, local_pos.to_pos2(), is_input));
                        state.dragging_pin = None;
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

        // Рендер текстуры RTT
        if let Some(tex_id) = rtt_texture_id {
            let uv = egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            ui.painter().image(tex_id, rect, uv, Color32::WHITE);
            ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 55)));
        } else {
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(12, 12, 14));
            ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(30, 30, 35)));
        }
    });

    // 2. БИРКИ (TAGS) ПОВЕРХ ВЬЮПОРТА
    let tag_size = egui::vec2(36.0, 36.0);
    let padding = 12.0;

    let io_pos = center_rect.left_top() + egui::vec2(padding, padding);
    let bp_pos = center_rect.left_bottom() + egui::vec2(padding, -padding - tag_size.y);
    let an_pos = center_rect.right_top() + egui::vec2(-padding - tag_size.x, padding);

    let mut clicked_popout = None;

    let mut draw_tag = |pos: Pos2, text: &str, popout_type: ShardPopout| {
        let rect = egui::Rect::from_min_size(pos, tag_size);
        let resp = ui.interact(rect, ui.id().with(text).with(shard_name), egui::Sense::click());

        let is_active = state.active_popout == Some(popout_type.clone());
        let bg_color = if is_active {
            Color32::from_rgb(70, 100, 150)
        } else if resp.hovered() {
            Color32::from_rgb(50, 50, 55)
        } else {
            Color32::from_rgb(25, 25, 27)
        };

        ui.painter().rect_filled(rect, 6.0, bg_color);
        ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)));
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(14.0).clone(), Color32::WHITE);

        if resp.clicked() {
            clicked_popout = Some(popout_type);
        }
        rect
    };

    let io_rect = draw_tag(io_pos, "IO", ShardPopout::Io);
    let bp_rect = draw_tag(bp_pos, "BP", ShardPopout::Blueprints);
    let an_rect = draw_tag(an_pos, "AN", ShardPopout::Anatomy);

    if let Some(p) = clicked_popout {
        if state.active_popout == Some(p.clone()) {
            state.active_popout = None;
        } else {
            state.active_popout = Some(p);
        }
    }

    // 3. СТИЛИЗОВАННЫЕ POPOUT-ОКНА
    let mut close_popout = false;
    if let Some(popout) = &state.active_popout {
        let (anchor_pos, pivot) = match popout {
            ShardPopout::Io => (io_pos + egui::vec2(tag_size.x + 8.0, 0.0), egui::Align2::LEFT_TOP),
            ShardPopout::Blueprints => (bp_pos + egui::vec2(tag_size.x + 8.0, tag_size.y), egui::Align2::LEFT_BOTTOM),
            ShardPopout::Anatomy => (an_pos + egui::vec2(-8.0, 0.0), egui::Align2::RIGHT_TOP),
        };

        let area_id = egui::Id::new("shard_popout_area").with(win_hash);
        let area_resp = egui::Area::new(area_id)
            .fixed_pos(anchor_pos)
            .pivot(pivot)
            .order(egui::Order::Foreground)
            .show(&ctx, |ui| {
                // Эталонный стиль из context_menu_ui_system
                let frame = egui::Frame::menu(ui.style())
                    .fill(Color32::from_rgb(25, 25, 27))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)))
                    .rounding(6.0)
                    .inner_margin(egui::Margin::symmetric(12.0, 12.0));

                frame.show(ui, |ui| {
                    ui.set_max_width(280.0); // Только ограничение сверху, размер диктует контент

                    let title = match popout {
                        ShardPopout::Io => "I/O Matrix",
                        ShardPopout::Blueprints => "Blueprints",
                        ShardPopout::Anatomy => "Anatomy Layers",
                    };

                    ui.label(egui::RichText::new(title).color(Color32::WHITE).strong().size(14.0));
                    ui.add_space(4.0);
                    ui.painter().hline(ui.min_rect().x_range(), ui.cursor().min.y, egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 65)));
                    ui.add_space(8.0);

                    // Компактная пустышка
                    ui.label(egui::RichText::new("Content pending...").color(Color32::DARK_GRAY).size(12.0));
                });
            });

        // Закрытие по клику мимо окна и бирок
        if ctx.input(|i| i.pointer.any_click()) {
            let interact_pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            if !area_resp.response.rect.contains(interact_pos) 
                && !io_rect.contains(interact_pos) 
                && !bp_rect.contains(interact_pos) 
                && !an_rect.contains(interact_pos) {
                close_popout = true;
            }
        }
    }

    if close_popout {
        state.active_popout = None;
    }
}
