use bevy::prelude::*;
use bevy_egui::egui;
use crate::domain::CodeEditorState;
use layout_api::{PluginWindow, base_domain, DOMAIN_CODE_EDITOR};

// --- Цвета подсветки TOML (инициализируются один раз) ---
const CLR_COMMENT: egui::Color32 = egui::Color32::from_gray(120);
const CLR_SECTION: egui::Color32 = egui::Color32::LIGHT_BLUE;
const CLR_KEY:     egui::Color32 = egui::Color32::KHAKI;
const CLR_VALUE:   egui::Color32 = egui::Color32::WHITE;
const CLR_DROP:    egui::Color32 = egui::Color32::YELLOW;
const DND_ID:      &str          = "dnd_path";

pub fn render_code_editor_system(
    mut contexts: bevy_egui::EguiContexts,
    mut windows: Query<(&PluginWindow, &mut CodeEditorState)>,
    mut topo_ev: EventWriter<layout_api::TopologyChangedEvent>,
) {
    let Some(ctx) = contexts.try_ctx_mut() else { return };

    for (window, mut state) in windows.iter_mut() {
        if !window.is_visible { continue; }
        if base_domain(&window.plugin_id) != DOMAIN_CODE_EDITOR { continue; }

        // Стабильный ID без format! — хешируем plugin_id
        let area_id = egui::Id::new(&window.plugin_id);

        egui::Area::new(area_id)
            .fixed_pos(window.rect.min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_clip_rect(window.rect);

                let (content_rect, _) = layout_api::draw_unified_header(ui, window.rect, "Code Editor");

                handle_dnd_drop(ui, content_rect, &mut state);

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(30, 30, 30))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            render_top_bar(ui, &mut state, &mut topo_ev);
                            render_editor(ui, &mut state);
                        });
                });
            });
    }
}

// ---------------------------------------------------------------------------

fn handle_dnd_drop(ui: &mut egui::Ui, rect: egui::Rect, state: &mut CodeEditorState) {
    if !ui.rect_contains_pointer(rect) { return; }

    let dragged = ui.memory_mut(|m| {
        m.data.get_temp::<std::path::PathBuf>(egui::Id::new(DND_ID))
    });
    let Some(path) = dragged else { return };

    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(2.0, CLR_DROP));

    if !ui.input(|i| i.pointer.any_released()) { return; }

    match std::fs::read_to_string(&path) {
        Ok(content) => { state.content = content; }
        Err(e) => { error!("[CodeEditor] Failed to read dropped file: {}", e); return; }
    }
    state.current_file = Some(path);
    ui.memory_mut(|m| m.data.remove_temp::<std::path::PathBuf>(egui::Id::new(DND_ID)));
}

fn render_top_bar(
    ui: &mut egui::Ui,
    state: &mut CodeEditorState,
    topo_ev: &mut EventWriter<layout_api::TopologyChangedEvent>,
) {
    egui::TopBottomPanel::top(ui.id().with("top")).show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            let file_name = state.current_file.as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "No File".to_string());

            ui.label(egui::RichText::new(format!("📝 {}", file_name)).strong());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("💾 Apply").clicked() {
                    save_and_notify(state, topo_ev);
                }
            });
        });
    });
}

fn save_and_notify(
    state: &mut CodeEditorState,
    topo_ev: &mut EventWriter<layout_api::TopologyChangedEvent>,
) {
    let Some(path) = &state.current_file else { return };

    if let Err(e) = std::fs::write(path, &state.content) {
        error!("[CodeEditor] Save failed: {}", e);
        return;
    }

    info!("[CodeEditor] Saved: {:?}", path);

    // Извлекаем имя проекта: ожидаем Genesis-Models/<project>/<file>
    let project_name = path.components().nth(1)
        .map(|c| c.as_os_str().to_string_lossy().to_string());

    if let Some(name) = project_name {
        topo_ev.send(layout_api::TopologyChangedEvent { project_name: name });
    } else {
        warn!("[CodeEditor] Could not extract project name from path: {:?}", path);
    }
}

fn render_editor(ui: &mut egui::Ui, state: &mut CodeEditorState) {
    egui::CentralPanel::default().show_inside(ui, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut layouter = |ui: &egui::Ui, string: &str, _wrap: f32| {
                toml_layout_job(ui, string)
            };
            ui.add(
                egui::TextEdit::multiline(&mut state.content)
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
            );
        });
    });
}

/// TOML-подсветка. Вызывается только при изменении текста (egui кэширует layout_job).
fn toml_layout_job(ui: &egui::Ui, text: &str) -> std::sync::Arc<egui::Galley> {
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();

    for line in text.split('\n') {
        let trimmed = line.trim_start();
        let color = if trimmed.starts_with('#') {
            CLR_COMMENT
        } else if trimmed.starts_with('[') {
            CLR_SECTION
        } else if let Some(eq) = line.find('=') {
            // Ключ и значение — разные цвета
            job.append(&line[..eq], 0.0, fmt(CLR_KEY,   &mono));
            job.append(&line[eq..], 0.0, fmt(CLR_VALUE, &mono));
            job.append("\n",        0.0, egui::TextFormat::default());
            continue;
        } else {
            CLR_VALUE
        };
        job.append(line, 0.0, fmt(color, &mono));
        job.append("\n", 0.0, egui::TextFormat::default());
    }

    ui.fonts(|f| f.layout_job(job))
}

#[inline]
fn fmt(color: egui::Color32, font_id: &egui::FontId) -> egui::TextFormat {
    egui::TextFormat { color, font_id: font_id.clone(), ..Default::default() }
}