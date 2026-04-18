# 02.  (Configuration)

>   [Axicor](../../README.md).  ,    TOML-.

---

## 1.   (Human Readable)

     . ,      ,    .

**       **

### 1.1.   (`simulation.toml`)

  .  ,  **  **  ,      .

- `tick_duration_us` -   ( ).
- `voxel_size_um` -   ( ).
- `signal_speed_um_tick` -   .
- `sync_batch_ticks` -       .
- `master_seed` -   (`String`,   `u64`  )   .
- `max_dendrites` - - (128).

**  :**   `const`  (Uniforms  CUDA).   ,    L1.

### 1.2.   ( `Axicor_Models/{project}/zones/V1/`)
       .     Runtime      .
, ****  .

1. **`anatomy.toml`** -  (L1L6)   .    (`world.height`),   .
   -  V1:  ,    %,  .
   -   :   `"Nuclear"`  100%.
   -   (4- : Geo  Sign  Variant)     .
2. **`blueprints.toml`** -  :   (`threshold`, `leak`)    ().
   -  GSOP (`74`, `-2`, `64000`).
   -       4- .
3. **`io.toml`** -  /:
   -    (External Axons   L4).
   -    (  L5/L6  Output Ports).
   - ,       .

### 1.3.   (`runtime/shard_04.toml`  CLI )

> [!NOTE]
> **[Planned: Macro-3D]**  DTO    3D .
//  `InstanceConfig` (`axicor-core/src/config/instance.rs`):
// 1.  `world_offset` (x, y, z)   `u64`.
// 2.  `Neighbors`  `z_plus: Option<String>`  `z_minus: Option<String>`.

, ****  ** **   .

- `zone_id` - `"V1"` (   ).
- `world_offset` - `{ x: 1000, y: 0, z: 0 }` (     ).
- `dimensions` - `{ w: 500, h: 2000 }` (  ).
- `neighbors` -   (`IP:Port`  Shared Memory ID)   (`X+`, `X-`, `Y+`, `Y-`).  **Self-loop** (`= Self`)       (. [06_distributed.md 1.5](./06_distributed.md)).

#### 1.3.1.  `[settings]` (Runtime Optimization)

,     ,     .

- `save_checkpoints_interval_ticks` -  ( )     (`checkpoint.state` / `.axons`).
  - ** :** `100 000` (   2000+ TPS).
  - **:**        SSD (I/O).
  - **:**      `interval_ticks / batch_size`.
- `ghost_capacity` (`u32`) -  VRAM    (Ghost Axons).
  - **:**   (Python SDK)        `SUM(width * height) * 2.0`.
  - **:**             .  `200000`   .

---

## 2. :  (Baking)

**:  (Runtime)    TOML  .**

    GPU -  .    (Baking),     (TOML)    (Binary Blobs).     `Axicor_Models/`.

### 2.1. 

```
TOML  -- [Compiler Tool (CPU)] --   -- [Runtime (GPU)]
                  |                          |
                  +-  TOML             +- .axons ()
                  +-        +- .state ( )
                  +-             +- .dendrites ( )
                  +-   SoA
```

- **Compiler Tool:**   CPU -  TOML,  ,   (Cone Tracing),     .
- **Zero-Copy Loading:**   `mmap` (   )   `cudaMemcpy`    VRAM.    .

### 2.2.  (Alignment)

-     **--** ,    .
-     28  -    **32 **  Coalesced Access.
-     =   SSD.

### 2.3.   (Baking Tool Asserts)

  `simulation.toml` Baking Tool      :

```rust
// baking_compiler/src/physics_constants.rs

let segment_length_um = config.voxel_size_um * config.segment_length_voxels;
let v_seg = config.signal_speed_um_tick / segment_length_um;

// Runtime assert: v_seg    (01_foundations.md 1.6)
assert!(
    config.signal_speed_um_tick % segment_length_um == 0,
    "CRITICAL: signal_speed_um_tick must be a multiple of segment_length_um!"
);
```

### 2.4.  `.state`: ShardStateSoA

  `.state` -   .  **--**    VRAM.

```rust
// baking_compiler/src/layout.rs

/// Warp Alignment:   32
pub const fn align_to_warp(count: usize) -> usize {
    (count + 31) & !31
}

pub struct ShardStateSoA {
    pub padded_n: usize,       //  (aligned to 32)
    pub total_axons: usize,    // Local + Ghost + Virtual (aligned to 32)

    // Soma arrays (size = padded_n)
    pub voltage: Vec<i32>,           // 4 bytes  N
    pub flags: Vec<u8>,              // 1 byte  N (upper nibble: Type, bit 0: Is_Spiking)
    pub threshold_offset: Vec<i32>,  // 4 bytes  N
    pub refractory_counter: Vec<u8>, // 1 byte  N
    pub soma_to_axon: Vec<u32>,      // 4 bytes  N ( Soma  Local Axon ID)

    // Dendrite arrays - Columnar Layout (size = 128  padded_n)
    //   CUDA: data[slot * padded_n + tid]
    pub dendrite_targets: Vec<u32>,  // 4 bytes  128  N (Packed: 22b Axon_ID | 10b Segment_Index)
    pub dendrite_weights: Vec<i16>,  // 2 bytes  128  N (Signed:  = E/I)
    pub dendrite_timers: Vec<u8>,    // 1 byte  128  N

    // Axon arrays (size = total_axons, NOT padded_n!)
    pub axon_heads: Vec<u32>,        // 4 bytes  A (PropagateAxons: += v_seg)
}
```

**Seed- :** PackedPosition (`u32`)   `entity_id`  :

```rust
pub fn entity_seed(master_seed: u64, packed_pos: u32) -> u64 {
    wyhash::wyhash(&packed_pos.to_le_bytes(), master_seed)
}
```

### 2.5.   (Night Baking)

Baking -   .   (. [07_gpu_runtime.md](./07_gpu_runtime.md)) Compiler Tool       (sprouting, nudging, ),     `.axons`   .      `Axicor_Models/{project}/baked/{zone}/`.

---

## 3.   : SoA (Structure of Arrays)

**      GPU.**   ,   .

### 3.1. : AoS (Array of Structures)

```
//  (Cache Miss)
struct Neuron { pos: Vec3, voltage: f32, type_id: u8 }
neurons: [Neuron; N]
```

  `voltage`  , GPU    `pos`  `type_id` - ****,   .   -  15%.

### 3.2. : SoA

```
//  (Cache Hit, Coalesced Access)
all_voltages:  [f32; N]    //  
all_types:     [u8; N]     //  
all_positions: [Vec3; N]   //  
```

 GPU (32 )  32   `f32`  **  **.   - = 100%.

### 3.3.   (Columnar Layout)

    : 128   .

- **** `Neuron.Dendrites[0..127]` ().
- **** `Column[Slot_K]` - K-   ****   ().

```
Slot_0:   [_0.0, _1.0, _2.0, ...]   //   N
Slot_1:   [_0.1, _1.1, _2.1, ...]   //   N
...
Slot_127: [_0.127, _1.127, ...]              //   N
```

  `for slot in 0..128`   GPU   `Slot_K` -   . Bandwidth   100%.

---

## 4. : `simulation.toml`

 ,    .

```toml
[world]
#    ( )
width_um = 1000
depth_um = 1000
height_um = 2500

[model_id_v1]
id = "1cf8a9c1-37ef-4f6d-8d7c-dd911257fb91"

[[department]]
name = "Zone_0"
config = "Zone_0.toml"
depart_id_v1 = { id = "c1cc_3c4c" }

[[department]]
name = "Zone_1"
config = "Zone_1.toml"
depart_id_v1 = { id = "c1cc_e91e" }

[simulation]
# ---   (   ) ---

#  
tick_duration_us = 100  # 1  = 100  (0.1 ).    GSOP (01_foundations.md 1.4).

#  
voxel_size_um = 25       #    (01_foundations.md 1.1).
                         # 25 m -        .

#  
segment_length_voxels = 2   # 1  = 2  = 50 . : 8  (01_foundations.md 1.2).
signal_speed_um_tick = 50   #       (01_foundations.md 1.5).
                            #  : v_seg = signal_speed_um_tick  (voxel_size_um  segment_length_voxels) = 50  50 = 1 

# 
sync_batch_ticks = 100      #      (100   100  = 10 ).

#   
total_ticks = 0             # 0 =   (  )
master_seed = "GENESIS"     # String    u64   (01_foundations.md 2.1).
                            #    .   : "GENESIS", "DEBUG_RUN_42", ..

# [DOD FIX] global_density . 
#     per-layer  anatomy.toml (bottom-up ).

#   (Baking/Sprouting)
axon_growth_max_steps = 500     #   Cone Tracing   (500   50  = 25 )
```

### 4.1.    (Memory Optimization)

  ,    / ([07_gpu_runtime.md](./07_gpu_runtime.md)):

|  |  |  |  |
|---|---|---|---|
| **Global Clock** | `u64` | `u32` |   ~5    (  100 ).    . |
| **Local Timers: Refractory** (soma, synapse) | `u64` | `u8` | Max 255  = 25 .    . |
| **Local Timers: Delays** (Delay_Counter) | `u64` | `u16` | 65 535  = 6.5 .   (Projectors). |
| **Batch Size** | `u64` | `u32` |        4  . |

### 4.2.   (Immutable)

,         :

|  |  |  |  |
|---|---|---|---|
| `tick_duration_us` | `u32` | `100` | 0.1  |
| `voxel_size_um` | `f32` | `25.0` |   |
| `signal_speed_um_tick` | `u16` | `50` |   . `int`   . |
| `max_dendrites` | `const` | `128` | Hard Constraint.     128  (- + ). |
| `master_seed` | `String` | `"GENESIS"` |   `u64`   (. [01_foundations.md 2](./01_foundations.md)). |

### 4.3.   (Warp Alignment)

  Compiler/Baking Tool (2):

- **Padding Rule:**        (`.state`)     , ** 32**.
- **:**    `UpdateNeurons<<<N/32, 32>>>`      ,     Coalesced.        ,     .

---

## 5. : `anatomy.toml`

   : , , .

### 5.1.   (Relative Height)

-      (`height_pct`)  `0.0`  `1.0`.
- **:** `L1 + L2 + ... + L6 = 1.0`.
- **:**      (   2   3 )  `world.height`,    .

### 5.2.   (Absolute Layer Density)

     :

1. **`density`** ( `anatomy.toml`):        .      .
   - : L4  `density = 0.08` (8% ), L5  `density = 0.02` (2% ).

### 5.3.   (Hard Quotas)

> **[!IMPORTANT]**
>     (RNG)   ** **.

- ** ():**     = 80%    N  .
- **:**    =  80%   .
  -    1000    ****   800   .
- **:**     ****  (`Shuffle`   `master_seed`),    .      .

### 5.4.  

     :

|  |  |  |
|---|---|---|
| **** (V1, V2, Motor) | 6 | `L1..L6`   `height_pct`  `composition` |
| **,  ** | 1 |   `"Nuclear"`  `height_pct = 1.0`     |

  (`V1`, `V2`, `Motor`, `Thalamus`)       `anatomy.toml`.

### 5.5.     ( Baker')

```toml
# config/zones/CortexPie/anatomy.toml

[[layer]]
name = "L4_Sensory"
height_pct = 0.2
# [DOD FIX]       (8%   )
density = 0.08  
composition = { "L4_spiny_MTG_1" = 0.8, "L4_aspiny_MTG_11" = 0.2 }

[[layer]]
name = "L23_Hidden"
height_pct = 0.5
density = 0.04  #   (4%)
composition = { "L23_spiny_MTG_1" = 0.8, "L23_aspiny_MTG_1" = 0.2 }

[[layer]]
name = "L5_Motor"
height_pct = 0.3
density = 0.02  #     ,    (2%)
composition = { "L5_spiny_MTG_1" = 0.8, "L5_aspiny_MTG_11" = 0.2 }
```

**    (Baking):**

-     : `Max_X = Width_um / voxel_size`, `Max_Y = Depth_um / voxel_size`, `Max_Z = Height_um / voxel_size`
-    `Total_Capacity = 0`.
-    `[[layer]]`:
  - `Z_Start = ( height_pct)  Max_Z`
  - `Z_End = Z_Start + height_pct  Max_Z`
  - `Layer_Volume_Voxels = Max_X  Max_Y  (Z_End - Z_Start)`
  - `Layer_Budget = floor(Layer_Volume_Voxels  layer.density)`
  - `Total_Capacity += Layer_Budget`
  -     `composition`:
    - `Type_Count = floor(quota  Layer_Budget)`
- Warp Alignment: `Padded_N = align_to_warp(Total_Capacity)` (    32).

---

## 6. : `blueprints.toml`

      4-  .  **  (Integer Physics)**        FPU.        -  .

### 6.1.  

```toml
[[neuron_type]]
# ID 0 (: 00) -  
name = "Vertical_Excitatory"

# ---   (Units: microVolts / absolute integers) ---
threshold = 42000               #  
rest_potential = 10000           #  
leak_rate = 1200                 #   (  )

# ---  (Units: Ticks) ---
refractory_period = 15           # u8.   .
synapse_refractory_period = 15   # u8.    ().

# ---   (Units: Integer geometry) ---
conduction_velocity = 200        # [  ]  (   )
signal_propagation_length = 10   #  ""    (Active Tail, per-variant)

# ---  (Adaptive Threshold) ---
homeostasis_penalty = 5000       # +5000    
homeostasis_decay = 10           #     ,  threshold    

# --- Adaptive Leak Hardware ---
adaptive_leak_max = 500          #      
adaptive_leak_gain = 10          #     
adaptive_mode = 1                # 1 = , 0 = 

# --- Neuromodulation (R-STDP) ---
d1_affinity = 128                #   LTP   (128 = 1.0x)
d2_affinity = 128                #       (128 = 1.0x)

# ---   (Steering & Arborization) ---
steering_fov_deg = 60.0              #    (Cone Tracing)

#    (Terminal Arborization, . 04_connectivity.md 1.4):
arborization_target_layer = "+1"     #   ("+1" =    Z,   ,  "L4")
arborization_radius_um = 150.0       #   ""   
arborization_density = 0.8           #   (0.0 =  , 1.0 =   V_noise)

sprouting_weight_distance = 0.5  # f32.  = 
sprouting_weight_power   = 0.4  # f32. soma_power_index (. 04_connectivity.md 1.6.1)
sprouting_weight_explore = 0.1  # f32.    (   )

# ---   ( ) ---
spontaneous_firing_period_ticks = 500  #      500  (0 = )

[[neuron_type]]
# ID 3 (: 11) -  
name = "Horizontal_Inhibitory"

# ---  ---
threshold = 40000                #    ( )
rest_potential = 10000
leak_rate = 1500                 #   (Leak )

# ---  ---
refractory_period = 10           #  
synapse_refractory_period = 5

# ---   ---
conduction_velocity = 100        # [  ]  

# ---  ---
homeostasis_penalty = 3000
homeostasis_decay = 15

# --- Neuromodulation ---
d1_affinity = 64                 #     
d2_affinity = 64

# --- Sprouting Score (    ) ---
sprouting_weight_distance = 0.6  #    
sprouting_weight_power   = 0.3  #    
sprouting_weight_explore = 0.1

# ---   ( ) ---
spontaneous_firing_period_ticks = 200  #  -   (  200 )
```

### 6.3.  DDS Heartbeat

Baker  - `spontaneous_firing_period_ticks`    `heartbeat_m`  GPU (. [03_neuron_model.md 4.1](./03_neuron_model.md#41--zero-cost-branchless)):

-  `period == 0`: `heartbeat_m = 0` (  ).
- : `heartbeat_m = clamp(65536 / period, 1, 65535)` (  DDS).

** :**
- `period = 500`  `heartbeat_m = 65536 / 500  131`    1   500 
- `period = 200`  `heartbeat_m = 65536 / 200  328`    1   200 
- `period = 1`  `heartbeat_m = 65535`     (spike generator,  )

** (Baking Tool):**

```rust
// axicor-baker/src/compile_heartbeat.rs

pub fn compile_dds_heartbeat(period_ticks: u32) -> u16 {
    if period_ticks == 0 {
        return 0;  // 
    }
    
    // DDS : 0..65535 (16 )
    let heartbeat_m = (65536 / period_ticks.max(1)).min(65535) as u16;
    
    //  
    assert!(heartbeat_m > 0, "period_ticks > 65536 is invalid (heartbeat too rare)");
    assert!(heartbeat_m <= 65535, "Critical: heartbeat_m overflow");
    
    heartbeat_m
}
```

### 6.4.   D1/D2 

 `blueprints.toml`     (Variant)     .

1. **d1_affinity (LTP-like):** 
   - : `Is_Excitatory`  `High` (1.5x), `Is_Inhibitory`  `Low` (0.5x).
   - : `d1_affinity = variant.is_excitatory ? 192 : 64`.
2. **d2_affinity (LTD-like):**
   - : `Is_Excitatory`  `Medium` (1.0x), `Is_Inhibitory`  `High` (2.0x).
   - : `d2_affinity = variant.is_excitatory ? 128 : 256`.

**:**     `VariantParameters` (1  )    `ApplyGSOP`          .

---

## 7. : `brain.toml` (Multi-Zone Architecture)

> [!NOTE]
> **[Planned: Macro-3D]**  `brain.toml`:
//   `[cluster]`   - (  ,  ),     `[simulation]`.

   :   ,       .  ** **     (. [06_distributed.md](./06_distributed.md)).

### 7.1.   

```
brain.toml ()
+- [simulation]  simulation.toml ( )
+- [[zone]]  N ( )
|  +- name: "SensoryCortex"
|  +- blueprints: "config/zones/.../blueprints.toml"
|  +- baked_dir: "baked/SensoryCortex/"
+- [[connection]]  M (   Ghost Axons)
   +- from: "SensoryCortex"
   +- to: "HiddenCortex"
   +- output_matrix: "sensory_out"
```

**:**      `baked_dir`  `Axicor_Models`.    `baked_dir`  **   ** (Must have: `.state`, `.axons`, `.gxo`).

**Smart Path Resolution:**    .   `--brain mouse_agent`,    `Axicor_Models/mouse_agent/brain.toml`.         `GENESIS_MODELS_PATH`.

### 7.2.  [simulation]

   `simulation.toml` -     `tick_duration_us`   .

```toml
[simulation]
config = "config/simulation.toml"  #    
```

**:** ,  SensoryCortex   ,  HiddenCortex,     (. [06_distributed.md 2.4](./06_distributed.md#24-batch-synchronization)).

### 7.3.  [[zone]]:  

```toml
[[zone]]
name = "SensoryCortex"                                 #   ()
blueprints = "Axicor_Models/mouse_agent/zones/SensoryCortex/blueprints.toml"  #   
baked_dir = "Axicor_Models/mouse_agent/baked/SensoryCortex/"                 #   

[[zone]]
name = "HiddenCortex"
blueprints = "config/zones/HiddenCortex/blueprints.toml"
baked_dir = "baked/HiddenCortex/"

[[zone]]
name = "MotorCortex"
blueprints = "config/zones/MotorCortex/blueprints.toml"
baked_dir = "baked/MotorCortex/"
```

#### 

|  |  |  |
|---|---|---|
| `name` | `String` |   .   `[[connection]]`   . : `"V1"`, `"V2"`, `"Motor"`, `"Thalamus"`. |
| `blueprints` | `String` |   `blueprints.toml`  .   4-   (6).        `brain.toml`. |
| `baked_dir` | `String` | ,   : `.state` (  ), `.axons` (), `.gxo` ( , ), `.gxi` ( , ).   Baking (axicor-baker). |

### 7.4.  [[connection]]:  

 Ghost Axon Projections -            (. [06_distributed.md 2.5](./06_distributed.md)).

```toml
[[connection]]
from = "SensoryCortex"                  #  
to = "HiddenCortex"                     #  
output_matrix = "sensory_out"           #      
                                        # (debe existir  SensoryCortex/blueprints.toml)
width = 64                              #    
height = 64                             #    
entry_z = "top"                         #  : "top" | "mid" | "bottom" |   
target_type = "All"                     #   : "All" |  
growth_steps = 1000                     #   Cone Tracing  Ghost Axon Growth

#   (Planned,     )
# synapse_weight = 5000                 #    Ghost Axon
# latency_ticks = 2                     #   ()
```

#### 

1. **from/to:**   `from`   `output_matrix`   `sensory_out`   `blueprints.toml`.
2. **width/height:**  .   : `spatial_scale_x = zone_width / width` (. [08_io_matrix.md](./08_io_matrix.md)).
3. **entry_z:**    ()      . `"top"=  L1, "bottom" =  L5/6`.
4. **target_type:**      Ghost Axon. `"All"` =   .
5. **growth_steps:**    .   0 -  ** ** (  ).   > 0 - ** ** (Cube Tracing   ).

### 7.5.   

```toml
[simulation]
config = "config/simulation.toml"

[[zone]]
name = "SensoryCortex"
blueprints = "config/zones/SensoryCortex/blueprints.toml"
baked_dir = "baked/SensoryCortex/"

[[zone]]
name = "HiddenCortex"
blueprints = "config/zones/HiddenCortex/blueprints.toml"
baked_dir = "baked/HiddenCortex/"

[[zone]]
name = "MotorCortex"
blueprints = "config/zones/MotorCortex/blueprints.toml"
baked_dir = "baked/MotorCortex/"

# ---    ---

[[connection]]
# V1  Hidden:  
from = "SensoryCortex"
to = "HiddenCortex"
output_matrix = "sensory_out"
width = 64
height = 64
entry_z = "top"
target_type = "All"
growth_steps = 1000

[[connection]]
# Hidden  Motor:  
from = "HiddenCortex"
to = "MotorCortex"
output_matrix = "hidden_out"
width = 32
height = 32
entry_z = "mid"
target_type = "All"
growth_steps = 800
```

### 7.6. Runtime  (Startup Sequence)

  :

1. ** `brain.toml`.**
2. **  `[[zone]]`:**
   -  `blueprints.toml`   
   -     `baked_dir/`  VRAM (`.state`, `.axons`)
   -   `.gxo` ( )   UDP output servers
   -  `ZoneRuntime`   
3. **  `[[connection]]`:**
   -   `from`  `to`   `zones`
   -  `IntraGpuChannel` (voir [06_distributed.md 2.6](./06_distributed.md#26-intra-gpu-channel-ghost-axon-routing))   Ghost Axons
   -    Cone Tracing ( `growth_steps > 0`)    
4. ** BSP Barrier**   (1.2.1, [06_distributed.md 1.3](./06_distributed.md#13-bsp-barrier))

**Fail-Fast Policy:**  -       immediate panic  .  `baked_dir/`  **not bootable**.

---

##  

```
simulation.toml (Laws of Physics)
    
anatomy.toml (Layer heights, Density per layer)
    
blueprints.toml (Neuron types, Synapse rules)
    
brain.toml (Multi-zone topology, Inter-zone connections)
    + baked/ (Compiled, immutable binary snapshots)
    + [Shard configs] (Instance-specific: offset, dimensions)
```

### 6.2.    Runtime

  ,    GPU-  Baking (2):

|  |  |  Runtime |  |
|---|---|---|---|
| **Potentials** | `threshold`, `rest_potential`, `voltage` | `i32` |       |
| **Timers** | `refractory_period`, `synapse_refractory_period` | `u8` |  > 255  (25 )     |
| **Geometry** | `conduction_velocity` | `u16` | [  ]  ,   65535 |
| **Homeostasis** | `homeostasis_penalty`, `homeostasis_decay` | `i32` / `u16` | Penalty   threshold (i32), decay -   |


---

## 8. Roadmap:  Hot-Reload (Manifest Extensions)

> [!NOTE]
> **[Planned]**   `manifest.toml`      .

### 8.1.   (Night Phase)
- ****:   `night_interval_ticks`      .
- ****:    (, , )          .             .

### 8.2.  
-  `gsop_potentiation`, `gsop_depression`  `prune_threshold`         ,   .
