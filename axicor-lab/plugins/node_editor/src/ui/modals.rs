use bevy_egui::egui;
use crate::domain::NodeGraphUiState;

pub fn draw_clear_graph_modal(
    ctx: &egui::Context,
    window_rect: egui::Rect,
    state: &mut NodeGraphUiState,
) -> bool {
    let mut confirmed = false;
    let mut close = false;

    // Area for the modal
    let area_id = egui::Id::new("clear_modal_area").with((window_rect.min.x.to_bits(), window_rect.min.y.to_bits()));
    egui::Area::new(area_id)
        .fixed_pos(window_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Clip to window bounds
            ui.set_clip_rect(window_rect);

            // 1. Dim background + Click-to-Close
            let (bg_rect, bg_resp) = ui.allocate_exact_size(window_rect.size(), egui::Sense::click());
            if bg_resp.clicked() {
                close = true;
            }
            ui.painter().rect_filled(bg_rect, 0.0, egui::Color32::from_black_alpha(200));

            // 2. Main dialog (manual Window layout via Frame)
            let frame = egui::Frame::window(ui.style()).inner_margin(20.0).rounding(8.0);
            let frame_rect = egui::Rect::from_center_size(window_rect.center(), egui::vec2(300.0, 150.0));

            ui.allocate_ui_at_rect(frame_rect, |ui| {
                frame.show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("[WARN] Warning").color(egui::Color32::GOLD).strong().size(18.0));
                        ui.add_space(8.0);
                        ui.label("Are you sure you want to completely delete all nodes?\nThis action cannot be undone.");
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new("Yes, Delete All").color(egui::Color32::RED)).clicked() {
                                confirmed = true;
                                close = true;
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        });
                    });
                });
            });
        });

    if close {
        state.show_clear_modal = false;
    }

    confirmed
}
