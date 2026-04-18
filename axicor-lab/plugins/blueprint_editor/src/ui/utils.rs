use bevy_egui::egui;

///    "Label : Control" (Blender Properties Style)
pub fn labeled_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    content: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.horizontal(|ui| {
        ui.add_sized(
            [140.0, 0.0],
            egui::Label::new(
                egui::RichText::new(label).color(egui::Color32::from_rgb(180, 180, 180)),
            ),
        );
        content(ui)
    })
    .inner
}
