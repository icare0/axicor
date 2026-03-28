use bevy_egui::egui;
use crate::domain::ProjectModel;

const EXPLORER_MIN_WIDTH: f32 = 200.0;
const EXPLORER_MAX_WIDTH: f32 = 300.0;
const SECTION_SPACING: f32 = 3.0;
const SEPARATOR_FRACTION: f32 = 0.9;

pub fn draw_explorer_tree<FLoad, FZone>(
    ui: &mut egui::Ui,
    bundles: &[&ProjectModel],
    sources: &[&ProjectModel],
    mut on_load: FLoad,
    mut on_zone: FZone,
) where
    FLoad: FnMut(String),
    FZone: FnMut(String, String),
{
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.set_min_width(EXPLORER_MIN_WIDTH);
        ui.set_max_width(EXPLORER_MAX_WIDTH);
        ui.add_space(3.0);

        draw_bundles(ui, bundles, &mut on_load, &mut on_zone);

        if !bundles.is_empty() && !sources.is_empty() {
            draw_separator(ui);
        }

        draw_sources(ui, sources);
    });
}

fn draw_bundles<FLoad, FZone>(
    ui: &mut egui::Ui,
    bundles: &[&ProjectModel],
    on_load: &mut FLoad,
    on_zone: &mut FZone,
) where
    FLoad: FnMut(String),
    FZone: FnMut(String, String),
{
    for project in bundles {
        ui.add_space(10.0); // отступ от разделителя до первой папки
        let header = egui::CollapsingHeader::new(format!("📦 {}", project.name))
            .id_source(&project.name)
            .show(ui, |ui| {
                ui.add_space(SECTION_SPACING);
                draw_bundle_zones(ui, project, on_zone);
            });

        if header.header_response.clicked() {
            on_load(project.name.clone());
        }

        ui.add_space(SECTION_SPACING);
    }
}

fn draw_bundle_zones<FZone>(
    ui: &mut egui::Ui,
    project: &ProjectModel,
    on_zone: &mut FZone,
) where
    FZone: FnMut(String, String),
{
    for (i, zone) in project.shards.iter().enumerate() {
        ui.push_id(i, |ui| {
            egui::CollapsingHeader::new(format!("Zone: {}", zone))
                .id_source(zone)
                .show(ui, |ui| {
                    if ui.selectable_label(false, format!("Shard: {}", zone)).clicked() {
                        on_zone(project.name.clone(), zone.clone());
                    }
                });
        });
    }
}

fn draw_sources(ui: &mut egui::Ui, sources: &[&ProjectModel]) {
    for project in sources {
        egui::CollapsingHeader::new(format!("📁 {}", project.name))
            .id_source(&project.name)
            .show(ui, |ui| {
                if project.dna_files.is_empty() {
                    ui.label(
                        egui::RichText::new("Empty Directory")
                            .color(egui::Color32::DARK_GRAY),
                    );
                } else {
                    for file in &project.dna_files {
                        // TODO: открытие в Code Editor / Node Editor
                        ui.selectable_label(false, format!("📄 {}", file));
                    }
                }
            });

        ui.add_space(SECTION_SPACING);
    }
}

fn draw_separator(ui: &mut egui::Ui) {
    // Убираем верхний add_space — он уже пришёл от последнего bundle
    let total_w = ui.available_width();
    let line_w = total_w * SEPARATOR_FRACTION;
    let pad = (total_w - line_w) / 2.0;

    ui.horizontal(|ui| {
        ui.add_space(pad);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(line_w, 1.0), egui::Sense::hover());
        ui.painter().hline(
            rect.x_range(),
            rect.center().y,
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
        );
    });

    ui.add_space(2.0); // меньший отступ снизу разделителя
}