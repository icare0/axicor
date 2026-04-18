use crate::ui::utils::labeled_row;
use bevy_egui::egui;
use node_editor::domain::NeuronType;

pub fn draw_membrane_section(ui: &mut egui::Ui, nt: &mut NeuronType) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(
        egui::RichText::new("Membrane Physics")
            .strong()
            .color(egui::Color32::from_rgb(200, 200, 220)),
    )
    .default_open(true)
    .show(ui, |ui| {
        changed |= labeled_row(ui, "Is Inhibitory", |ui| {
            ui.checkbox(&mut nt.is_inhibitory, "")
        })
        .changed();
        changed |= labeled_row(ui, "Threshold", |ui| {
            ui.add(egui::DragValue::new(&mut nt.threshold).speed(10.0))
        })
        .changed();
        changed |= labeled_row(ui, "Rest Potential", |ui| {
            ui.add(egui::DragValue::new(&mut nt.rest_potential).speed(1.0))
        })
        .changed();
        changed |= labeled_row(ui, "Leak Rate", |ui| {
            ui.add(
                egui::DragValue::new(&mut nt.leak_rate)
                    .speed(1.0)
                    .clamp_range(0..=10000),
            )
        })
        .changed();
    });
    ui.add_space(5.0);
    changed
}
