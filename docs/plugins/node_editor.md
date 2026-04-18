# Node Editor (Shard Assembler)

|  |  |
| :--- | :--- |
| **** | `PluginDomain::NodeEditor` (`axicor.node_editor`) |
| **** | `axicor-lab/plugins/node_editor/` |
| **** |    : - (    )  - (CAD-   DND-   ). |

---

## 1.   (EditorLevel)

    ,     (`ui/breadcrumb/`).

|  | - |  |  |
|:---|:---|:---|:---|
| **Model** | `simulation.toml` |  (`Zone_N`) | Inter-Department Ghost Axons |
| **Department** | `{dept}.toml` |  (`Shard_N`) | Inter-Shard Ghost Axons |
| **Zone (Shard)** |   `{dept}.toml` |  |  (-    : `shard_cad`, `io_inspector`, `blueprint_editor`, `anatomy_slicer`) |

   Zone       -.

---

## 2. ECS  ()

###  ( Entity )

|  |  |
|:---|:---|
| `NodeGraphUiState` |  UI- : pan/zoom,  ,  , DND- (`dragging_pin`, `pending_3d_drop`, `active_3d_hover`),  , RTT handle  CAD Inspector |
| `ShardCadEntity` |   3D  CAD Inspector ( , hover-plane, ) |
| `CadCameraState` | Orbital camera: target, radius, alpha (yaw), beta (pitch) |
| `CadGeometryMarker` |     ( +   ) |
| `CadHoverPlane` |     Z-  DND |

###  ()

|  |  |
|:---|:---|
| `BrainTopologyGraph` |  . `sessions: HashMap<PathBuf, ProjectSession>`, `active_path`, `active_project` |

### ProjectSession (  )

```
zones, zone_ids, env_rx_nodes, env_tx_nodes,
connections: Vec<(from, from_port, to, to_port)>,
node_inputs/outputs: HashMap<zone, Vec<port_name>>,
layout_cache: HashMap<id, (x, y)>,
shard_anatomies: HashMap<zone, ShardAnatomy>,
is_dirty: bool
```

---

## 3.  

### 

|  |  |  |
|:---|:---|:---|
| `TopologyMutation::Create(CreateTarget)` | DND-,  "+", double-click, - | WM `create_entity_system` ( + RAM) |
| `TopologyMutation::Delete(DeleteTarget)` | - | `mutations.rs` (RAM) + WM `delete_entity_system` () |
| `TopologyMutation::Rename(RenameTarget)` | Inline TextEdit | `mutations.rs` (RAM) + WM `rename_zone_system` () |
| `OpenContextMenuEvent` |   ///3D-drop | WM `render_context_menu_system` |
| `SaveProjectEvent` / `CompileGraphEvent` / `BakeProjectEvent` |   | IO-  |
| `OpenFileEvent` | Breadcrumb click | WM |

### 

|  |  |
|:---|:---|
| `LoadGraphEvent` | Async- TOML  `ProjectSession` |
| `OpenFileEvent` |  `active_path` |
| `EntityDeletedEvent` | Evict `sessions`,   |
| `ContextMenuActionTriggeredEvent` |    `interaction.rs` |

---

## 4.   (Data Flow)

> [!IMPORTANT]  
>   : **Create**  RAM +    WM ( UUID). **Delete/Rename**  RAM   ,    WM.

```
+--------------------------------------------------------------+
|  UI (node.rs / panels.rs / mod.rs)                           |
|   TopologyMutation  FnMut               |
+----------------------+---------------------------------------+
                       | EventWriter<TopologyMutation>
           +-----------+-----------+
                                  
+---------------------+  +--------------------------+
| mutations.rs (NE)   |  | create_entity_system (WM)|
| RAM: Delete, Rename |  | RAM + DISK: Create       |
+---------------------+  | (wm_file_ops  sandbox)  |
                         +--------------------------+
           |                       |
                                  
+----------------------+  +--------------------------+
| delete/rename_system |  | session.is_dirty = true  |
| (WM, DISK only)      |  |   layout  |
+----------------------+  +--------------------------+
```

---

## 5.  Ordering (lib.rs .chain())

> [!CAUTION]  
>    `.chain()`   DND pipeline. `render_node_editor_system` ****   `dnd_raycast_system`,  `pending_3d_drop`   .

### Chain 1:  + IO ( )
```
init_node_editor_windows  handle_menu_triggers
 save  compile  bake  autosave_layout
 apply_topology_mutations  evict_deleted_entities
 spawn_load_task  apply_loaded_graph
```

### Chain 2: CAD Inspector + Render ( )
```
allocate_vram  sync_vram
 spawn_cad_camera  sync_camera_aspect  cad_camera_control
 spawn_cad_geometry
 render_node_editor_system   -- UI  pending_3d_drop, dragging_over_3d
 sync_hover_plane            --   (active_3d_hover   )
 dnd_raycast_system          --  pending_3d_drop  OpenContextMenuEvent
 cleanup_cad_scene
 clear_graph_modal
```

---

## 6. DND Pipeline (   )

### 6.1  DND ( NodeGraphUiState)

|  |  |  |
|:---|:---|:---|
| `dragging_pin` | `Option<(zone, port, pos, is_input)>` |  drag.   |
| `dragging_over_3d` | `Option<Pos2>` |     3D ( raycast  hover) |
| `active_3d_hover` | `Option<(Pos2, u32)>` |  raycast:   snap + voxel_z |
| `pending_3d_drop` | `Option<(zone, port, screen_pos, local_pos, is_input)>` |  drop,   raycast |

### 6.2  flow

```
 N:
  panels.rs          dragging_pin.clone()  dragging_over_3d = Some(local_pos)
  dnd_raycast_system  dragging_over_3d  intersect_shard()  active_3d_hover = Some((snap, z))

 N+1:
  panels.rs          active_3d_hover  wire snap   draw_matrix_capsule
  (hover plane   sync_hover_plane_system)

 R (release):
  panels.rs          pointer.any_released() + rect_contains_pointer
                     pending_3d_drop = Some(...), dragging_pin = None
  dnd_raycast_system  pending_3d_drop.take()  intersect_shard()
                     OpenContextMenuEvent { "connect_matrix|zone|port|zone|in|Z" }

 R+1:
  WM context_menu     "Connect to Z-Voxel N"
                 ContextMenuActionTriggeredEvent

 R+2:
  handle_menu_triggers  TopologyMutation::Create(Connection{..., voxel_z})
  create_entity_system  io.toml + ghost_capacity DCR + RAM session update
```

### 6.3 Raycast (AABB)

```rust
// raycast.rs::intersect_shard
//  slab intersection  AABB [-w/2..w/2, -h/2..h/2, -d/2..d/2]
// voxel_z = (hit.y + h/2).floor().clamp(0, h-1)
```

Snap-: **input**    (`-w/2`), **output**    (`+w/2`). Snap-    2D  `camera.world_to_viewport`.

---

## 7. Hot Path: Infinite Canvas Rendering (-)

3-Pass Render   `egui::Painter`:

1. **Calc Pass** (`node::calc_all_layouts`):     `Rect`   zoom/pan.
2. **Background Pass** (`connections::draw_all_connections`):    Ghost Axons.     .
3. **Foreground Pass** (`node::draw_all_nodes`):  , , .  `Sense::click_and_drag()`  .

###  

|  |   |  |
|:---|:---|:---|
| `Shard` | - `(45,55,70)` | Inputs + Outputs |
| `EnvRX` (Sensor) |  `(35,65,45)` |  Outputs |
| `EnvTX` (Motor) |  `(65,35,35)` |  Inputs |

---

## 8.  - (The Great Vivisection)

 `node_editor`     3D- (`cad_inspector`)    .    *Separation of Concerns*     God Object. 

 -    ,   Event Bus  RAM-:
* **`shard_cad`**:  3D-     DND- (Raycasting).
* **`io_inspector`**:  I/O .   In/Out   `IoWirePayload`  `egui::Memory`  .
* **`blueprint_editor`**:    (GLIF/STDP).
* **`anatomy_slicer`**:      .

** (Cross-Plugin DND Protocol):**
1.      `io_inspector`.   `IoWirePayload`    `egui`       `Order::Tooltip`.
2.     `shard_cad`.   .            Z-  (Hover Plane).
3.   (Drop) `shard_cad`  `OpenContextMenuEvent`   .        .

---

## 9. DCR (Dynamic Capacity Reservation)

   :
1. `create_entity_system`  `width  height` output- 
2.  `ghost_capacity += w  h  2`    (`shard.toml`)
3.     : `ghost_capacity -= w  h  2`

---

## 10.   (pipe-delimited protocol)

  `OpenContextMenuEvent`  `action_id`  :
```
node_editor.{action}|{param1}|{param2}|...
```

| Action ID  |  |  |
|:---|:---|:---|
| `delete_node\|{name}` | zone name |   |
| `start_rename\|{name}` | zone name |  inline-rename |
| `start_rename_port\|{zone}\|{is_input}\|{port}` | zone, 0/1, port name | Rename  |
| `delete_port\|{zone}\|{is_input}\|{port}` | zone, 0/1, port name |  IO  |
| `add_node\|{x}\|{y}` | canvas coords |     |
| `add_env_rx\|{x}\|{y}` | canvas coords |  Sensor |
| `add_env_tx\|{x}\|{y}` | canvas coords |  Motor |
| `connect_matrix\|{from}\|{port}\|{to}\|{to_port}\|{z}` | zones, ports, voxel_z | DND 3D  |
| `clear_graph` |  |   |

> [!WARNING]
>  `|`       ,    `.split('|')`   DTO- .   (TextEdit)   : `rename_buffer.retain(|c| c.is_alphanumeric() || c == '_');`.
