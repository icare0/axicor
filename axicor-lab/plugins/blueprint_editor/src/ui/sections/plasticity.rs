use crate::ui::utils::labeled_row;
use bevy_egui::egui;
use node_editor::domain::NeuronType;

pub fn draw_plasticity_section(ui: &mut egui::Ui, nt: &mut NeuronType) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(
        egui::RichText::new("Neuromodulation (R-STDP)")
            .strong()
            .color(egui::Color32::from_rgb(200, 200, 220)),
    )
    .default_open(true)
    .show(ui, |ui| {
        changed |= labeled_row(ui, "D1 Affinity (LTP)", |ui| {
            ui.add(
                egui::DragValue::new(&mut nt.d1_affinity)
                    .speed(1.0)
                    .clamp_range(0..=255),
            )
        })
        .changed();
        changed |= labeled_row(ui, "D2 Affinity (LTD)", |ui| {
            ui.add(
                egui::DragValue::new(&mut nt.d2_affinity)
                    .speed(1.0)
                    .clamp_range(0..=255),
            )
        })
        .changed();
        changed |= labeled_row(ui, "GSOP Potentiation", |ui| {
            ui.add(egui::DragValue::new(&mut nt.gsop_potentiation).speed(10.0))
        })
        .changed();
        changed |= labeled_row(ui, "GSOP Depression", |ui| {
            ui.add(egui::DragValue::new(&mut nt.gsop_depression).speed(10.0))
        })
        .changed();
        changed |= labeled_row(ui, "Initial Weight", |ui| {
            ui.add(egui::DragValue::new(&mut nt.initial_synapse_weight).speed(10.0))
        })
        .changed();
    });
    ui.add_space(5.0);
    changed
}
