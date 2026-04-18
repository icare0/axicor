# 08. Axicor Lab:    IDE (Window Manager)

>   Axicor.  Blender-  Axicor Lab,    Bevy 0.13, `bevy_egui`  `egui_tiles`.

---

## 1.   (DOD)

Axicor Lab   **  **,    (3D-,  )    .

1. **Render-To-Texture (RTT) :**         .         `Handle<Image>` (VRAM-). `egui`    .
2. ** Overlap:**       (Z-index  ).     ().
3. ** :**       RTT-.       .

---

## 2.   (ECS Pipeline)

  UI   4     Bevy.       Rayon   .

### 2.1. `render_workspace_system` ( UI)
 `egui::CentralPanel`    . 
* **:**     ECS ( `Commands`).
* **:**      (`HashMap<TileId, egui::Rect>`)      .
* **:**  10-          (Drag-and-Drop).

2.2. evaluate_drag_intents_system ( )
    egui.   `WindowDragState`    `TopologyCache`,    .
*   **Alignment Check:**       ( 2.0 ).          (Blender-way).
*   **Clamp Protection:**     ,       100 .
*   ** :**   `TreeCommands` (Split/Merge)   .     .

2.3. execute_window_commands_system ( )
  ,    `Commands` Bevy   .
*   **VRAM Allocation:**          `rect`  `fraction`   .  `Handle<Image>`    ,   .
*   **Pure Absorption:**    (Merge)  100%  (`share`)    , ,          .

2.4. window_garbage_collector_system ()
 ,   .    `primary_released`.
*    `TopologyCache`.  `width < 100.0`  `height < 100.0`,   .
*    ECS- (`despawn_recursive`)   VRAM.
*      `egui_tiles::Tree`   `simplify()`.
*        () .

---

3.   (Blender-way Topology)

3.1. Action Zones  
      (`tab_bar_height = 0`).     10-   (`Sense::drag()`)     .
*   ** ():**      (>20px).   -.
*   ** ():**      (>20px).     .
*   **Hover:**      ( , 5% opacity).

3.2. DOD   
      .  `pane_ui`      5  (`shrink(5.0)`),   10px ,     .
 (`rounding(10.0)`)     RTT-  `egui::Image`.

---

4.     WGPU

4.1. Debounce  VRAM
           .
*   **:**  `sync_plugin_geometry_system`    `image.resize()`,     .
*      `egui`    .  VRAM     .

4.2. WGPU Panic Prevention
 WGPU       <= 0.
 `Clamp Protection`  `evaluate_drag_intents_system`  `window_garbage_collector_system` ,  3D-          100x100 .

---

## 5.     (Axicor WM)

1. **Borderless & System Drag:**     (`decorations: false`).      `Sense::drag()`  Top Bar      -    (NonSend `WinitWindows`),    UI-  - Rayon.
2. **O(1) Data-Oriented Swap:**         O(1) swap ECS-   `egui_tiles`.    VRAM    ,    .
3. **Debounce VRAM Allocations:**     WGPU    (  ). `egui`    .  `image.resize()`       ( ).
4. **Domain Switcher:**       Blender-.      ECS-      (`despawn_recursive`),    ,   .
5. **Garbage Collector:**        `egui_tiles`    `PluginWindow` in ECS.    ,         .

## 6. WebSocket Telemetry Protocol
Axicor Lab        (HFT)   WebSocket  (   9003).

**Binary Frame Packing (Little-Endian):**
         :
- `[0..4]` **Magic:** `0x4B495053` (`"SPIK"`)
- `[4..12]` **Tick:** `u64` (  )
- `[12..16]` **Count:** `u32` (  `N`   )
- `[16..16 + N*4]` **Payload:**   `u32` (Dense ID ,   ).

 IDE   ,  Magic,    `&[u32]`       (  Shader Material).

---

## 7.   WM (Global WM Systems)

7.1. Unified Context Menu
       (`OpenContextMenuEvent` -> `ContextMenuActionTriggeredEvent`),     (Z-fighting)  .
* ****:    `MenuAction`    .  WM (`context_menu_ui_system`)  `egui::Area`  `Order::Foreground`   .
* **Global Injection**:         (, "wm.create_file")    .
* **Intent Routing**:      WM  `ContextMenuActionTriggeredEvent`    `target_window`,    .

7.2. Global Cache Invalidation
 WM   .    (, , )       `layout_api::EntityDeletedEvent { path }`.
*   (project_explorer, node_editor, code_editor)  - (, evict_deleted_entities_system).
*       .starts_with(deleted_path)   ,   -    UI.

---

8. DOD  UI (Anti-Footguns)

               `egui`:

8.1. String DTO Routing (Context Menu)
   ,   `Pos2`      UI  .
*   **:**          ID : `action_id: format!("node_editor.add_env_rx|{}|{}", local_pos.x, local_pos.y)`.
*   **:**     (`interaction.rs`)  .               (Copy/Clone)  DTO-.

8.2. Focus Trap Prevention (Inline Editing)
 `egui::TextEdit`    `Enter`  `Escape`,     `ui.input()`     .
*   **:**     :
    ```rust
    if edit.lost_focus() {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            state.editing = None; //    Escape
        } else {
            send_mutation(...);   //  (Enter   )
            state.editing = None;
        }
    } else if !edit.has_focus() {
        edit.request_focus();     //     
    }
    ```
*   **:**   `edit.request_focus()`      ( `lost_focus`   , UI ).       inline-   .
