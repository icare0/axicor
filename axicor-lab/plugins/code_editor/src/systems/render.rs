use bevy::prelude::*;
use bevy_egui::egui;
use crate::domain::CodeEditorState;
use layout_api::{PluginWindow, base_domain, DOMAIN_CODE_EDITOR};

// --- Цвета подсветки TOML (инициализируются один раз) ---
const CLR_COMMENT: egui::Color32 = egui::Color32::from_gray(120);
const CLR_SECTION: egui::Color32 = egui::Color32::LIGHT_BLUE;
const CLR_VALUE:   egui::Color32 = egui::Color32::WHITE;
const CLR_DROP:    egui::Color32 = egui::Color32::YELLOW;
const DND_ID:      &str          = "dnd_path";

pub fn render_code_editor_system(
    mut contexts: bevy_egui::EguiContexts,
    mut windows: Query<(&PluginWindow, &mut CodeEditorState)>,
    mut topo_ev: EventWriter<layout_api::TopologyChangedEvent>,
    mut open_ev: EventWriter<layout_api::OpenFileEvent>,
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

                let (content_rect, _) = layout_api::draw_unified_header(ui, window.rect, "");

                // [DOD FIX] Вычисляем зону для вкладок (отступ SYS_UI_SAFE_ZONE для DND якоря)
                let mut header_rect = window.rect;
                header_rect.set_height(28.0);
                header_rect.min.x += layout_api::SYS_UI_SAFE_ZONE;
                let mut header_ui = ui.child_ui(header_rect, egui::Layout::left_to_right(egui::Align::Center));
                render_top_bar(&mut header_ui, &mut state, &mut topo_ev, &mut open_ev);

                handle_dnd_drop(ui, content_rect, &mut state);

                ui.allocate_ui_at_rect(content_rect, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(30, 30, 30))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
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

    match layout_api::overlay_read_to_string(&path) {
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
    open_ev: &mut EventWriter<layout_api::OpenFileEvent>,
) {
    ui.horizontal(|ui| {
        let is_modified = state.content != state.saved_content;

        // Если файл является частью шарда (io, shard, anatomy, blueprints), показываем вкладки
        let path = state.current_file.as_ref();
        let is_shard_level = path.map_or(false, |p| {
            let n = p.file_name().unwrap_or_default().to_string_lossy();
            n == "shard.toml" || n == "io.toml" || n == "anatomy.toml" || n == "blueprints.toml"
        });

        if is_shard_level {
            if let Some(p) = path {
                let parent = p.parent().unwrap();
                for name in &["shard.toml", "io.toml", "anatomy.toml", "blueprints.toml"] {
                    let sibling = parent.join(name);
                    // Проверяем существование файла либо в Cold, либо в Sandbox
                    if sibling.exists() || layout_api::resolve_sandbox_path(&sibling).exists() {
                        let is_active = Some(&sibling) == path;
                        let modified_star = if is_active && is_modified { " *" } else { "" };
                        
                        let mut rich_text = egui::RichText::new(format!("📄 {}{}", name, modified_star));
                        if is_active { rich_text = rich_text.color(egui::Color32::WHITE).strong(); }
                        else { rich_text = rich_text.color(egui::Color32::GRAY); }

                        if ui.selectable_label(is_active, rich_text).clicked() && !is_active {
                            open_ev.send(layout_api::OpenFileEvent { path: sibling });
                        }
                    }
                }
            }
        } else {
            let name = path.and_then(|p| p.file_name()).unwrap_or_default().to_string_lossy();
            let modified_star = if is_modified { " *" } else { "" };
            let _ = ui.selectable_label(true, egui::RichText::new(format!("📝 {}{}", name, modified_star)).strong());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let apply_btn = egui::Button::new("💾 Apply");
            let apply_btn = if is_modified { apply_btn.fill(egui::Color32::from_rgb(0, 100, 200)) } else { apply_btn };
            
            if ui.add_enabled(is_modified, apply_btn).clicked() {
                save_and_notify(state, topo_ev);
            }
        });
    });
}

fn save_and_notify(
    state: &mut CodeEditorState,
    topo_ev: &mut EventWriter<layout_api::TopologyChangedEvent>,
) {
    let Some(path) = &state.current_file else { return };

    // [DOD FIX] Защита чистых данных: пишем строго в Sandbox
    let sandbox_path = layout_api::resolve_sandbox_path(path);
    if let Some(p) = sandbox_path.parent() { let _ = std::fs::create_dir_all(p); }

    if let Err(e) = std::fs::write(&sandbox_path, &state.content) {
        error!("[CodeEditor] Save failed: {}", e);
        return;
    }

    state.saved_content = state.content.clone();
    info!("[CodeEditor] Saved to Sandbox: {:?}", sandbox_path);

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
    let max_h = ui.available_height();
    
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .max_height(max_h)
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                // Выделяем фиксированное место слева под колонку номеров
                let gutter_width = 36.0;
                let (gutter_rect, _) = ui.allocate_exact_size(
                    egui::vec2(gutter_width, 0.0), 
                    egui::Sense::hover()
                );

                let text_area_width = ui.available_width();
                
                let mut layouter = |ui: &egui::Ui, string: &str, _wrap: f32| {
                    toml_layout_job(ui, string)
                };

                let output = egui::TextEdit::multiline(&mut state.content)
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .frame(false)
                    .desired_width(text_area_width)
                    .layouter(&mut layouter)
                    .show(ui);

                let galley = output.galley;
                let text_rect = output.response.rect;

                // Вспомогательная логика для номеров строк (Line by Line Diff)
                let saved_lines: Vec<&str> = state.saved_content.split('\n').collect();
                let current_lines: Vec<&str> = state.content.split('\n').collect();
                
                let painter = ui.painter();
                let font_id = egui::TextStyle::Monospace.resolve(ui.style());

                // Отрисовываем номера строк на точных пиксельных Y-координатах,
                // извлеченных напрямую из движка рендеринга текста (Galley).
                let mut logical_line = 0;
                for row in &galley.rows {
                    // Так как wrap отключён, 1 visual row = 1 logical line.
                    if logical_line < current_lines.len() {
                        let text_line = current_lines.get(logical_line).copied().unwrap_or("");
                        let saved_line = saved_lines.get(logical_line).copied().unwrap_or("");
                        
                        let color = if text_line != saved_line {
                            egui::Color32::from_rgb(100, 255, 100)
                        } else {
                            egui::Color32::from_gray(100)
                        };

                        // row.y_min / y_max это сдвиг относительно начала виджета (text_rect.min)
                        let y_center = text_rect.min.y + row.rect.center().y;
                        
                        // Рисуем текст номера линии
                        painter.text(
                            egui::pos2(gutter_rect.max.x - 8.0, y_center),
                            egui::Align2::RIGHT_CENTER,
                            format!("{}", logical_line + 1),
                            font_id.clone(),
                            color,
                        );
                        
                        logical_line += 1;
                    }
                }
            });
        });
}

fn toml_layout_job(ui: &egui::Ui, text: &str) -> std::sync::Arc<egui::Galley> {
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = f32::INFINITY; // Disable text wrapping to keep line numbers synced

    for line in text.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            job.append(line, 0.0, fmt(CLR_COMMENT, &mono));
            job.append("\n", 0.0, egui::TextFormat::default());
        } else if trimmed.starts_with('[') {
            job.append(line, 0.0, fmt(CLR_SECTION, &mono));
            job.append("\n", 0.0, egui::TextFormat::default());
        } else if let Some(eq) = line.find('=') {
            job.append(&line[..eq], 0.0, fmt(egui::Color32::from_gray(180), &mono)); // Ключ
            job.append("=", 0.0, fmt(egui::Color32::from_gray(100), &mono)); // Равно
            
            let val = &line[eq+1..];
            let val_trimmed = val.trim_start();
            let indent_len = val.len() - val_trimmed.len();
            
            job.append(&val[..indent_len], 0.0, fmt(CLR_VALUE, &mono));
            
            // Продвинутая эвристика типов TOML
            let val_color = if val_trimmed.starts_with('"') || val_trimmed.starts_with('\'') {
                egui::Color32::from_rgb(152, 195, 121) // Green strings
            } else if val_trimmed == "true" || val_trimmed == "false" {
                egui::Color32::from_rgb(198, 120, 221) // Purple booleans
            } else if val_trimmed.chars().next().map_or(false, |c| c.is_ascii_digit() || c == '-') {
                egui::Color32::from_rgb(209, 154, 102) // Orange numbers
            } else if val_trimmed.starts_with('[') {
                egui::Color32::from_rgb(97, 175, 239) // Blue arrays
            } else {
                CLR_VALUE
            };
            
            job.append(val_trimmed, 0.0, fmt(val_color, &mono));
            job.append("\n", 0.0, egui::TextFormat::default());
        } else {
            job.append(line, 0.0, fmt(CLR_VALUE, &mono));
            job.append("\n", 0.0, egui::TextFormat::default());
        }
    }

    ui.fonts(|f| f.layout_job(job))
}

#[inline]
fn fmt(color: egui::Color32, font_id: &egui::FontId) -> egui::TextFormat {
    egui::TextFormat { color, font_id: font_id.clone(), ..Default::default() }
}