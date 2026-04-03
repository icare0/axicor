use bevy_egui::egui;
use node_editor::domain::NeuronType;

pub enum TypeAction {
    None,
    Add,
    Delete,
}

pub fn draw_type_selector(ui: &mut egui::Ui, types: &[NeuronType], selected_idx: &mut usize) -> TypeAction {
    if types.is_empty() { return TypeAction::None; }

    let mut action = TypeAction::None;
    let current_name = types.get(*selected_idx).map(|t| t.name.clone()).unwrap_or_else(|| "Unknown".to_string());

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Neuron Type:").strong().color(egui::Color32::from_rgb(200, 200, 220)));
        
        egui::ComboBox::from_id_source("type_selector")
            .selected_text(current_name)
            .width(ui.available_width() - 60.0) // Оставляем место для кнопок
            .show_ui(ui, |ui| {
                for (i, t) in types.iter().enumerate() {
                    let text = format!("[{}] {}", i, t.name);
                    let color = if t.is_inhibitory { egui::Color32::from_rgb(200, 80, 80) } else { egui::Color32::from_rgb(80, 200, 120) };
                    ui.selectable_value(selected_idx, i, egui::RichText::new(text).color(color));
                }
            });

        // [DOD FIX] Аппаратные лимиты (макс 16, мин 1)
        if ui.add_enabled(types.len() < 16, egui::Button::new("+")).on_hover_text("Add new type (Max 16)").clicked() {
            action = TypeAction::Add;
        }
        if ui.add_enabled(types.len() > 1, egui::Button::new("🗑")).on_hover_text("Delete selected type").clicked() {
            action = TypeAction::Delete;
        }
    });

    action
}
