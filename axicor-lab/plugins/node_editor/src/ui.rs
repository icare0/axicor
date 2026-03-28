use bevy_egui::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};

pub fn render_editor_ui(
    ui: &mut egui::Ui,
    window_rect: Rect,
    graph: &mut BrainTopologyGraph, // Теперь mut, чтобы добавлять связи!
    state: &mut NodeGraphUiState,
    mut _send_mutation: impl FnMut(TopologyMutation),
    mut _send_save: impl FnMut(),
    mut _send_bake: impl FnMut(),
    mut send_open: impl FnMut(std::path::PathBuf), // ДОБАВЛЕНО
) {
    // 1. Отрисовка кастомного хедера навигации
    let header_height = 28.0;
    let header_rect = Rect::from_min_size(window_rect.min, Vec2::new(window_rect.width(), header_height));
    let content_rect = Rect::from_min_max(Pos2::new(window_rect.min.x, window_rect.min.y + header_height), window_rect.max);

    ui.painter().rect_filled(header_rect, 0.0, Color32::from_rgb(30, 30, 30));
    ui.painter().line_segment([header_rect.left_bottom(), header_rect.right_bottom()], Stroke::new(1.0, Color32::from_rgb(50, 50, 50)));

    // DOD FIX: Используем child_ui для идеального выравнивания по центру по вертикали и явного отступа
    let mut header_ui = ui.child_ui(header_rect, egui::Layout::left_to_right(egui::Align::Center));
    header_ui.add_space(layout_api::SYS_UI_SAFE_ZONE); // ЖЕСТКИЙ СИСТЕМНЫЙ ОТСТУП СЛЕВА (41.5px)

    // Вычисляем цвета в зависимости от текущего уровня (EditorLevel)
    let is_lvl_0 = state.level == crate::domain::EditorLevel::Model;
    let is_lvl_1 = state.level == crate::domain::EditorLevel::Department;
    
    let color_model = if is_lvl_0 { Color32::WHITE } else { Color32::GRAY };
    let color_dept  = if is_lvl_1 { Color32::WHITE } else { Color32::GRAY };
    let color_zone  = if !is_lvl_0 && !is_lvl_1 { Color32::WHITE } else { Color32::GRAY };

    let current_model = graph.project_name.clone().unwrap_or_else(|| "Select Model".to_string());
    let current_dept = if is_lvl_0 { "Select Dept".to_string() } else { "brain".to_string() }; // Улучшим позже
    let current_zone = if is_lvl_0 || is_lvl_1 { "Select Zone".to_string() } else { "Zone".to_string() };

    // --- КРОШКА 1: MODELS ---
    if let Some(new_model) = draw_searchable_breadcrumb(
        &mut header_ui, 
        &current_model, 
        &mut state.model_search, 
        color_model, 
        || {
            let mut models = Vec::new();
            if let Ok(entries) = std::fs::read_dir("Genesis-Models") {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.ends_with(".axic") || e.path().is_dir() { 
                        models.push(name.replace(".axic", "").replace(" (Source)", "")); 
                    }
                }
            }
            models.sort(); models.dedup();
            (models, vec![])
        }
    ) {
        let path = std::path::PathBuf::from("Genesis-Models").join(&new_model).join("simulation.toml");
        send_open(path);
    }

    header_ui.label(egui::RichText::new("›").color(Color32::DARK_GRAY));

    // --- КРОШКА 2: DEPARTMENTS ---
    if let Some(new_dept) = draw_searchable_breadcrumb(
        &mut header_ui, 
        &current_dept, 
        &mut state.dept_search, 
        color_dept, 
        || {
            let mut local_depts = Vec::new();
            let mut global_depts = Vec::new();
            
            // DOD FIX: Честно сканируем все модели для поиска Департаментов
            if let Ok(models) = std::fs::read_dir("Genesis-Models") {
                for m in models.flatten() {
                    let m_name = m.file_name().to_string_lossy().replace(".axic", "").replace(" (Source)", "");
                    let is_local = m_name == current_model;
                    
                    if let Ok(entries) = std::fs::read_dir(m.path()) {
                        for e in entries.flatten() {
                            let name = e.file_name().to_string_lossy().to_string();
                            if name.ends_with(".toml") && name != "simulation.toml" && name != "manifest.toml" {
                                let dept_name = name.replace(".toml", "");
                                if is_local { local_depts.push(dept_name); }
                                else { global_depts.push(format!("{}/{}", m_name, dept_name)); }
                            }
                        }
                    }
                }
            }
            local_depts.sort(); global_depts.sort();
            (local_depts, global_depts)
        }
    ) {
        let parts: Vec<&str> = new_dept.split('/').collect();
        let (m_name, d_name) = if parts.len() == 2 { (parts[0], parts[1]) } else { (current_model.as_str(), new_dept.as_str()) };
        let path = std::path::PathBuf::from("Genesis-Models").join(m_name).join(format!("{}.toml", d_name));
        send_open(path);
    }

    header_ui.label(egui::RichText::new("›").color(Color32::DARK_GRAY));

    // --- КРОШКА 3: ZONES (SHARDS) ---
    if let Some(new_zone) = draw_searchable_breadcrumb(
        &mut header_ui, 
        &current_zone, 
        &mut state.zone_search, 
        color_zone, 
        || {
            let mut local_zones = Vec::new();
            let mut global_zones = Vec::new();
            
            // DOD FIX: Честно сканируем все модели для поиска Зон (Шардов)
            if let Ok(models) = std::fs::read_dir("Genesis-Models") {
                for m in models.flatten() {
                    let m_name = m.file_name().to_string_lossy().replace(".axic", "").replace(" (Source)", "");
                    let is_local = m_name == current_model;
                    
                    if let Ok(subdirs) = std::fs::read_dir(m.path()) {
                        for sub in subdirs.flatten() {
                            if sub.path().is_dir() {
                                // Признак того, что это папка шарда
                                if sub.path().join("shard.toml").exists() || sub.path().join("anatomy.toml").exists() {
                                    let z_name = sub.file_name().to_string_lossy().to_string();
                                    if is_local { local_zones.push(z_name); } 
                                    else { global_zones.push(format!("{}/{}", m_name, z_name)); }
                                }
                            }
                        }
                    }
                }
            }
            local_zones.sort(); global_zones.sort();
            (local_zones, global_zones)
        }
    ) {
        let parts: Vec<&str> = new_zone.split('/').collect();
        let (m_name, z_name) = if parts.len() == 2 { (parts[0], parts[1]) } else { (current_model.as_str(), new_zone.as_str()) };
        let path = std::path::PathBuf::from("Genesis-Models").join(m_name).join(z_name).join("shard.toml");
        send_open(path);
    }

    // Ограничиваем канвас зоной под хедером
    ui.allocate_ui_at_rect(content_rect, |ui| {
        ui.set_clip_rect(content_rect);

        // 2. Управление камерой канваса (Pan & Zoom)
        let interact_response = ui.interact(content_rect, ui.id().with("canvas_bg"), egui::Sense::click_and_drag());
        
        // DOD FIX: Восстановленное контекстное меню для создания нод
        interact_response.context_menu(|ui| {
            let is_model_level = state.level == crate::domain::EditorLevel::Model;
            let title = if is_model_level { "🏢 Create Department" } else { "🧩 Create Zone" };
            
            ui.label(egui::RichText::new(title).strong().color(Color32::LIGHT_BLUE));
            
            let resp = ui.text_edit_singleline(&mut state.new_node_buffer);
            resp.request_focus();
            
            ui.add_space(4.0);
            
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("➕ Add Node").clicked() || enter_pressed {
                let node_name = state.new_node_buffer.trim().to_string();
                if !node_name.is_empty() {
                    graph.zones.push(node_name.clone());
                    
                    if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                        let local_pos = (mouse_pos.to_vec2() - content_rect.min.to_vec2() - state.pan) / state.zoom;
                        state.node_positions.insert(node_name, local_pos.to_pos2());
                    }
                    
                    println!("TODO: Generate files for {}", state.new_node_buffer);
                    state.new_node_buffer.clear();
                    ui.close_menu();
                }
            }
            
            ui.separator();
            if ui.button("📋 Paste").clicked() { ui.close_menu(); }
        });

        if interact_response.dragged_by(egui::PointerButton::Middle) || 
           (interact_response.dragged_by(egui::PointerButton::Primary) && ui.ctx().dragged_id().is_none()) {
            state.pan += interact_response.drag_delta();
        }
        
        if ui.rect_contains_pointer(content_rect) {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let old_zoom = state.zoom;
                state.zoom = (state.zoom + scroll * 0.005).clamp(0.2, 5.0);
                if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let zoom_ratio = state.zoom / old_zoom;
                    state.pan = mouse_pos.to_vec2() - (mouse_pos.to_vec2() - state.pan) * zoom_ratio;
                }
            }
        }

        let painter = ui.painter_at(content_rect);
        let node_size = Vec2::new(160.0, 60.0) * state.zoom;
        let pin_radius = 6.0 * state.zoom;
        let to_screen = |pos: Pos2| -> Pos2 { content_rect.min + (pos.to_vec2() * state.zoom) + state.pan };

        // --- PASS 1: CALC ---
        let mut screen_rects = std::collections::HashMap::new();
        let mut input_pins = std::collections::HashMap::new();  // Целевые зоны для Drop
        let mut output_pins = std::collections::HashMap::new(); // Источники для Drag

        for (i, zone) in graph.zones.iter().enumerate() {
            let local_pos = state.node_positions.entry(zone.clone())
                .or_insert_with(|| Pos2::new(100.0 + (i as f32 * 200.0), 150.0));
            
            let screen_pos = to_screen(*local_pos);
            let node_rect = Rect::from_min_size(screen_pos, node_size);
            screen_rects.insert(zone.clone(), node_rect);

            // Координаты пинов
            input_pins.insert(zone.clone(), node_rect.left_center());
            output_pins.insert(zone.clone(), node_rect.right_center());
        }

        // --- PASS 2: BACKGROUND (Связи и Временная линия) ---
        // Рисуем существующие связи
        for (from, to) in graph.connections.clone() { // Клонируем для обхода займов
            if let (Some(&p1), Some(&p2)) = (output_pins.get(&from), input_pins.get(&to)) {
                draw_connection_line(&painter, p1, p2, state.zoom, Color32::from_rgb(200, 100, 50));

                // DOD FIX: Вычисляем середину кривой Безье для хитбокса
                let mid_point = p1 + (p2 - p1) * 0.5;
                let conn_rect = Rect::from_center_size(mid_point, Vec2::splat(15.0 * state.zoom));
                let conn_id = ui.id().with(format!("conn_{}_{}", from, to));
                let conn_resp = ui.interact(conn_rect, conn_id, egui::Sense::click());

                // Подсветка точки соединения при наведении
                if conn_resp.hovered() {
                    painter.circle_filled(mid_point, 5.0 * state.zoom, Color32::YELLOW);
                }

                // Контекстное меню связи
                conn_resp.context_menu(|ui| {
                    ui.label(format!("Link: {} → {}", from, to));
                    ui.separator();
                    if ui.button("⚙ Properties").clicked() { println!("TODO: Link Properties"); ui.close_menu(); }
                    if ui.button("✂ Delete Connection").clicked() {
                        println!("TODO: Delete {} → {}", from, to);
                        ui.close_menu();
                    }
                });
            }
        }

        // Рисуем линию, которую сейчас тянет пользователь
        if let Some((_, start_pos)) = &state.dragging_pin {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                draw_connection_line(&painter, *start_pos, mouse_pos, state.zoom, Color32::from_rgb(255, 200, 100));
            }
        }

        // --- PASS 3: FOREGROUND (Ноды и Пины) ---
        for zone in &graph.zones {
            let node_rect = screen_rects[zone];
            
            // Тело ноды
            painter.rect_filled(node_rect, 6.0 * state.zoom, Color32::from_rgb(45, 45, 50));
            painter.rect_stroke(node_rect, 6.0 * state.zoom, Stroke::new(1.5 * state.zoom, Color32::from_rgb(100, 150, 200)));
            painter.text(node_rect.center(), egui::Align2::CENTER_CENTER, zone, egui::FontId::proportional(16.0 * state.zoom), Color32::WHITE);

            // Интерактивность тела (перемещение)
            let node_id = ui.id().with(zone);
            let node_response = ui.interact(node_rect, node_id, egui::Sense::drag());

            // Контекстное меню Ноды
            node_response.context_menu(|ui| {
                ui.label(format!("Node: {}", zone));
                ui.separator();
                if ui.button("⚙ Properties").clicked() { println!("TODO: Properties"); ui.close_menu(); }
                if ui.button("🗑 Delete Node").clicked() { println!("TODO: Delete"); ui.close_menu(); }
            });

            if node_response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = state.node_positions.get_mut(zone) {
                    *pos += node_response.drag_delta() / state.zoom;
                }
            }

            // Отрисовка и логика OUTPUT PIN (Справа)
            let out_pin_pos = output_pins[zone];
            let out_pin_rect = Rect::from_center_size(out_pin_pos, Vec2::splat(pin_radius * 3.0));
            painter.circle_filled(out_pin_pos, pin_radius, Color32::from_rgb(255, 150, 50));
            
            let out_response = ui.interact(out_pin_rect, node_id.with("out"), egui::Sense::drag());
            if out_response.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair); }
            
            // Контекстное меню Output Пина
            out_response.context_menu(|ui| {
                if ui.button("🔌 Disconnect All Targets").clicked() { ui.close_menu(); }
            });

            if out_response.drag_started() {
                state.dragging_pin = Some((zone.clone(), out_pin_pos));
            }

            // Отрисовка и логика INPUT PIN (Слева)
            let in_pin_pos = input_pins[zone];
            let in_pin_rect = Rect::from_center_size(in_pin_pos, Vec2::splat(pin_radius * 3.0));
            painter.circle_filled(in_pin_pos, pin_radius, Color32::from_rgb(100, 200, 255));

            let in_response = ui.interact(in_pin_rect, node_id.with("in"), egui::Sense::click());
            in_response.context_menu(|ui| {
                if ui.button("🔌 Disconnect All Sources").clicked() { ui.close_menu(); }
            });
            
            // Если мы отпустили связь над этим Input пином
            if ui.rect_contains_pointer(in_pin_rect) && ui.input(|i| i.pointer.any_released()) {
                if let Some((src_zone, _)) = &state.dragging_pin {
                    if src_zone != zone {
                        // Создаем связь! (Пока просто в кэше UI)
                        graph.connections.push((src_zone.clone(), zone.clone()));
                        println!("🔗 Connected: {} -> {}", src_zone, zone);
                    }
                }
            }
        }

        // Сброс состояния протягивания
        if ui.input(|i| i.pointer.any_released()) {
            state.dragging_pin = None;
        }
    });
}

fn draw_searchable_breadcrumb(
    ui: &mut egui::Ui,
    current_name: &str,
    search_buffer: &mut String,
    active_color: Color32,
    fetch_items: impl FnOnce() -> (Vec<String>, Vec<String>), // (Local, Global)
) -> Option<String> {
    let mut selected = None;
    
    ui.menu_button(egui::RichText::new(current_name).strong().color(active_color), |ui| {
        let resp = ui.text_edit_singleline(search_buffer);
        resp.request_focus(); // Автофокус на поиске
        ui.separator();

        let query = search_buffer.to_lowercase();
        // Лениво читаем директории ТОЛЬКО если меню открыто (Zero CPU cost в фоне)
        let (local_items, global_items) = fetch_items(); 

        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            let mut has_locals = false;
            // 1. Local Scope (Текущий уровень)
            for item in local_items {
                if item.to_lowercase().contains(&query) {
                    if ui.button(egui::RichText::new(&item).color(Color32::LIGHT_BLUE)).clicked() {
                        selected = Some(item);
                        ui.close_menu();
                    }
                    has_locals = true;
                }
            }

            // 2. Global Scope (Чужие уровни)
            if !global_items.is_empty() {
                if has_locals { ui.separator(); }
                ui.label(egui::RichText::new("Global").italics().color(Color32::DARK_GRAY));
                
                for item in global_items {
                    if item.to_lowercase().contains(&query) {
                        if ui.button(&item).clicked() {
                            selected = Some(item);
                            ui.close_menu();
                        }
                    }
                }
            }
        });
    });
    
    selected
}

// Хелпер для отрисовки красивых кубических кривых
fn draw_connection_line(painter: &egui::Painter, p1: Pos2, p2: Pos2, zoom: f32, color: Color32) {
    let control_scale = (p2.x - p1.x).abs().max(50.0) * 0.5;
    let shape = egui::epaint::CubicBezierShape::from_points_stroke(
        [p1, p1 + Vec2::X * control_scale, p2 - Vec2::X * control_scale, p2],
        false, Color32::TRANSPARENT, Stroke::new(3.0 * zoom, color),
    );
    painter.add(shape);
}
