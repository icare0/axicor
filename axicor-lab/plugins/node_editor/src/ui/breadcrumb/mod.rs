pub mod crumb_model_system;
pub mod crumb_department_system;

use bevy_egui::egui::{self, Color32};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState};

pub fn draw_breadcrumbs(
    ui: &mut egui::Ui,
    graph: &BrainTopologyGraph,
    state: &mut NodeGraphUiState,
    send_open: &mut impl FnMut(std::path::PathBuf),
) {
    // Рисуем крошки строго по иерархии: Дед -> Батя -> Я
    
    // 1. Модель
    crumb_model_system::draw_model_crumb(ui, graph, state, &mut *send_open);
    ui.label(egui::RichText::new("›").color(Color32::DARK_GRAY));

    // 2. Департамент
    crumb_department_system::draw_department_crumb(ui, graph, state, &mut *send_open);
}
