# Node Editor (Shard Assembler)

| Свойство | Значение |
| :--- | :--- |
| **Домен** | `PluginDomain::NodeEditor` (`axicor.node_editor`) |
| **Крейт** | `axicor-lab/plugins/node_editor/` |
| **Роль** | Визуальный конструктор нейронных топологий: макро-роутинг (Модель → Департаменты → Шарды) и микро-инспекция (CAD-вьюпорт шарда с DND-проекцией связей на слои). |

---

## 1. Иерархия уровней (EditorLevel)

Навигация по топологии — трёхуровневая, управляется через хлебные крошки (`ui/breadcrumb/`).

| Уровень | Файл-контекст | Ноды | Связи |
|:---|:---|:---|:---|
| **Model** | `simulation.toml` | Департаменты (`Zone_N`) | Inter-Department Ghost Axons |
| **Department** | `{dept}.toml` | Шарды (`Shard_N`) | Inter-Shard Ghost Axons |
| **Zone (Shard)** | Тот же `{dept}.toml` | — | — (Микро-уровень вырезан в независимые плагины: `shard_cad`, `io_inspector`, `blueprint_editor`, `anatomy_slicer`) |

Переход на уровень Zone скрывает граф нод и активирует специализированные микро-плагины.

---

## 2. ECS Контракт (Данные)

### Компоненты (на Entity окна)

| Компонент | Описание |
|:---|:---|
| `NodeGraphUiState` | Полное UI-состояние окна: pan/zoom, позиции нод, буферы переименования, DND-стейт (`dragging_pin`, `pending_3d_drop`, `active_3d_hover`), флаги шторок, RTT handle для CAD Inspector |
| `ShardCadEntity` | Маркер для 3D сущностей CAD Inspector (геометрия слоёв, hover-plane, камера) |
| `CadCameraState` | Orbital camera: target, radius, alpha (yaw), beta (pitch) |
| `CadGeometryMarker` | Маркер для мешей геометрии (слои + якоря существующих связей) |
| `CadHoverPlane` | Маркер для полупрозрачной плоскости Z-привязки при DND |

### Ресурсы (глобальные)

| Ресурс | Описание |
|:---|:---|
| `BrainTopologyGraph` | Менеджер сессий. `sessions: HashMap<PathBuf, ProjectSession>`, `active_path`, `active_project` |

### ProjectSession (кэш открытого графа)

```
zones, zone_ids, env_rx_nodes, env_tx_nodes,
connections: Vec<(from, from_port, to, to_port)>,
node_inputs/outputs: HashMap<zone, Vec<port_name>>,
layout_cache: HashMap<id, (x, y)>,
shard_anatomies: HashMap<zone, ShardAnatomy>,
is_dirty: bool
```

---

## 3. Шина Событий

### Генерирует

| Событие | Когда | Потребитель |
|:---|:---|:---|
| `TopologyMutation::Create(CreateTarget)` | DND-связь, кнопка "+", double-click, контекст-меню | WM `create_entity_system` (диск + RAM) |
| `TopologyMutation::Delete(DeleteTarget)` | Контекст-меню | `mutations.rs` (RAM) + WM `delete_entity_system` (диск) |
| `TopologyMutation::Rename(RenameTarget)` | Inline TextEdit | `mutations.rs` (RAM) + WM `rename_zone_system` (диск) |
| `OpenContextMenuEvent` | ПКМ на ноде/порте/канвасе/3D-drop | WM `render_context_menu_system` |
| `SaveProjectEvent` / `CompileGraphEvent` / `BakeProjectEvent` | Кнопки хедера | IO-системы плагина |
| `OpenFileEvent` | Breadcrumb click | WM |

### Слушает

| Событие | Действие |
|:---|:---|
| `LoadGraphEvent` | Async-загрузка TOML в `ProjectSession` |
| `OpenFileEvent` | Переключение `active_path` |
| `EntityDeletedEvent` | Evict `sessions`, сброс фокуса |
| `ContextMenuActionTriggeredEvent` | Маршрутизация действий через `interaction.rs` |

---

## 4. Мутационный конвейер (Data Flow)

> [!IMPORTANT]  
> Асимметрия по дизайну: **Create** пишет RAM + диск на стороне WM (нужен UUID). **Delete/Rename** — RAM на стороне плагина, диск на стороне WM.

```
┌──────────────────────────────────────────────────────────────┐
│  UI (node.rs / panels.rs / mod.rs)                           │
│  Генерирует TopologyMutation через FnMut сигнал              │
└──────────────────────┬───────────────────────────────────────┘
                       │ EventWriter<TopologyMutation>
           ┌───────────┴───────────┐
           ▼                       ▼
┌─────────────────────┐  ┌──────────────────────────┐
│ mutations.rs (NE)   │  │ create_entity_system (WM)│
│ RAM: Delete, Rename │  │ RAM + DISK: Create       │
└─────────────────────┘  │ (wm_file_ops → sandbox)  │
                         └──────────────────────────┘
           │                       │
           ▼                       ▼
┌──────────────────────┐  ┌──────────────────────────┐
│ delete/rename_system │  │ session.is_dirty = true  │
│ (WM, DISK only)      │  │ → автосохранение layout  │
└──────────────────────┘  └──────────────────────────┘
```

---

## 5. Система Ordering (lib.rs .chain())

> [!CAUTION]  
> Порядок систем в `.chain()` критичен для DND pipeline. `render_node_editor_system` **ОБЯЗАН** идти до `dnd_raycast_system`, иначе `pending_3d_drop` теряется между кадрами.

### Chain 1: Логика + IO (холодный путь)
```
init_node_editor_windows → handle_menu_triggers
→ save → compile → bake → autosave_layout
→ apply_topology_mutations → evict_deleted_entities
→ spawn_load_task → apply_loaded_graph
```

### Chain 2: CAD Inspector + Render (горячий путь)
```
allocate_vram → sync_vram
→ spawn_cad_camera → sync_camera_aspect → cad_camera_control
→ spawn_cad_geometry
→ render_node_editor_system   ◄── UI ПИШЕТ pending_3d_drop, dragging_over_3d
→ sync_hover_plane            ◄── Визуализация плоскости (active_3d_hover из прошлого кадра)
→ dnd_raycast_system          ◄── ЧИТАЕТ pending_3d_drop → OpenContextMenuEvent
→ cleanup_cad_scene
→ clear_graph_modal
```

---

## 6. DND Pipeline (связи на уровне шарда)

### 6.1 Состояния DND (поля NodeGraphUiState)

| Поле | Тип | Назначение |
|:---|:---|:---|
| `dragging_pin` | `Option<(zone, port, pos, is_input)>` | Активный drag. Устанавливается капсулой |
| `dragging_over_3d` | `Option<Pos2>` | Локальные координаты курсора над 3D (для raycast → hover) |
| `active_3d_hover` | `Option<(Pos2, u32)>` | Результат raycast: экранная позиция snap + voxel_z |
| `pending_3d_drop` | `Option<(zone, port, screen_pos, local_pos, is_input)>` | Финальный drop, ждёт обработки raycast |

### 6.2 Покадровый flow

```
Кадр N:
  panels.rs         → dragging_pin.clone() → dragging_over_3d = Some(local_pos)
  dnd_raycast_system → dragging_over_3d → intersect_shard() → active_3d_hover = Some((snap, z))

Кадр N+1:
  panels.rs         → active_3d_hover → wire snap визуализация в draw_matrix_capsule
  (hover plane видна через sync_hover_plane_system)

Кадр R (release):
  panels.rs         → pointer.any_released() + rect_contains_pointer
                    → pending_3d_drop = Some(...), dragging_pin = None
  dnd_raycast_system → pending_3d_drop.take() → intersect_shard()
                    → OpenContextMenuEvent { "connect_matrix|zone|port|zone|in|Z" }

Кадр R+1:
  WM context_menu   → рендерит "Connect to Z-Voxel N"
  Клик              → ContextMenuActionTriggeredEvent

Кадр R+2:
  handle_menu_triggers → TopologyMutation::Create(Connection{..., voxel_z})
  create_entity_system → io.toml + ghost_capacity DCR + RAM session update
```

### 6.3 Raycast (AABB)

```rust
// raycast.rs::intersect_shard
// Классический slab intersection для AABB [-w/2..w/2, -h/2..h/2, -d/2..d/2]
// voxel_z = (hit.y + h/2).floor().clamp(0, h-1)
```

Snap-проекция: **input** → левая грань (`-w/2`), **output** → правая грань (`+w/2`). Snap-точка проецируется обратно в 2D через `camera.world_to_viewport`.

---

## 7. Hot Path: Infinite Canvas Rendering (макро-уровни)

3-Pass Render через низкоуровневый `egui::Painter`:

1. **Calc Pass** (`node::calc_all_layouts`): Локальные координаты → экранные `Rect` с учётом zoom/pan.
2. **Background Pass** (`connections::draw_all_connections`): Кубические Безье для Ghost Axons. Отрисовываются первыми → под нодами.
3. **Foreground Pass** (`node::draw_all_nodes`): Плашки зон, заголовки, пины. Регистрация `Sense::click_and_drag()` для перемещения.

### Типы нод

| Тип | Цвет хедера | Порты |
|:---|:---|:---|
| `Shard` | Тёмно-синий `(45,55,70)` | Inputs + Outputs |
| `EnvRX` (Sensor) | Зелёный `(35,65,45)` | Только Outputs |
| `EnvTX` (Motor) | Красный `(65,35,35)` | Только Inputs |

---

## 8. Декомпозиция Микро-Уровня (The Great Vivisection)

Исторически `node_editor` включал в себя рендер 3D-стекла (`cad_inspector`) и боковые шторки свойств. Это нарушало инвариант *Separation of Concerns* и превращало плагин в God Object. 

Теперь микро-уровень распилен на независимые плагины, общающиеся через Event Bus и RAM-блэкборды:
* **`shard_cad`**: Чистый 3D-рендерер анатомии шарда и обработчик DND-лучей (Raycasting).
* **`io_inspector`**: Роутер I/O матриц. Рендерит капсулы In/Out и инициирует `IoWirePayload` в `egui::Memory` при драге.
* **`blueprint_editor`**: Инспектор параметров нейронов (GLIF/STDP).
* **`anatomy_slicer`**: Управление плотностью и процентом высоты слоев.

**Связывание (Cross-Plugin DND Protocol):**
1. Пользователь тянет капсулу порта из `io_inspector`. Плагин пишет `IoWirePayload` в глобальную память `egui` и рисует кривую Безье на слое `Order::Tooltip`.
2. Курсор наводится на тайл `shard_cad`. Плагин читает блэкборд. Если видит летящий провод — делает рейкаст в стекло и подсвечивает Z-слой пересечения (Hover Plane).
3. При отпускании (Drop) `shard_cad` шлет `OpenContextMenuEvent` для завершения подключения. Плагины не имеют прямых ссылок друг на друга.

---

## 9. DCR (Dynamic Capacity Reservation)

При создании межзонной связи:
1. `create_entity_system` читает `width × height` output-матрицы источника
2. Вычисляет `ghost_capacity += w × h × 2` на целевом шарде (`shard.toml`)
3. При удалении — симметричное освобождение: `ghost_capacity -= w × h × 2`

---

## 10. Контекстное меню (pipe-delimited protocol)

Плагин шлёт `OpenContextMenuEvent` с `action_id` в формате:
```
node_editor.{action}|{param1}|{param2}|...
```

| Action ID шаблон | Параметры | Описание |
|:---|:---|:---|
| `delete_node\|{name}` | zone name | Удаление ноды |
| `start_rename\|{name}` | zone name | Запуск inline-rename |
| `start_rename_port\|{zone}\|{is_input}\|{port}` | zone, 0/1, port name | Rename порта |
| `delete_port\|{zone}\|{is_input}\|{port}` | zone, 0/1, port name | Удаление IO порта |
| `add_node\|{x}\|{y}` | canvas coords | Добавление ноды в позицию |
| `add_env_rx\|{x}\|{y}` | canvas coords | Добавление Sensor |
| `add_env_tx\|{x}\|{y}` | canvas coords | Добавление Motor |
| `connect_matrix\|{from}\|{port}\|{to}\|{to_port}\|{z}` | zones, ports, voxel_z | DND 3D проекция |
| `clear_graph` | — | Модалка очистки |

> [!WARNING]
> Символ `|` категорически запрещён в именах зон и портов, иначе парсинг через `.split('|')` сломается при DTO-роутинге интентов. Текстовые поля (TextEdit) обязаны фильтровать ввод: `rename_buffer.retain(|c| c.is_alphanumeric() || c == '_');`.
