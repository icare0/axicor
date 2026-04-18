use crate::ui::utils::labeled_row;
use bevy_egui::egui;
use node_editor::domain::NeuronType;

pub fn draw_timing_section(ui: &mut egui::Ui, nt: &mut NeuronType) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(
        egui::RichText::new("Timing & Routing")
            .strong()
            .color(egui::Color32::from_rgb(200, 200, 220)),
    )
    .default_open(true)
    .show(ui, |ui| {
        changed |= labeled_row(ui, "Refractory Period", |ui| {
            ui.add(
                egui::DragValue::new(&mut nt.refractory_period)
                    .speed(1.0)
                    .clamp_range(0..=255),
            )
        })
        .changed();
        changed |= labeled_row(ui, "Synapse Refract", |ui| {
            ui.add(
                egui::DragValue::new(&mut nt.synapse_refractory_period)
                    .speed(1.0)
                    .clamp_range(0..=255),
            )
        })
        .changed();
        changed |= labeled_row(ui, "Propagation Len", |ui| {
            ui.add(
                egui::DragValue::new(&mut nt.signal_propagation_length)
                    .speed(1.0)
                    .clamp_range(0..=255),
            )
        })
        .changed();
        changed |= labeled_row(ui, "Spontaneous Ticks", |ui| {
            ui.add(egui::DragValue::new(&mut nt.spontaneous_firing_period_ticks).speed(10.0))
        })
        .changed();
    });
    ui.add_space(5.0);
    changed
}
