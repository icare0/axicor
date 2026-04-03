use bevy_egui::egui;
use node_editor::domain::NeuronType;
use crate::ui::utils::labeled_row;

pub fn draw_growth_section(ui: &mut egui::Ui, nt: &mut NeuronType) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(egui::RichText::new("Axon Growth & Routing").strong().color(egui::Color32::from_rgb(200, 200, 220)))
        .default_open(true)
        .show(ui, |ui| {
            changed |= labeled_row(ui, "Steering FOV (°)", |ui| ui.add(egui::DragValue::new(&mut nt.steering_fov_deg).speed(1.0).clamp_range(10.0..=180.0))).changed();
            changed |= labeled_row(ui, "Steering Radius (µm)", |ui| ui.add(egui::DragValue::new(&mut nt.steering_radius_um).speed(5.0))).changed();
            changed |= labeled_row(ui, "Weight: Inertia", |ui| ui.add(egui::Slider::new(&mut nt.steering_weight_inertia, 0.0..=1.0))).changed();
            changed |= labeled_row(ui, "Weight: Sensor", |ui| ui.add(egui::Slider::new(&mut nt.steering_weight_sensor, 0.0..=1.0))).changed();
            changed |= labeled_row(ui, "Weight: Jitter", |ui| ui.add(egui::Slider::new(&mut nt.steering_weight_jitter, 0.0..=1.0))).changed();
            changed |= labeled_row(ui, "Vertical Bias", |ui| ui.add(egui::Slider::new(&mut nt.growth_vertical_bias, 0.0..=1.0))).changed();
            changed |= labeled_row(ui, "Type Affinity", |ui| ui.add(egui::Slider::new(&mut nt.type_affinity, 0.0..=1.0))).changed();
        });
    ui.add_space(5.0);
    changed
}
