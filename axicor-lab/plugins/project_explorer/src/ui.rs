use crate::domain::ProjectModel;
use bevy_egui::egui;

/// Чистая функция отрисовки. Никаких ECS-мутаций.
pub fn render_explorer_ui<FLoad, FZone>(
    ui: &mut egui::Ui,
    projects: &[ProjectModel],
    mut on_load_topology: FLoad,
    mut on_zone_selected: FZone,
) where
    FLoad: FnMut(String),
    FZone: FnMut(String, String),
{
    render_project_list(ui, projects, &mut on_load_topology, &mut on_zone_selected);
}

// ---------------------------------------------------------------------------

fn render_project_list<FLoad, FZone>(
    ui: &mut egui::Ui,
    projects: &[ProjectModel],
    on_load_topology: &mut FLoad,
    on_zone_selected: &mut FZone,
) where
    FLoad: FnMut(String),
    FZone: FnMut(String, String),
{
    for proj in projects {
        match proj.is_bundle {
            true  => render_bundle(ui, proj, on_load_topology, on_zone_selected),
            false => render_raw_dir(ui, proj),
        }
        ui.add_space(4.0);
    }
}

fn render_bundle<FLoad, FZone>(
    ui: &mut egui::Ui,
    proj: &ProjectModel,
    on_load_topology: &mut FLoad,
    on_zone_selected: &mut FZone,
) where
    FLoad: FnMut(String),
    FZone: FnMut(String, String),
{
    let header = egui::CollapsingHeader::new(format!("📦 {}", proj.name))
        .id_source(&proj.name)
        .show(ui, |ui| {
            ui.add_space(4.0);
            for (i, zone) in proj.shards.iter().enumerate() {
                ui.push_id(i, |ui| {
                    egui::CollapsingHeader::new(format!("Zone: {}", zone))
                        .id_source(zone)
                        .show(ui, |ui| {
                            if ui.selectable_label(false, format!("Shard: {}", zone)).clicked() {
                                on_zone_selected(proj.name.clone(), zone.clone());
                            }
                        });
                });
            }
        });

    if header.header_response.clicked() {
        on_load_topology(proj.name.clone());
    }
}

fn render_raw_dir(ui: &mut egui::Ui, proj: &ProjectModel) {
    egui::CollapsingHeader::new(format!("📂 {}", proj.name))
        .id_source(&proj.name)
        .show(ui, |ui| { ui.label("Raw Directory"); });
}