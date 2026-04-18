|  |  |
| :--- | :--- |
| **** | `PluginDomain::ProjectExplorer` |
| **** |  ,   (Intents)      Drag-and-Drop (DND). |

## 1. ECS  ()
* **`ProjectFsCache` ()**:     . ** **  `std::fs::read_dir`    .
* **`ProjectExplorerState` ()**:   .  `active_file: Option<PathBuf>`   " " (Smart Focus)          .  UI   .

## 2.   (Event API)
      (Intent Emitter),      :

|  |  |   |   |
| :--- | :--- | :--- | :--- |
| `ZoneSelectedEvent` |   .pos / .state | `zone_hash`, `target_window` | Connectome Viewer (3D) |
| `LoadGraphEvent` |   .axic  | `project_name` | Node Editor |
| `OpenFileEvent` |   .toml  | `path: PathBuf` | Code Editor () & Node Editor () |
| `EntityDeletedEvent` |   WM | `path` (Listens) |   `active_file`,   . |
| `OpenContextMenuEvent` |   / | `actions`, `target_window` |   /  WM. |

> **[DOD Invariant]**:  `OpenFileEvent`   **Multi-Cast**.               ()   `brain.toml`.

## 3. DND  (Drag-and-Drop)
*    **Drag Source**.
*     (drag),          UI: `ui.memory_mut(|m| m.data.insert_temp(Id::new("dnd_path"), file_path))`.
* **Zero-Leak Guarantee**:    (`any_released`)           `memory`,     (Dangling Payloads)   .

## 4.   (Execution Paths)
* **Cold Path:**           (Bundles)    (Sources).
* **UI Path:**     `egui::ScrollArea`.     (200-300px)   Layout-,        