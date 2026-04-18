use bevy_egui::egui;
use node_editor::domain::NeuronType;

pub fn draw_inertia_section(ui: &mut egui::Ui, nt: &mut NeuronType) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(egui::RichText::new("Inertia Curve").strong().color(egui::Color32::from_rgb(200, 200, 220)))
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for i in 0..16 {
                    let val = &mut nt.inertia_curve[i];
                    let height = (*val as f32 / 255.0) * 40.0;
                    
                    // Single column
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(12.0, 40.0), egui::Sense::click_and_drag());
                    
                    // Gradient: Plastic (Blue) -> Monumental (Red)
                    let t = i as f32 / 15.0;
                    let color = egui::Color32::from_rgb((t * 255.0) as u8, 80, ((1.0 - t) * 255.0) as u8);

                    // Drag-and-drop value adjustment
                    if resp.dragged() || resp.clicked() {
                        if let Some(pos) = ui.ctx().pointer_interact_pos() {
                            let new_h = (rect.bottom() - pos.y).clamp(0.0, 40.0);
                            *val = ((new_h / 40.0) * 255.0) as u8;
                            changed = true;
                        }
                    }

                    // Background and bar rendering
                    let bar_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.min.x, rect.max.y - height),
                        rect.max
                    );
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_black_alpha(50)); // Channel background
                    ui.painter().rect_filled(bar_rect, 1.0, color); // Visualization

                }
            });
            ui.label(egui::RichText::new("Rank 0 (Plastic)  Rank 15 (Monumental)").color(egui::Color32::DARK_GRAY).size(10.0));
        });
    ui.add_space(5.0);
    changed
}
