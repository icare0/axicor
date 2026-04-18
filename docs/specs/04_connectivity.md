# 04.  (Connectivity)

>   [Axicor](../../README.md).     ,   ,   .

---

## 1.     (Connectivity)

### 1.1. : Pub/Sub

|  |  |  |
|---|---|---|
| ** (Publisher)** |  |     (Active Tail + Global Buffer).   ,  0  1 000 000 . **   ** (   ). |
| **/ (Subscriber)** |  | ,   Axon ID .   = 128 . |

  128 -   **  **  ,     .

### 1.2.   (Axon Classes)

 `anatomy.toml`  `blueprints.toml`      ,     :

|  |  |  |
|---|---|---|
| **Local** () |  ,   |  ,  |
| **Horizontal** () |    XY |     |
| **Projectors** ( ) | /  |  `Port Out` + `Port In` + `Delay_Counter` |

- ** Projectors:**     ( )   .   ** **    (   ) -     `Delay_Counter`.

### 1.3.  (Axon Growth)

- ** (Target):**  "  " (    ),     Z (/)   .
- ** (Penetration):**       ,    (       `axon_target_level`).
- ** (Stop Condition):**
  -       : `Soma_Rel_Z = (Soma_Z - Layer_Min) / Layer_Height`.
  -          : `Target_Z = Target_Layer_Min + (Soma_Rel_Z * Target_Layer_Height)`.
- **:**  ,    .
- **  (H/V):**
  -   (`growth_vertical_bias  0.5`):  =    Z.    Target_Z.
  -   (`growth_vertical_bias < 0.5`):  =    XY-  .        Z   .
- ** (Identification):**     ** 4-  ** - (Type Index 0..15, . [03_neuron_model.md 2](./03_neuron_model.md)).
  -        ( Learning Rates)     ,     .

### 1.4.   (Terminal Arborization)

  -  .     ,        -   GPU  Day Phase.

####  :  (Trunk / Projection)

       (`Target_Z`)  .   `V_global` -    ,  .   Cone Tracing,   4.

####  :  (Terminal Arborization)

  `Target_Z`   :
- **`V_global` ** -   .
- **`V_noise` () ** -      .
-      `arborization_radius_um`,     50100    .

```
  ():                       ():
                                    
                                  +===========+
    |  V_global                      |   |    (Target_Z)
    |                                 |     |
    | (Cone Tracing)                  |       |     
    |                                 |    ~~     |      arborization_radius
    +------------------ Target_Z     +===========+
```

####    GPU

 3D- (    )  **  CPU** -   Baking   . GPU  Day Phase    ** 1D-  `BurstHeads8`** -  ,     .

      `axicor-baker` ,    `shard.paths`.   GPU-  = 0.

#### :   

     `SpatialGrid` 25     .       .

       **50100 **.              -     2050      .

### 1.5.  (Dendrite Connectivity)  En Passant Synapses

- **Sprouting Candidate Search:**             (tips),   **   ** (En Passant synapses). 
- **Axon Segment Grid (O(K) Spatial Hashing):**    3D Spatial Hash Grid,        ( `shard.paths`).    O(N) ,    **O(K) Spatial Hashing** (Zero-Cost Spatial Search).
- **Hebbian Structural Rule:**   (  ) **      ** (`flags[i] & 0x01 != 0`).   90-99%   CPU    .

**Sprouting Density Invariant (The Dense Rule):**
       ,   CPU **   (0..127)**.  `sort_and_prune`  GPU      .    `target`            .     (,  127)     Day Phase -  Early Exit (`if (target == 0) break`).

**Dead on Arrival Protection ( ):**
      (`initial_synapse_weight`)      (`<< 16`).        (`prune_threshold << 16`),     "" ( ),             .

- **:**  `PackedTarget`  `Axon_ID` (24 )  `Segment_Offset` (8 ).   GPU  Day Phase      `Active Tail`   .
- **  (Hard Filter):**
  -        ( Segment ID)   .
  - :       (whitelist/blacklist).     .
- ** (Limits):**
  - **Max Synapses:**  128    .
  - **Rule of Uniqueness:**    **  **   Axon ID.    -   (128)     .

### 1.6.   (Super-Synapse)

 -      (`i16`:  -32768  +32767).      `is_inhibitory`  `blueprints.toml`: (+) = Excitatory, (-) = Inhibitory.      per-type  `initial_synapse_weight`.

- **:**  ,     : `Soma_Voltage += Synapse_Weight` (GPU: `v += (int32_t)dendrite_weights[col_idx]`, cast i16i32    128 ).
- **:**
  -   ( ):  = `initial_synapse_weight` (  `74`).
  -   ():  =  `32 767` (i16 max).  `+32 767`  .
- **:**    (32767)  443  (74  443  32767).  -    .
- ** :**   ****  . GSOP    : `new_abs = abs(w) + delta`,  = `sign(w)  new_abs`. Clamp: `0  abs  32767`.
- **  :**   5     (  ).    **** - 5   10 = 1   50.

**    (Night Phase Sprouting):**

         ( O(K) Spatial Hash Grid),    `initial_synapse_weight`  **  - ** (Sender-Side Type),     .

 (DOD)   4-       Spatial Grid (`SegmentRef.type_idx`).    `is_inhibitory = true`,    ** ** (, `-74`).  ,  **       **.    -     Baker  Memory Corruption.

```rust
// axicor-baker/src/bake/sprouting.rs
fn create_new_synapse(
    axon_id: usize,
    axon_tips: &[u32],  // Packed UVW  + 4- 
    cfg: &SprintingConfig,
) -> i16 {
    // Extract type ( 4 )
    let type_bits = (axon_tips[axon_id] >> 28) & 0xF;
    let is_inhibitory = (type_bits >> 1) & 1 != 0;  //  1  = E/I 
    
    //     ,   
    let sign = if is_inhibitory { -1 } else { 1 };
    let initial_weight = cfg.initial_synapse_weight as i16;
    
    sign * initial_weight  //  is_inhibitory: -74, : +74
}
```

### 1.7.   (Baking Heuristics)

  45%          (128).   Baking (CPU)     ,   :

1. **Distance Bias:**    (   ).
2. **Type Matching:**   `dendrite_whitelist` ( )  `sprouting_weight_type` ( ).

#### 1.7.1. Sprouting Score ( , Night Phase)

  Night Phase (Sprouting)   -   .    :

```rust
// axicor-baker/src/bake/sprouting.rs

///  1   Night Phase  Sprouting.
///    dendrite_weights[],    GPU (Step 2: Download).
fn compute_power_index(soma_id: usize, weights: &[i16], padded_n: usize) -> f32 {
    let mut power = 0u32;
    for slot in 0..128 {
        let w = weights[slot * padded_n + soma_id];
        power += w.unsigned_abs() as u32; //  float,  
    }
    power as f32 / (128.0 * 32767.0) //  0.0..1.0
}

fn sprouting_score(
    dist: f32,
    target_power: f32, // compute_power_index()  
    epoch_seed: u64,   // wyhash(dendrite_id, master_seed + night_epoch)
    cfg: &SproutingWeights,
) -> f32 {
    let dist_score  = 1.0 / (dist + 1.0);                   //   
    let power_score = target_power;                          //   
    let explore     = random_f32(epoch_seed);                //    

    dist_score  * cfg.weight_distance
        + power_score * cfg.weight_power
        + explore     * cfg.weight_explore
}
```

|  |  |   |
|---|---|---|
| `weight_distance` |   =  | 0.5 |
| `weight_power` |   ( )   | 0.4 |
| `weight_explore` |  /   ,      | 0.1 |
| `weight_type` |      ( ) | 0.2 |

> **:** `soma_power_index` = |weight|  . ,  ,   .    : 1  = 1  + `max_dendrites = 128`.

### 1.8.   (Ghost Axons)

> [!NOTE]
> **[Planned: Macro-3D]**  Z-  Cone Tracing.
//          (L1-L6)     .
//       `Isotropic_Growth`.
//     `V_global`     Z-,    3D-   Z+,    `axon_growth_max_steps`.

---

## 5. Roadmap:   (Topology Distillation)

> [!NOTE]
> **[Planned: ]** 
        .

### 5.1.   (Path-Based Extraction)
- ****:    (Motor Readout)      (Back-tracing)      (`abs(w) > threshold`).
- ** **:              (PV/Sst),    .
- ****:       ,     .

### 5.2.   (Ensemble Merging/Stitching)
- ****:           .
- ****: Baker      `blueprints.toml`,    `.state` -.   ,     ,  -    .

### 5.3.    :
1. **Dopamine-Gated Distillation**:        ,         (`global_reward`).
2. **Quantization Optimization**:      `i16`  `i8`  `ui4`      .
3. **Sub-graph Grafting**:     (, HiddenCortex   )       .

#### 

        (`ShardBounds`).  :

1.  **GhostPacket** -     
2.  B     `inject_ghost_axons()`
3. **Ghost Axon**    `remaining_steps`    (`entry_dir`)
4. Ghost Axon       B     
5.  Ghost Axon          C

```rust
// axicor-baker/src/bake/axon_growth.rs

pub struct GhostPacket {
    pub origin_shard_id: u32,  //  
    pub soma_idx: usize,       //   (usize::MAX  )
    pub type_idx: usize,       //   ( whitelist, affinity  ..)
    pub entry_x: u32,          //     
    pub entry_y: u32,
    pub entry_z: u32,
    pub entry_dir: Vec3,       //     ()
    pub remaining_steps: u32,  //    
}
```

####  Ghost Axon   B

- ** `v_global`** - Ghost Axon    ,   
- ** `v_attract`** -     B ( Cone Tracing)
- **`soma_idx = usize::MAX`** -     GSOP   ( Ghost Axon   -)
- ** ** -   B    Ghost Axon   

####  ( )

`ShardBounds::full_world(&sim)`   =   .  
Ghost Packets **  **,   .  
   -  .

- **:**   Global Connectivity (   , . [06_distributed.md 3](./06_distributed.md)).   -  .

### 1.9. Living Axons (Tip Nudging)

 Axicor    Baking.           ,      (Structural Plasticity).

     CPU   ,   **Activity-Based Nudging**:

#### Logic:   ?

1. **  (Activity Gate):**
   Baker Daemon   `soma_flags` (  `flags_offset`  SHM v4, . [07_gpu_runtime.md 1.7](./07_gpu_runtime.md#17-shm-binary-contract-night-phase-ipc-v4)).
   
    0-    1 (`flags & 0x01 == 1` -   ), **   (Tip) **  `step_and_pack` (  1 ).
   
      - CPU **  ** (O(1)  ).

2. **Ghost- (Inertial Nudging):**
   Ghost-     (`soma_idx = usize::MAX`).   **  **     (`forward_dir`),    `remaining_steps`   .

3. **Persistence:**
        (`nudging`)   :
   1.  `len = lengths[axon_id]`.
   2.  `len < 256`,   `PackedPosition`   `paths[axon_id * 256 + len]`.
   3. `lengths[axon_id] += 1`.
   4.      `AxonSegmentGrid`        .

####   (CPU-side, Night Phase)

```rust
// axicor-baker/src/bake/growth.rs

/// Living Axon    Night Phase
pub struct LivingAxon {
    pub axon_id: usize,              //    axon_tips
    pub soma_idx: usize,             // usize::MAX  Ghost-
    pub tip_uvw: u32,                // Packed UVW + type ( 4 )
    pub forward_dir: Vec3,           //  /  
    pub remaining_steps: u32,        //    
    pub last_night_active: bool,     //     
}

fn nudge_living_axons(
    living: &mut [LivingAxon],
    soma_flags: &[u8],
    spatial_grid: &mut SpatialGrid,
) {
    for axon in living.iter_mut() {
        // Decision:     ?
        let should_grow = if axon.soma_idx == usize::MAX {
            true  // Ghost-:  
        } else {
            //  :     
            soma_flags[axon.soma_idx] & 0x01 != 0
        };
        
        if should_grow && axon.remaining_steps > 0 {
            // Step:   1   
            axon.tip_uvw = step_and_pack(
                axon.tip_uvw,
                axon.forward_dir,
                /* gravity */ None,  //  CPU  ,  
            );
            axon.remaining_steps -= 1;
            
            // Grid update:     Spatial Grid
            spatial_grid.insert(axon.axon_id, axon.tip_uvw);
        }
    }
}
```

#### : Data-Oriented 

- **:**  ~1030%      (  ).    O(1).
- ** CPU:**   (~50  )  ~10K50K  , 0-  .
- ** :**   **     **.        .
- **Spatial Coherence:**  Spatial Grid ,  Sprouting ( )          .

* DOD- ,       "",   ,         .*

---

## 2.    (Plasticity & Math)

### 2.1.  

- **:** `i16` (-32768..+32767).   E/I (   ).
- **:**  =    .  : `Soma_Voltage += Weight`.  (+/-)  E/I,    .
- **:**      (`initial_synapse_weight` per-type  blueprints.toml).

### 2.2.  : Active Tail Check

- **:**    ,    ** ** (Active Tail)    (. [05_signal_physics.md](./05_signal_physics.md)).
- **:**        GSOP .    ,    .
- **:**

|  | Active Tail |  |
|---|---|---|
|  ,     | [OK]  | `Weight += potentiation` () |
|  ,      | [ERROR]  | `Weight -= depression` () |
|    `1` | - |   (Pruning) |

### 2.3. GSOP  LUT (Variant-Dependent Learning)

  GSOP ** **.    [02_configuration.md 6](./02_configuration.md)    `__constant__`  GPU  LUT  Type ID ( 16   `blueprints.toml`):

| Type Index | Potentiation | Depression |  |  |
|---|---|---|---|---|
| `0..15` |   |   | `1.0` |   ( per-type) |

```
// GPU ()
let gsop = GSOP_LUT[type_id];
weight += gsop.potentiation * gsop.multiplier;
```

### 2.4.   (Inertia Curves)

   (Permanence bit) - **  **.

- **:**  `abs()` (`i16`)   **16 ** ( 2048 ).     `inertia_curve: [u8; 16]`   (per-type).
- **:**   ,     ( ,   ).   ,  - .

|  |  abs() |  |  |
|---|---|---|---|
| 03 | 0  8 191 |  |  ,   |
| 47 | 8 192  16 383 |  |  |
| 811 | 16 384  24 575 |  |   |
| 1215 | 24 576  32 767 |  |    |

```
// GPU ()
let rank = abs(weight) >> 11;                // 16  ( 2048  )
let inertia = p.inertia_curve[rank];         //   constant memory (per-type)
let delta = gsop.potentiation * inertia;   //   ,   delta
weight = sign(weight) * clamp(abs(weight) + delta, 0, 32767);
```

**  Permanence bit:**      - ,  ,   . Inertia Curves          .

### 2.5.   (D1/D2 Affinities)

     (LTM/WM)     ALU     64-  . 
       .     `d1_affinity` ( LTP  )  `d2_affinity` ( LTD  ).     `05_signal_physics.md`.

### 2.6.   (Nightly Sort)

    (CPU, . [07_gpu_runtime.md](./07_gpu_runtime.md))   :

1. **GPU Sort & Prune:** Segmented Radix Sort  `abs(weight)` descending.   `abs(w) < threshold` .
2. ** Promote / Evict:**    (target=0)     ,       ,      `0..ltm_slot_count` (LTM),      `ltm_slot_count..127` (WM).   Promote  Evict  .
3. **:**   LTM  `ltm_slot_count`   , WM -  .


---

## 3.   (Multi-Zone Connectivity)

Axicor     **** (Zones) -      ,    .       Ghost Axons.  : **  (GPU-)        **.      Channel.

### 3.1. : Zone  Shard  Node

|  |  |  |
|---|---|---|
| **Simulation** |   :  `simulation.toml`    (tick_duration, voxel_size, sync_batch, master_seed). BSP-   . | 1 |
| **Zone** |    (V1, V2, A1, Hippocampus).   `blueprints.toml`, `anatomy.toml`.   (`width_um`, `depth_um`, `height_um`) -  (~simulation.toml),     ****  **** (     koordin XYZ). |  |
| **Shard** |   (Tile)  .      XY (. [06_distributed.md 1](./06_distributed.md)).   = 1 .   = NM . |  |
| **Node** |   : 1 GPU + RAM.  Node      . |  |

** :**
```
Simulation ( )
+-- Zone: V1
|   +-- Shard: V1[0,0]  Node 0 (GPU 0)
|   +-- Shard: V1[1,0]  Node 1 (GPU 1)
|   +-- Shard: V1[0,1]  Node 2 (GPU 2)
+-- Zone: V2
|   +-- Shard: V2[0,0]  Node 0 (GPU 0)
|   +-- Shard: V2[0,1]  Node 1 (GPU 1)
+-- Zone: Motor
    +-- Shard: Motor[0,0]  Node 0 (GPU 0)

   3 GPU:
+-- Node 0: [V1[0,0], V2[0,0], Motor[0,0]] - - - (3 ,  )
+-- Node 1: [V1[1,0], V2[0,1]]  ---------------- (2 )
+-- Node 2: [V1[0,1]]  - - - - - - - - - - - - - (1 )
```

** :**
```
 1:   (1 GPU)
+-------------------------------------+
|   Node 0 (RTX 4090, 24 GB VRAM)     |
|  +------+ +------+ +------------+   |
|  |V1 1 | |V2 1 | |Hippoc. 1  |   |
|  |shard | |shard | |shard       |   |
|  +------+ +------+ +------------+   |
|  IntraGPU (VRAM  VRAM, 0 copy)     |
+-------------------------------------+

** 1 - Memory Breakdown (1 RTX 4090, 3   100K ):**
- Soma SoA: 3  100K  910 B = 273 MB
- Axons: 3  100K  10  4 B = 12 MB
- I/O (input + output): 3  200M pixels  4 B = 2.4 MB
- **Total: ~290 MB (1.2%  24 GB)**  Very Comfortable

```

** 2 - Memory Breakdown (2 A100, 3   1M ):**
```
+--------------------------------------------+
|                  Machine 0                 |
|  +-----------------+  +-----------------+  |
|  | Node 0 (GPU 0)  |  | Node 1 (GPU 1)  |  |
|  | V1 (2 shards)   |  | V2 + A1         |  |
|  +--------+--------+  +--------+--------+  |
|           +--------------------+           |
|         IntraNode (NVLink P2P)             |
+--------------------------------------------+
```
|  |  |  |
|---|---|---|
| **Node 0 (V1)** | 1M  1166 B | 1.166 GB |
| **Node 1 (V2 + A1)** | 2M  1166 B | 2.332 GB |
| **Axons** | (10M + 20M)  4 B | 120 MB |
| **I/O Matrices** | 16.5 MB + 104 MB | 120.5 MB |
| --- | --- | --- |
| **Node 0 Total** | 1.166 + 60 + 58 | **1.28 GB** (~3.2%  40 GB) |
| **Node 1 Total** | 2.332 + 60 + 62.5 | **2.45 GB** (~6.1%  40 GB) |
| ** ** | 3.73 GB / 80 GB | **~4.7% total** |


** 3 - Memory Breakdown (10- , 20M  total):**
```
         (N   M GPU)
+------------------+  +------------------+
| Machine 0        |  | Machine 1        |
|  Node 0: V1_s0   |  |  Node 2: V1_s1   |
|  Node 1: V2_s0   |  |  Node 3: A1_s0   |
+--------+---------+  +---------+--------+
         +----------------------+
         InterNode (UDP/TCP + BSP)
```
|  |  |
|---|---|
| **Per 1  (Soma)**                     | 1166 B (voltage 4B + flags 1B + offset 4B + timer 1B + somaaxon 4B + dendrites 1152B) |
| **Per 10 axons (avg)**                      | 40 B (10  u32 heads)               |
| **Per 1M  **                 | 16.5 MB (map 4M + bitmask 12.5M)    |
| **Per 1M  soma**                    | 104 MB (soma_ids 4M + history 100M) |
| ---                                         | ---                                 |
| **20M  Soma SoA**                   | 20M  1166 B = 23.32 GB               |
| **200M axon heads**                         | 200M  4 B = 800 MB                 |
| **3  Input matrices (64K  each)**  | 3  1 MB = 3 MB                     |
| **3  Output matrices (64K soma each)**     | 3  6.4 MB = 19.2 MB                |
| **Inter-zone Ghost Axons**                  | ~2.5 GB (assume 10%  soma count)  |
| **Overhead (~10%)**                         | ~2.5 GB                               |
| ---                                         | ---                                 |
| **TOTAL**                                   | ~29.2 GB                            |

**  10   2 GPU (20 GPU total, 40 GB each):**
- 20M   20 GPU = 1M /GPU = 1.46 GB/GPU
- **Utilization: 1.46 GB / 40 GB = 3.65% per GPU** [OK]  
-   10+    20M   

---

> **  :** Scheduler      Channel:
> -    GPU  `IntraGPU`.    GPU    `IntraNode`.      `InterNode`.
> -   ,    -     `Channel` trait.

### 3.2.    (Channel Trait)

  ,     .     **Channel**,    :

| Channel |  |  |  |  |
|---|---|---|---|---|
| **IntraGPU** |    GPU |   VRAM | ~0 | .         . |
| **IntraNode** |    GPU   | NVLink / PCIe P2P | ~ |  DMA  GPU. |
| **InterNode** |     | UDP (Fast) / TCP (Slow) | ~ |     .   . |

**     :**

```rust
trait Channel {
    // Fast Path:       sync_batch
    fn sync_spikes(&mut self, zones: &mut [ZoneRuntime]) -> anyhow::Result<()>;
    
    // Slow Path:    (Night Phase, Sprouting)
    fn sync_geometry(&mut self) -> anyhow::Result<()>;
    
    // Runtime Query:    ( debug  )
    fn channel_type(&self) -> ChannelType;  // IntraGPU | IntraNode | InterNode
}

enum ChannelType {
    IntraGPU,    // Shared VRAM:   
    IntraNode,   // NVLink/PCIe:  DMA  GPU
    InterNode,   // UDP/TCP:   
}
```

** :**
- `sync_spikes(&mut self, zones: &mut [ZoneRuntime])`:       runtime     VRAM-.    :
  - IntraGPU:  `axon_heads[]`   VRAM
  - IntraNode:  CUDA Peer-to-Peer memcpy
  - InterNode: ,  UDP, 
- `sync_geometry(&mut self)`:     (Sprouting, Pruning)   Night Phase

### 3.3. IntraGPU Channel:  VRAM,  

   -     GPU.      SoA- (`soma_voltage`, `dendrite_weights`, `axon_heads`, ...)   `ConstantMemory` (blueprints).

** :** GPU- ****     .         (`UpdateNeurons`, `PropagateAxons`, `ApplyGSOP`)   ,  `padded_n`   `ConstantMemory`.

**Ghost Axon  IntraGPU:**

1. Ghost Axon  V2,    V1 -      `axon_heads[]` ** V2** ( V1).
2. **CPU Sync ( sync_batch):**   `head`   V1  ghost-  V2.
   - **  (Issue #4):**  O(N)  -  `cudaMemcpy`   Ghost Axon.  5120      5120  DMA    sync_batch (  = ).
   - **Planned (Issue #4 ):**  CUDA  `ghost_sync_kernel()`   Ghost Links   - O(1)   , O(N)   .
3.  GPU- V2  ghost     . Active Tail, GSOP, Decay -   .

** :**
- **IntraGPU:**  ,      memcpy
- **IntraNode (NVLink):**   DMA,   - 
- **InterNode (UDP):**    , ~110   sync

**:**
-   IntraGPU, IntraNode  InterNode - **  ,  CPU  ghost-**. GPU-    .
-   GPU 5    blueprints    ,       (  IntraGPU).
-       ,  ,    -    GPU  .

### 3.4.    (Fast & Slow Paths)

      .   .

**1.  (Fast Path / Spikes)** -  `sync_batch`

|  |  |  |
|---|---|---|
| `ghost_axon_id` | `u32` |  ID Ghost Axon    / |
| `tick_offset` | `u8` |     |

**2.  (Slow Path / Growth)** - Night Phase

|  |  |  |
|---|---|---|
| `entry_point` | `(u16, u16)` |    (2D) |
| `vector` | `(i8, i8, i8)` |    |
| `type_mask` | `u8` | Type ID    |
| `remaining_length` | `u16` |    |

:  Handshake ( Ghost Axon)    `ghost_axon_id`.    -   `u32`,   UUID.

### 3.5. Ghost Axons ( )

**:**     ,      .

- **:** Ghost Axon -     Soma.   -    .
- ** :**
  1. **:**   A    `GhostPacket` / `AxonHandover`   B  Ghost Axon.
  2. **:**   A  `SpikeBatch`  `axon_heads[ghost_id] = 0`   B.
  3. **:** `PropagateAxons`  Ghost Axon  . `UpdateNeurons`  Active Tail -  .
  4. **:**  A    `PruneAxon(ghost_id)`   B  .
  5. **:** Ghost Axon      B     C.


### 3.6.   ()

 `brain.toml`       `[[connection]]` (. [02_configuration.md 7.4](./02_configuration.md)):

```toml
[[connection]]
from = "V1"
to = "V2"
output_matrix = "v1_output"       #    V1
width = 64                         #  
height = 64
entry_z = "top"                   #       V2
target_type = "All"               #    
growth_steps = 1000               #   Cone Tracing  Ghost Axon

[[connection]]
from = "V2"
to = "Motor"
output_matrix = "v2_output"
width = 32
height = 32
entry_z = "mid"
target_type = "All"
growth_steps = 800
```

** :**

1. **Baking:**   (axicor-baker)   `[[connection]]`:
   -    `from` ,     `exit_points` 
   -  Ghost Axons  `to`    Cone Tracing    
   - :   `baked/connections/{from}_{to}.ghosts`   `self_soma_id  target_ghost_axon_id`

2. **Runtime:**   (axicor-runtime):
   -   `.ghosts`    `baked/connections/`
   -  `IntraGpuChannel`    (dari [03 04 3.3](./04_connectivity.md#33-intragpu-channel--vram--))
   - Ghost Axons    `axon_heads[]`   (  )

> [!IMPORTANT]
>     MVP-    ** **  Baking      ,     (--, --,  ).

## 4.  : Cone Tracing & Steering

     (Raycast).   ,   **Sensing  Weighting  Steering**    .  ,   ** **   Baking (CPU).  GPU-    .

### 4.1.  1:  (Sensing)

      (Head)    .

- **:**   (Cone Query)  Spatial Hash   PackedPosition (X|Y|Z).
- **:**
  - `FOV` (Field of View):   (, 60).   `fov_cos = cos(FOV/2)` -    .
  - `Lookahead` (`MAX_SEARCH_RADIUS`):   (, 50 ).
- **:**        (  `Type_ID`  `blueprints.toml`).  ,      .
- **:** O(K) -      Spatial Grid,    N .

### 4.2.  2:  (Weighting)

    (`V_attract`). CPU-only, f32 .

```rust
// baking_compiler/src/cone_tracing.rs

pub fn calculate_v_attract(
    head_pos: Vec3,       // f32,    
    forward_dir: Vec3,    // f32,   
    fov_cos: f32,         // cos(FOV / 2),   
    spatial_grid: &Grid,  //   X|Y|Z PackedPosition
) -> Vec3 {
    let mut v_attract = Vec3::ZERO;
    let mut total_weight = 0.0;

    // 1.     : O(K),  O(N)
    //   OOB: Grid   clamp(0, MAX)   
    let candidates = spatial_grid.get_in_radius_safe(head_pos, MAX_SEARCH_RADIUS);

    for target in candidates {
        let dir = target.pos - head_pos;
        let dist_sq = dir.length_squared();
        if dist_sq > MAX_RADIUS_SQ { continue; }  //  sqrt   

        let dist = dist_sq.sqrt();
        let dir_norm = dir / dist;

        // 2.  : Dot Product ( atan2/sin/cos)
        if forward_dir.dot(dir_norm) >= fov_cos {
            //    =  
            let weight = target.attraction_gradient / (dist_sq + 1e-5);
            v_attract += dir_norm * weight;
            total_weight += weight;
        }
    }

    // 3.   
    if total_weight > 0.0 {
        (v_attract / total_weight).normalize()
    } else {
        forward_dir  //     
    }
}
```

### 4.3.  3:    (Steering & Packing)

   -      :

```
V_next = Normalize(V_global * W_inertia + V_attract * W_sensor + V_noise * W_jitter)
```

|  |  |  |
|---|---|---|
| `V_global` (Goal) |     ( Z /  ) | ,      |
| `V_attract` (Local) |     (4.2) |   (tortuosity) |
| `V_noise` (Jitter) |  (`WyHash64(master_seed + axon_id + step)`) |     |

- ** (`W_*`):**   `anatomy.toml`.  6080%.

**    (CPU Night Phase):**

   f32,      `u32` (PackedPosition).  f32       drift .

```rust
// baking_compiler/src/axon_growth.rs

pub fn step_and_pack(
    current_f32_pos: &mut Vec3,   // f32  -  drift
    v_global: Vec3,
    v_attract: Vec3,
    v_noise: Vec3,
    weights: &SteeringWeights,
    owner_type_mask: u8,          // 4  (Geo|Sign|Variant)
    segment_length_voxels: f32,   // 2.0 (50  / 25 )
) -> u32 {
    // 1. Steering (f32)
    let v_steer = (v_global * weights.global
                 + v_attract * weights.attract
                 + v_noise * weights.noise).normalize_or_zero();

    // 2. Step
    *current_f32_pos += v_steer * segment_length_voxels;

    // 3. Quantize  voxel coords (  )
    let x = (current_f32_pos.x.round() as u32).min(1023); // 10 
    let y = (current_f32_pos.y.round() as u32).min(1023); // 10 
    let z = (current_f32_pos.z.round() as u32).min(255);  // 8 
    let t = (owner_type_mask & 0x0F) as u32;               // 4 

    let packed = (t << 28) | (z << 20) | (y << 10) | x;

    // 4.   Stagnation (  )
    //       (   ),
    //   ,        .
    assert!(packed != previous_packed_pos, "Stagnation detected: axon stalled at boundary");
    
    packed
}
// let new_pos = step_and_pack(...);
// if new_pos == last_pos { axon.ttl = 0; } // Stop growth
// axon_geometry.push(new_pos);
```

- **4   .** 1M  = 4   Read-Only/Texture Memory.
- **Type  :** `seg_val >> 28`         .

### 4.4. : Smart Wiring

   ,  :

1. ** :**    ,    `V_global` ( ).
2. **  :**    , ,    ,  Overlap Probability  .
3. ** :** ,    ,   ,   (   ) -        .

