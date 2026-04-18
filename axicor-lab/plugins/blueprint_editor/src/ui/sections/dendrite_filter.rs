use bevy_egui::egui;
use node_editor::domain::NeuronType;

pub fn draw_dendrite_filter_section(
    ui: &mut egui::Ui,
    nt: &mut NeuronType,
    all_type_names: &[String],
) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(
        egui::RichText::new("Dendrite Whitelist")
            .strong()
            .color(egui::Color32::from_rgb(200, 200, 220)),
    )
    .default_open(true)
    .show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if nt.dendrite_whitelist.is_empty() {
                ui.label(egui::RichText::new("All types accepted").color(egui::Color32::GRAY));
            } else {
                let mut to_remove = None;
                for (i, allowed_type) in nt.dendrite_whitelist.iter().enumerate() {
                    let text = egui::RichText::new(format!("{} ", allowed_type))
                        .color(egui::Color32::WHITE);
                    //  ""
                    if ui
                        .add(
                            egui::Button::new(text)
                                .fill(egui::Color32::from_rgb(40, 40, 50))
                                .rounding(8.0),
                        )
                        .clicked()
                    {
                        to_remove = Some(i);
                    }
                }
                if let Some(i) = to_remove {
                    nt.dendrite_whitelist.remove(i);
                    changed = true;
                }
            }

            //
            ui.menu_button(
                egui::RichText::new(" + ")
                    .strong()
                    .color(egui::Color32::from_rgb(100, 200, 255)),
                |ui| {
                    for type_name in all_type_names {
                        if !nt.dendrite_whitelist.contains(type_name) {
                            if ui.button(type_name).clicked() {
                                nt.dendrite_whitelist.push(type_name.clone());
                                changed = true;
                                ui.close_menu();
                            }
                        }
                    }
                },
            );
        });
    });
    ui.add_space(5.0);
    changed
}
