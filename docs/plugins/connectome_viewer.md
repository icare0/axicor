# Connectome Viewer (3D)

|  |  |
| :--- | :--- |
| **** | `PluginDomain::Viewport3D` |
| **** | 3D-   (, )   FPS |

## 1. ECS  ()
* **ShardGeometry { viewport: Entity }**:  ,       .  Garbage Collector'    VRAM   .

## 2.   (Event API)
* **Listens to:** `ZoneSelectedEvent`.
> [!IMPORTANT]
>    **early-exit**,   `target_window`      `Entity`   .

## 3.   (Execution Paths)

###   (Loading)
* **Zero-Copy Geometry:**  `shard.pos`         `&[u32]`  `bytemuck::cast_slice`.
* **Dynamic AABB Centering:**       ,       `Vec3::ZERO`      .

###   (Interaction)
* ** :**     .        `PluginInput`,     RTT-.
* **Scroll Quantization:**      `.signum()`      (15%),    .
* **WGPU Safe-Start:**   `Camera`  (`is_active = false`)     VRAM-  .