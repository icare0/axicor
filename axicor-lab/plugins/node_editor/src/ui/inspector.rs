// ui/inspector.rs
use bevy_egui::egui::{self, Color32, Rect};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};

pub fn draw_inspector_panel(
    ui: &mut egui::Ui,
    rect: Rect,
    graph: &mut BrainTopologyGraph,
    state: &mut NodeGraphUiState,
    _send_mutation: &mut impl FnMut(TopologyMutation),
) {
    ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(25, 25, 28));
    ui.painter().line_segment([rect.left_top(), rect.left_bottom()], egui::Stroke::new(1.0, Color32::from_rgb(50, 50, 50)));

    ui.allocate_ui_at_rect(rect, |ui| {
        egui::Frame::none().inner_margin(12.0).show(ui, |ui| {
            if let Some(selected) = state.selected_node.clone() {
                ui.heading(egui::RichText::new(&selected).color(Color32::WHITE).strong());
                ui.add_space(10.0);

                ui.label(egui::RichText::new("Zone Properties").color(Color32::GRAY));
                ui.separator();

                egui::Grid::new("props_grid").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
                    ui.label("Prune Threshold:");
                    // Заглушка, позже привяжем к config.plasticity
                    let mut dummy_prune = 15;
                    ui.add(egui::Slider::new(&mut dummy_prune, 0..=5000));
                    ui.end_row();

                    ui.label("Max Sprouts:");
                    let mut dummy_sprouts = 16;
                    ui.add(egui::Slider::new(&mut dummy_sprouts, 0..=128));
                    ui.end_row();

                    ui.label("Night Interval:");
                    let mut dummy_night = 100_000;
                    ui.add(egui::Slider::new(&mut dummy_night, 0..=500_000).text("ticks"));
                    ui.end_row();
                });

                ui.add_space(20.0);
                ui.label(egui::RichText::new("Hardware Metrics").color(Color32::GRAY));
                ui.separator();
                
                let inputs = graph.node_inputs.get(&selected).map(|v| v.len()).unwrap_or(0);
                let outputs = graph.node_outputs.get(&selected).map(|v| v.len()).unwrap_or(0);
                ui.label(format!("Input Matrices: {}", inputs));
                ui.label(format!("Output Matrices: {}", outputs));

            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new("Select a node\nto inspect properties").color(Color32::DARK_GRAY));
                });
            }
        });
    });
}
