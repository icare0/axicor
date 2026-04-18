# Brain Builder: HFT-  Offline-

 Axicor          `new Neuron()`.     (GC)    Zero-Copy   GPU. 

     **" "**.   Python- `BrainBuilder`       TOML-.   `axicor-baker` ""      C-ABI  (`.state`  `.axons`).    Axicor.

## 1.   (Pipeline Clarity)

1. **Python Script (`builder.build()`):**     TOML- (`anatomy.toml`, `blueprints.toml`, `io.toml`, `shard.toml`).
2. **Axicor Baker (CPU Compiler):** `cargo run -p axicor-baker -- --brain config/brain.toml`.  TOML,  , ""   3D- (Cone Tracing)      VRAM-.
3. **Axicor Node (GPU Runtime):** `cargo run -p axicor-node`.  `mmap`      ,  .

---

## 2.  HFT- ()

     (100+ )    Gymnasium,       .           **Reward-Gated Plasticity** (   ).

```python
from axicor.builder import BrainBuilder

# 1.  
builder = BrainBuilder(project_name="HftAgent", output_dir="Axicor-Models/")

#     HFT- (1 ms )
builder.sim_params["sync_batch_ticks"] = 10
builder.sim_params["tick_duration_us"] = 100

# 2.   (, ,   )
#     = 25 
cortex = builder.add_zone("SensoryCortex", width_vox=64, depth_vox=64, height_vox=63)

# 3.    HFT- 
#  HFT-        (pot=0)
#   (dep=2)    .    Axicor.
exc_type = builder.gnm_lib("VISp4/141").set_plasticity(pot=0, dep=2)
inh_type = builder.gnm_lib("VISp4/114").set_plasticity(pot=0, dep=2)

# 4.   (Bottom-Up )
# : height_pct       1.0!
cortex.add_layer("L4_Input", height_pct=0.1, density=0.3)\
      .add_population(exc_type, fraction=0.8)\
      .add_population(inh_type, fraction=0.2)

cortex.add_layer("L23_Hidden", height_pct=0.6, density=0.15)\
      .add_population(exc_type, fraction=0.8)\
      .add_population(inh_type, fraction=0.2)

cortex.add_layer("L5_Motor", height_pct=0.3, density=0.1)\
      .add_population(exc_type, fraction=1.0)

# 5.     (I/O Matrix)
cortex.add_input("sensors", width=8, height=8, entry_z="top")
cortex.add_output("motors", width=16, height=8, target_type="All")

# 6.  
builder.build()
```

## 3.      (DOD)

1. ** RNG   (Hard Quotas):**  `add_population(fraction=0.8)`   .     1000     density,     800     200 .     (master_seed).
2. ** :**    (`add_input`, `add_output`)   3D-  (UV-).       .     I/O   Spatial Hashing.
3. **C-ABI :** Baker   `padded_n`       ,  32 (Warp Alignment). GPU      Divergence  Cache Misses.
4. **  (Strict Dale's Law):**  Axicor    (  )        .   **    ()**.     `is_inhibitory = true`  `blueprints.toml`,        .    Baker-    .    GPU-  GSOP-    (Branchless Math),    HFT-.

## 4. Shift-Left Validation & Ergonomics

Python SDK (`BrainBuilder`)       C-ABI .      `.toml` .

### 4.1. Interactive Auto-Fix
      (`sys.stdout.isatty()`),     SDK   ,    ,          (   Enter   ).
   (CI/CD)     `ValueError`.

### 4.2. Integer Physics Validation (`v_seg`)
       .
*   **:** `v_seg = (signal_speed_m_s * 1000 * (tick_duration_us / 1000)) / (voxel_size_um * segment_length_voxels)`.  `v_seg`    .
*   **UX:**   `v_seg` SDK    `signal_speed_m_s`,  `segment_length_voxels`,    .

### 4.3. Topological Auto-Routing (MTU Fragmentation)
      (, `width=256, height=256`).
*   **:** SDK   payload': `(width * height) / 8` .
*   **:**  payload   MTU (  65507  PC, 1400  ESP32), `BrainBuilder`      `N`   ().
*   **:**       `uv_rect = [u_offset, v_offset, u_width, v_height]`,  `axicor-baker`       .

### 4.4. 4-Bit Type Limit
   (Shard)     16   ,   `type_mask`   4    `soma_flags`.   17-   `add_population`   .

## 5.     (I/O Matrix & Blueprint Wiring)
       .   -   ,     `layout`.

```python
#  8x8 (64  )
#  3     ,  61   (Padding)
cortex.add_input("sensors", width=8, height=8, entry_z="top", layout=[
    "pos_x", "pos_y", "angle_joint_0"
])

#   16x8
cortex.add_output("motors", width=16, height=8, target_type="All", layout=[
    "motor_left", "motor_right"
])
```

 layout: Rust- (Data Plane)   .   width=8, height=8     64 ,    - (The 64-Byte Alignment Rule). Python SDK (Control Plane)     io.toml    Zero-Cost ,  pos_x      memoryview,  pos_y  memoryview.

