use crate::ui::utils::labeled_row;
use bevy_egui::egui;
use node_editor::domain::NeuronType;

pub fn draw_homeostasis_section(ui: &mut egui::Ui, nt: &mut NeuronType) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(
        egui::RichText::new("Homeostasis")
            .strong()
            .color(egui::Color32::from_rgb(200, 200, 220)),
    )
    .default_open(true)
    .show(ui, |ui| {
        changed |= labeled_row(ui, "Penalty", |ui| {
            ui.add(egui::DragValue::new(&mut nt.homeostasis_penalty).speed(100.0))
        })
        .changed();
        changed |= labeled_row(ui, "Decay", |ui| {
            ui.add(egui::DragValue::new(&mut nt.homeostasis_decay).speed(1.0))
        })
        .changed();
        changed |= labeled_row(ui, "Adaptive Max", |ui| {
            ui.add(egui::DragValue::new(&mut nt.adaptive_leak_max).speed(10.0))
        })
        .changed();
        changed |= labeled_row(ui, "Adaptive Gain", |ui| {
            ui.add(egui::DragValue::new(&mut nt.adaptive_leak_gain).speed(1.0))
        })
        .changed();
    });
    ui.add_space(5.0);
    changed
}
