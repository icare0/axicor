#     (Axicor Lab)

**:**    `axicor-lab/plugins/*`
**:**  .       PR  .

Axicor Lab     UI-.     (WM),   Bevy ECS.    Data-Oriented Design (DOD)   .   (`layout-api`)   ,     .

## 1.   

|  |  |
| :--- | :--- |
| **Cross-Plugin Blackboard** |       .    (, Drag-and-Drop   `io_inspector`  `shard_cad`)    `egui` (`ui.memory_mut(|m| m.data.insert_temp(...))`)   DTO-. |
| **  (SSoT)** |   ,          : `layout_api::AVAILABLE_PLUGINS`. UI-    ,    UI . |
| **RTT-** |  ** **     .         VRAM- (`Handle<Image>`),  WM    `egui`. |
| ** ** |         (`Res<Input<MouseButton>>`).        `PluginInput`   . |
| **  (GC)** |         `egui_tiles`.    WM  `despawn_recursive()`   ,   ,   VRAM-. |
| **  ** |        `EventWriter/Reader`.  - (*Intent*)    `target_window: Entity`    . |
| **Sandbox Overlay FS** |  ** **      .      RAM (`ProjectSession`).       WM   (`.Sandbox/.tmp.autosave/`).          Compile. |

## 2.   ( )

        (Hardware Sympathy).   .

*   **src/lib.rs ()**:   .   (`app.add_systems`),   . -  .
*   **src/domain.rs ()**:  DOD-.  ,   .  `impl`     .
*   **src/systems/ ()**:    :
    *   `camera.rs`:  RTT-   .
    *   `geometry.rs` (** **):   .  VFS, ,  3D-.
    *   `interaction.rs` (** **):  HFT- (, raycasting).   ,    L1.
    *   `render.rs`:      VRAM  UI- `egui`.

### 2.1.     (  )
      (Hot Path)   ,        :
*   **  I/O  (Cold Path):**  ,  ,  (VFS),  TOML/JSON,     , ****     (, `systems/io.rs`, `systems/sync.rs`).
*   **  :**      ,     (, `systems/io/mod.rs`   `systems/io/save.rs`, `systems/io/compile.rs`).
*   **`interaction.rs`   Hot Path:**  `interaction.rs`   ****  :  , ,    UI-.  I/O         .

## 3.  ECS- (DOD-)

1.  ** **:       (`ResMut<T>`)   .   (, , )    ,   `Entity` .    N     .
2.  **  (Hot Path)**: ,   `Update`,   `#[derive(Copy, Clone)]`.  heap- (`String`, `Vec`, `Box`)  ,   ,   .
3.  **  (Early-Exit)**:      O(1).  `target_window`     `Entity`      `continue`.   ,         .
4.  **   UI (No Closures):**    `EventWriter`    UI (`ui/`). UI-     (bool, enum). ECS-  `systems/`          Intent-.
5. **   (Z-Index Bleed Protection):**      `egui::Window`,       (    )     Root.       `egui::Area` + `egui::Frame::none()`    `ui.set_clip_rect(window_rect)`.      ECS- (, `systems/modals.rs`).
6. **  CRUD- (DTO-):**      ECS-     (, `create_shard_system`, `create_io_system`).      DTO (, `CreateTarget`, `DeleteTarget`)    - (, `create_entity_system.rs`).   Intent       .          .
7. **Z-Index Bleed  Focus Theft (  ):**    `egui::Area`   `Order::Middle`,  100% .   WM (,  DND)     `Order::Tooltip`  `Order::Foreground`. 
8. ** Hit-Test  WM:**       `ui.interact()`   ,    (-  ). WM      (`ui.ctx().input(|i| i.pointer.latest_pos())`)     `rect.contains(p)`.       (clipping)     VRAM : `ui.ctx().layer_painter(...)`.

## 4.   

```rust
// domain.rs
#[derive(Component, Clone, Copy)]
pub struct ViewportHotState {
    pub active_zone_hash: u32,
    pub is_dirty: bool,
}

#[derive(Event)]
pub struct ZoneSelectedEvent {
    pub zone_hash: u32,
    pub target_window: Entity, //  
}

// systems/interaction.rs
pub fn handle_zone_selection(
    mut events: EventReader<ZoneSelectedEvent>,
    mut query: Query<(&PluginWindow, &mut ViewportHotState)>,
) {
    for ev in events.read() {
        // Early-exit  :   
        let Ok((_window, mut state)) = query.get_mut(ev.target_window) else {
            continue;
        };

        state.active_zone_hash = ev.zone_hash;
        state.is_dirty = true;
    }
}
```

## 5. Sandbox Overlay FS ( )

> ** .**        :  ,         .

### 5.1.   

|  |   |   |  |
|:---|:---|:---|:---|
| **RAM** | `ProjectSession` ( `zones`, `connections`, `node_inputs/outputs`, `shard_anatomies`, ...) |  (`mutations.rs`)  WM (`create_entity_system`) |    UI- |
| **Sandbox ( )** | `{project}/.Sandbox/.tmp.autosave/{relative_path}` | ** WM**  `wm_file_ops::save_document()` |    () |
| **Cold Files ()** | `{project}/{relative_path}` ( `.toml`) | ** Compile** |     |

### 5.2.  Data Flow

```
+------------------------------------------------------------------+
|           1. UI ACTION (click, DND, rename, delete)              |
|                     TopologyMutation event                      |
+--------------------------+---------------------------------------+
                           |
           +---------------+----------------+
                                           
+----------------------+       +------------------------------+
| mutations.rs (NE)    |       | create/delete_entity (WM)    |
|  RAM:         |       |  RAM + DISK:          |
| session.zones,       |       | 1. RAM (session.*)           |
| connections, etc.    |       | 2. wm_file_ops              |
| is_dirty = true      |       |    resolve_sandbox_path()   |
+----------------------+       |    .Sandbox/.tmp.autosave/   |
                               +------------------------------+
           |                                |
                                           
+----------------------------------------------------------------------+
| 2. : overlay_read_to_string(cold_path)                         |
|    Sandbox ?   sandbox. ?   cold.         |
|          .              |
+----------------------------------------------------------------------+
                           |
                              [ Compile]
+----------------------------------------------------------------------+
| 3. COMPILE ( )                                   |
|    3.0  rm -rf .tmp.old_backup                                       |
|    3.1  mv .tmp.last_backup  .tmp.old_backup   ( )    |
|    3.2  cp -r .tmp.autosave/*  cold files       (apply overlay)     |
|    3.3  mv .tmp.autosave  .tmp.last_backup      ( sandbox)   |
|    3.4  session.is_dirty = false                                     |
+----------------------------------------------------------------------+
```

### 5.3. API (layout-api)

|  |  |
|:---|:---|
| `resolve_sandbox_path(cold_path)` | `{base}/.Sandbox/.tmp.autosave/{relative}`      |
| `overlay_read_to_string(cold_path)` |    sandbox, fallback  cold |
| `wm_file_ops::load_document(path)` | `overlay_read_to_string` + `parse::<DocumentMut>` |
| `wm_file_ops::save_document(path, doc)` | `resolve_sandbox_path` + `fs::write`  sandbox |

### 5.4.  

```
Axicor-Models/MyProject/
+-- simulation.toml               COLD ()
+-- Zone_0.toml                   COLD
+-- Zone_0/
|   +-- Shard_0/
|       +-- shard.toml            COLD
|       +-- anatomy.toml          COLD
|       +-- io.toml               COLD
+-- .Sandbox/
    +-- .tmp.autosave/              (overlay)
    |   +-- simulation.toml        
    |   +-- Zone_0/Shard_0/
    |       +-- io.toml               
    +-- .tmp.last_backup/          
    +-- .tmp.old_backup/           
```

### 5.5.  

| [ERROR]  | [OK]  |
|:---|:---|
| `fs::write("simulation.toml", ...)`   | `wm_file_ops::save_document()`  WM |
| `fs::read_to_string(path)`    | `overlay_read_to_string(path)` |
|   TOML  UI- | `TopologyMutation`  WM-  `save_document` |
|  `compile`   `is_dirty` | Compile  ****     |

## 6. Multi-Workspace Layout ( )

  , WM     (Workspaces).
* **RAM :** `WorkspaceState`  `HashMap<String, egui_tiles::Tree>`    .      .       (`is_visible = false`).
* **Data-Driven Persistence:**       DTO `SavedLayout`   `config/default_layout.ron`. WM   "" :  RON,  `Tree`,  .
