# Code Editor (Text & TOML IDE)

|  |  |
| :--- | :--- |
| **** | `PluginDomain::CodeEditor` |
| **** |     (TOML).  O(N)       . |

## 1. ECS  ()
* **`CodeEditorState` ()**:       (`current_file`)      (`content`   `String`).   `Entity`    .

## 2.   (Event API)
         :

* **Listens to:** `OpenFileEvent` ( `layout_api`). 
  *              `CodeEditorState`.
* **Listens to:** `EntityDeletedEvent` ( `layout_api`).
  *   `current_file`   `content`,            ,      .
* **Emits:** `TopologyChangedEvent` ( `layout_api`). 
  *  ****      ( `Apply`).      ,   `Node Editor`  `Connectome Viewer`      .

## 3.   (Execution Paths)

###   (Rendering & Syntax Highlighting)
    IDE,      AST-      (Regex), `code_editor`  Data-Oriented :
* **O(N) Linear Layouter:**        `egui::text::LayoutJob`.         (`.split('\n')`),        TOML ( `[ ]`,  `#`,  `=`).
* **Zero-Garbage Background:**        60 FPS                 UI-.

###   (I/O)
* **Explicit Apply:**     Debounced Auto-Save.     `TopologyChangedEvent`    .    `axicor_core`           .

## 4. DND  (Drag-and-Drop)
*    **Drop Target**.     (  `egui::Memory`  ID `dnd_path`)     `content_rect`   `Code Editor`,       ,        .
