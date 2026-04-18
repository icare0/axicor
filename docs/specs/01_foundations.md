# 01.  (Foundations)

>   [Axicor](../../README.md).    .  : **GNM** (Axicor Neuron Model),  : **GLIF** (Axicor Leaky Integer-Fire), : **GSOP** (Axicor Spike-Overlap Plasticity).

---

## 1.    (Units & Coordinates)

             .

### 1.1.   (Spatial Domain)

> [!NOTE]
> **[Planned: Macro-3D]**  Nested Coordinate System   .
// - **Micro-Space (VRAM, 32-bit):**     `PackedPosition` (11-bit X, 11-bit Y, 6-bit Z).   .
// - **Macro-Space (Cluster, 64-bit):**    `axicor-node`    : `Global_X = Shard_Macro_X + Local_X`. 
// - **:** GPU     -.     GhostPacket ""       ,    64- -.

- ** :** `1.0` (float) = **1 ** ().
  -  :   ,  ,     .
  - :         .
- **  ():** `0.0 ... 1.0` (pct).
  -  :    (Layers)   .
  - :  L1   `0.0 - 0.1` (10% ).     `world.height` ( )  .     ""    .
- ** (Grid):**    **** (   , , 10 ).
  -  :      (Spatial Hashing).

### 1.2.   (Axon Segmentation)

  ,      .  -   ****.

- **:** `segment_length_voxels` (, `4`  = 100    25 ).
- **:**
  -   .         .
  - **:**  -  .      `(X, Y, Z)`,    `Axon_ID + Segment_Index`.
  - **:**        ( ).       ( )  .
- **:**     `segment_length_voxels` .  1000   1   (  1 ) -  **10 ** (  100 ).

### 1.3.  :   (Active Tail)

   (  ).   ,   .

- **:** `signal_propagation_length` (   ).
- **:**
  -  -  ,     ().
  -  `signal_propagation_length = 3`,    T   `[i, i+1, i+2]`.
  -   `T+1`      `v`:  `[i+v, i+v+1, i+v+2]`.
- **:** ,         .      .

**:    **

 `v_seg = 1` (1 /), `propagation_length = 3` (3 ):

```
 0:  head=3  Active: [1, 2, 3]       ------+
                                               |
 1:  head=4  Active: [2, 3, 4]         ----+--+
                                               |  |
 2:  head=5  Active: [3, 4, 5]           --+--+--+
                                               |  |  |
: --+--+--+--+--+--+--+--
            0  1  2  3  4  5  6  7

```

 :
- `head`   `v_seg`  
-   `[head - propagation_length + 1, head]`    
- ,     ,  

### 1.4.   (Temporal Domain)

- ** :** 1 .
- **:** `tick_duration_us = 100` (0.1 ).
  - **:** 1   ""  GSOP (  ). 100  -    .
  -   (, )   `u64` .
  - :  5  = 50 .

### 1.5.   (Pre-calculated)

     (Hot Loop).       (/, , ),           GPU.

**  :**

|   |   |  |  |  |
|---|---|---|---|---|
| `signal_speed_um_tick` | `signal_speed` (/), `tick_duration_us` | `(speed_m_s  1e6 /)  (tick_duration_us  1e-6 )` | `0.5 /  100  = 50 /` |       |
| `v_seg` | `signal_speed_um_tick`, `segment_length_um` | `signal_speed_um_tick  segment_length_um` | `50 /  50  = 1 /` |    GPU- (`head += v_seg`) |
| `segment_length_um` | `voxel_size_um`, `segment_length_voxels` | `voxel_size_um  segment_length_voxels` | `25   2 = 50 ` |    `v_seg` |
| `matrix_spatial_scale` | `zone_width_um`, `matrix_width_px`, `zone_depth_um`, `matrix_height_px` | `(width_um / width_px, depth_um / height_px)` | `(1000  / 64 px, 1000  / 64 px) = (15.625, 15.625)` |       |

### 1.7.    (C-ABI)

 Axicor        .       Zero-Copy mmap .

#### 1.7.1. The 1166-Byte Invariant (VRAM/State)
          `.state`.
**:** `Soma (14) + 128 * (Targets:4 + Weights:4 + Timers:1) = 1166 `.
*   **Soma (14B):** Voltage(4) + Flags(1) + Threshold(4) + Timer(1) + SomaToAxon(4).
*   **Dendrites (1152B):** 128 ,   9  (Target Axon ID & Offset, 32-bit Weight, 8-bit Synaptic Timer).

#### 1.7.2. SHM Night Phase IPC v4 (Exchange)
    Shared Memory       .
**:** `Header(64) + Weights(N*512) + Targets(N*512) + Flags(N*1) = 1025   `.
*    (`i32`)   (`u32`)   4    (128  = 512 ).
*          (Activity Gate).

**:**     [`axicor_core::physics::compute_derived_physics()`](https://github.com/your-repo/axicor-core/src/physics.rs) -      raw .

**  (Startup  GPU):**

   :
-  `simulation.toml`: `signal_speed = 0.5 /`, `tick_duration = 100 `
-  : `signal_speed_um_tick = 50 /`
-  GPU-   **`50`**
-     : `axon_head += 50` ( `Speed  Time`)

   (`v_seg`):
-  : `segment_length_voxels = 2`, `voxel_size_um = 25`
-  : `segment_length_um = 50 `
- : `v_seg = 50 /  50  = 1`
-  GPU-   **`1`**
-  : `segment_head += 1` ( `u32` )
- Active Tail (1.3) -   `[head - propagation_length .. head]`,   `v_seg`  

### 1.6.    (Math Check)

     -   .   ,  .

- **:** `signal_speed_um_tick` ****   `segment_length_um`.
  -   /: `v_seg = signal_speed_um_tick / segment_length_um`  ** **.
  -  `v_seg`  (, 1.5 /) -    float-,    .  GPU  .
- **:**     (Active Tail, 1.3)    `v_seg` .    -  `u32`  .
- ** :**

|  |  |  |
|---|---|---|
| `tick_duration_us` | `100` |  (0.1 ) |
| `voxel_size_um` | `25` |  |
| `segment_length_voxels` | `2` |  (= 50 ) |
| `signal_speed_um_tick` | `50` | / (= 0.5 /) |
| **`v_seg`** | **`1`** | **/** |

- **:** `50 /  50 / = 1`  ().
- ** :** 0.5 / -       .

#### 1.6.1.   v_seg

       :

** 1.    :**
```
segment_length_um = voxel_size_um  segment_length_voxels
segment_length_um = 25   2 = 50 
```

** 2.   (/):**
```
v_seg = signal_speed_um_tick  segment_length_um
v_seg = 50 /  50 / = 1 /
```

** 3.  :**
```
assert (signal_speed_um_tick % segment_length_um == 0)
assert (50 % 50 == 0) 
```

   3  **  ** -        .

> **:**          `v_seg`  ,    .

---

## 2.   (Master Seed)

     .      10   .    -  `master_seed`,         .

### 2.1. 

- **:** `master_seed` (`String`)  `simulation.toml`.
- ** String:**    `u64`  .      ( ),     : `"GENESIS"`, `"HELLO_WORLD"`, `"DEBUG_RUN_42"`.
- **:**  **   **.  `time(NULL)`, `std::random_device`  `SystemTime::now()`     .

```toml
[simulation]
master_seed = "GENESIS"
```

### 2.2.  (Stateless Hashing)

    `Random(seed)`   `.next()`.    (     )   `rand()`  -     .

- **:** ,    .
- **:** `Local_Seed = Hash(Master_Seed_u64 + Unique_Entity_ID)`
  - :   5001    `WyHash64(Seed + 5001)`.
  - : `WyHash64` - , , .
- **  :**    `Entity_ID`,     .     ()   ,   5001    - ,     .

### 2.3.  (Structural Determinism)

- **:**     `master_seed` +      = **--   ** (, , , UV-).   :   1  CPU     100 GPU   .
- **Hot Loop ():**     GLIF   GSOP  - Integer Physics      GPU.
- **:**      ,      ****.  GPU =   =    =  . ** :**   =   ,  .
- **Backend Diversity:** [MVP] AMD ROCm/HIP   -  .  `axicor-compute`  -   NVIDIA (CUDA),   AMD (Polaris/RDNA),  --     `master_seed`. : [10_hardware_backends.md](./10_hardware_backends.md).
- **:**        -   `master_seed`,     `night_cycle_count`.
