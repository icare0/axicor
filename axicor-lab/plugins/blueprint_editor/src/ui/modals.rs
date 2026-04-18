use bevy_egui::egui;

/// DOD :      ( egui::Window)
pub fn draw_delete_type_modal(
    ctx: &egui::Context,
    window_rect: egui::Rect,
    type_name: &str,
) -> (bool, bool) { //  (confirmed, closed)
    let mut confirmed = false;
    let mut closed = false;

    let area_id = egui::Id::new("bp_delete_modal").with((window_rect.min.x.to_bits(), window_rect.min.y.to_bits()));
    egui::Area::new(area_id)
        .fixed_pos(window_rect.min)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_clip_rect(window_rect);

            //     ()
            let (bg_rect, bg_resp) = ui.allocate_exact_size(window_rect.size(), egui::Sense::click());
            if bg_resp.clicked() { closed = true; }
            ui.painter().rect_filled(bg_rect, 0.0, egui::Color32::from_black_alpha(200));

            let frame = egui::Frame::window(ui.style()).inner_margin(20.0).rounding(8.0).fill(egui::Color32::from_rgb(30, 30, 35));
            let frame_rect = egui::Rect::from_center_size(window_rect.center(), egui::vec2(280.0, 120.0));

            ui.allocate_ui_at_rect(frame_rect, |ui| {
                frame.show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("[WARN] Delete Neuron Type").color(egui::Color32::RED).strong().size(16.0));
                        ui.add_space(8.0);
                        ui.label(format!("Delete '{}'?\nAll related dendrite filters will be cleared.", type_name));
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new("Yes, Delete").color(egui::Color32::RED)).clicked() {
                                confirmed = true;
                                closed = true;
                            }
                            if ui.button("Cancel").clicked() {
                                closed = true;
                            }
                        });
                    });
                });
            });
        });

    (confirmed, closed)
}
