use bevy_egui::egui;
use node_editor::domain::NeuronType;
use crate::ui::utils::labeled_row;

pub fn draw_sprouting_section(ui: &mut egui::Ui, nt: &mut NeuronType) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(egui::RichText::new("Sprouting (Night Growth)").strong().color(egui::Color32::from_rgb(200, 200, 220)))
        .default_open(true)
        .show(ui, |ui| {
            changed |= labeled_row(ui, "Distance Weight", |ui| ui.add(egui::Slider::new(&mut nt.sprouting_weight_distance, 0.0..=1.0))).changed();
            changed |= labeled_row(ui, "Power Weight", |ui| ui.add(egui::Slider::new(&mut nt.sprouting_weight_power, 0.0..=1.0))).changed();
            changed |= labeled_row(ui, "Explore Weight", |ui| ui.add(egui::Slider::new(&mut nt.sprouting_weight_explore, 0.0..=1.0))).changed();
            
            // Валидация суммы (Инвариант: сумма весов должна быть = 1.0)
            let sum = nt.sprouting_weight_distance + nt.sprouting_weight_power + nt.sprouting_weight_explore + nt.type_affinity;
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if (sum - 1.0).abs() > 0.05 {
                    ui.label(egui::RichText::new(format!("⚠ Sum: {:.2} (expected 1.0)", sum)).color(egui::Color32::from_rgb(255, 180, 50)));
                } else {
                    ui.label(egui::RichText::new(format!("✓ Sum: {:.2}", sum)).color(egui::Color32::from_rgb(80, 200, 120)));
                }
            });
        });
    ui.add_space(5.0);
    changed
}
