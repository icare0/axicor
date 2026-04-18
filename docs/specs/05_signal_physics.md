# 05.    I/O (Signal Physics)

>   [Axicor](../../README.md).   :     .

---

## 1.   (Signal Physics)

         .        (Global Memory Reads).

### 1.0.    (Day Phase Pipeline)

      CUDA kernels.  :  kernel    .

```mermaid
sequenceDiagram
    participant Host
    participant GPU as GPU / CUDA Stream
    
    Host->>GPU: 1. InjectInputs<br/>(  )
    GPU->>GPU:  input_bitmask<br/> axon_heads[virtual_axons] = 0
    GPU-->>GPU: Signal birth: virtual spikes
    
    GPU->>GPU: 2. ApplySpikeBatch<br/>( )
    GPU->>GPU:  ghost_indices   <br/> axon_heads[ghost_axons] = 0
    GPU-->>GPU: Signal birth: network spikes
    
    GPU->>GPU: 3. PropagateAxons<br/>(  )
    GPU->>GPU:  axon_heads ()<br/> head += v_seg<br/>  axon_heads
    GPU-->>GPU:    
    
    GPU->>GPU: 4. UpdateNeurons (GLIF kernel)<br/>(   )
    GPU->>GPU:  voltage, flags, axon_heads, dendrite struct.<br/>:     GLIF leak<br/> columnar dendrite loop  threshold check  fire
    GPU->>GPU: : voltage, flags ( !)<br/>  
    GPU-->>GPU: Soma fires or stays silent
    
    GPU->>GPU: 5. ApplyGSOP ()<br/>(STDP   )
    GPU->>GPU:  flags () + dendrite_timers ()<br/>: potentiation/depression  <br/>  dendrite_weights
    GPU-->>GPU:    
    
    GPU->>GPU: 6. RecordReadout ( )<br/>(  )
    GPU->>GPU:  flags (soma spikes)<br/> output_history[current_tick] = spikes
    GPU-->>GPU:    
    
    Host<<-GPU:   (stream synchronization point)
```

**  (Dependency Chain):**
1. **InjectInputs**   
2. **ApplySpikeBatch**     
3. **PropagateAxons**    
4. **UpdateNeurons**     soma fires    
5. **ApplyGSOP**    (     UpdateNeurons)
6. **RecordReadout**      

** :**
-  4 (UpdateNeurons) ****    5 (ApplyGSOP), .. GSOP       (dendrite timers).
-  3 (PropagateAxons) ****    4 (UpdateNeurons), .. UpdateNeurons    .

**:**
-      CUDA stream (   ).
-   kernel'  soft sync (    ).
-  RecordReadout - : Host       ( ).

---

### 1.1. :   (Burst Train Model)

 -  ,    (Burst),    .

- ** :**    8  `BurstHeads8` (32 , 1 -).
- **:** `v_seg` (  /).
- **Update Loop:**      $h_i$:  $h_i \neq AXON\_SENTINEL$,  $h_i \mathrel{+}= v\_seg$.
- **Active Tail:**  $i$  ,          8 :
  $(h_k - Tail\_Length) \le i \le h_k$

   ,   :   `h7` ,   `h0`  `0`.         .

### 1.2. :  (Inference Pipeline)

     .  **Early Exit**    .

** 1. Refractory Gate (   ):**

```cuda
// Refractory timer - 1   . 32   1  = 32  (1  L1)
u8 timer = refractory_timer[slot * N_padded + tid];
if (timer > 0) {
    refractory_timer[slot * N_padded + tid] = timer - 1;
    return; // Early Exit: ~90%       Global Memory 
}
```

** 2. Overlap Check ( ):**

-  `Axon_Head_Index`    .
- :       Active Tail.
-  **:**
  - `Soma_Voltage += Synapse_Weight` (     warp).
  - `dendrite.timer = const_mem.variants[variant_id].synapse_refractory` (  Constant Memory, 1 ).

* **   (Burst Gating):**      8     (Branchless OR),        **    ** (`synapse_refractory_period`).           ,      ( 1)   .        .

### 1.2.1.   : ApplySpikeBatch

 #2 Day Phase.   `schedule_indices[]`  `SpikeBatch` (. [06_distributed.md 2.5](./06_distributed.md)). Sender-Side Mapping:    `u32`,   ID.

> **Early Exit:**  `num_spikes == 0` (      ),    -  ,   .

```cuda
__global__ void apply_spike_batch_kernel(u32 num_spikes,
                                         const u32* schedule_indices,
                                         u32* axon_heads,
                                         u32 total_axons) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    if (tid >= num_spikes) return;

    // O(1) routing. schedule_indices[tid] =    axon_heads[].
    // Bounds checking (index < total_axons)    CPU 
    //   Map Phase (06_distributed.md 2.8).
    //   0 =  .
    u32 ghost_id = schedule_indices[tid];
    if (ghost_id < total_axons) {
        axon_heads[ghost_id] = 0;
    }
}
```

  ****     (`Is_Spiking == true`).    (  5  ?),  **  ** Active Tail.

- **:**           ,     . -   ,    .

**Constant Memory:** `AxicorConstantMemory` (. [07_gpu_runtime.md 1.5](./07_gpu_runtime.md)).  array  16 `VariantParameters` structs (       blueprints). Variant ID     `(flags >> 4) & 0xF` ( 4-7 = 16 ).

### 1.3. :  GSOP  

       : ** ** ( )  ** ** (  ).

#### 1.3.1.     (Fixed-Point Downscale)

             .

*   **Mass Domain (i32):**         **2,140,000,000**.  STDP- (GSOP)     .     -,      ,     .
*   **Charge Domain (16-bit shift):**     `UpdateNeurons`      ()   : `int32_t charge = w >> 16;`. 

**:**
1.  ** :**       80 ,    **~0.00002%**   .     .
2.  **  (Silent Synapses):**     (, 30,000),        **0**.    , ,      ,       ""   .

    .      `dist = min(h_k - seg_idx)`.

   (Burst)  8  **  **,     .   **Winner-Takes-All**:

**  R-STDP (Zero-cost Local Trace):**

1. **  .**  128 .

2. **Branchless Unroll (  ):**  32  `BurstHeads8`.  `min_dist`     (`#pragma unroll`),   Warp Divergence.   **  ()** ,  .
   ```cuda
   min_dist = min(min_dist, (d < len) ? d : 0xFFFFFFFF);
   ```

3. ** (Cooling Shift):**     ,     .  : `cooling_shift = dist >> 4` ( 16     2 ).

4. **Asymmetric Dopamine Modulation (D1/D2 Receptors):**       (Dopamine),   `__constant__` .           :
   ```cpp
   int32_t pot_mod = (current_dopamine * p.d1_affinity) >> 7; 
   int32_t dep_mod = (current_dopamine * p.d2_affinity) >> 7;
   
   // D1    , D2     ( )
   int32_t final_pot = max(0, p.gsop_potentiation + pot_mod);
   int32_t final_dep = max(0, p.gsop_depression - dep_mod);
   ```

4.5. **Burst-Dependent Plasticity (BDP):**        `burst_count` ( `[3:1]`  `flags`). 
   - ** :**     (`burst_count = 1`),    .
   - ** :**      (`burst_count > 1`),         ( x7).
   - **Branchless Math:** 
     ```cpp
     //      UpdateNeurons (Zero Warp Divergence)
     //  final_spike == 0,     .
     burst_count = final_spike * (burst_count + (burst_count < 7 ? 1 : 0));
     
     //   ApplyGSOP
     int32_t burst_mult = (burst_count > 0) ? burst_count : 1;
     int32_t delta_pot = (raw_pot * inertia * burst_mult) >> 7;
     ```
        "Near-Zero Economy",     (  ),  BDP-      , -  .

5. ** STDP:**
   -    : `rank = abs(weight) >> 27`.
   -  : `inertia = p.inertia_curve[rank]`.
   -   (Branchless ): `delta = is_active ? ((final_pot * inertia) >> 7) : -((final_dep * inertia) >> 7);`
   -   (): `delta >>= cooling_shift`.

6. **Slot Decay  :**
   -   : `delta = (delta * slot_decay) >> 7;`
   -  : `weight = sign(weight) * clamp(abs(weight) + delta, 0, 32767)`.

** Winner-Takes-All:**   ()    GSOP  .  7   `BurstHeads8`  .           .

** Branchless:**  FPU-,   .    STDP     (`>>`).   `#pragma unroll(8)`         .

### 1.4. Zero-Cost 

|  |  |  |
| :--- | :--- | :--- |
| **Early Exit** |     (`is_spiking == 0`),      (`return`). |  ~99%  GSOP           . |
| **Branchless Math** |  `is_active`    `head - seg_idx < propagation_length`.    -  ALU. |    (Warp Divergence).  32      . |
| **Integer Physics** |    `i16`,     /  saturate clamp `32767`. | 0% Float .    ALU.  FPU  - ALU   . |

**:** Kernel ApplyGSOP   -    ,   ,    128      .  ,   ,   .  Data-Oriented Design.

### 1.5.  : UpdateNeurons (GLIF Kernel)

,       : GLIF leak, , Early Exit,  , threshold check, fire/reset.    `AxicorConstantMemory` (. [07_gpu_runtime.md 1.5](./07_gpu_runtime.md)).

```cuda
__constant__ AxicorConstantMemory const_mem;

__global__ void update_neurons_kernel(
    u32 padded_n,           // Padded neuron count
    i32* voltage,           // Membrane voltage
    i32* threshold_offset,  // Homeostasis offset
    u8* refractory_timer,   // Soma refractory countdown
    u8* flags,              // [31..8] reserved | [7..4] variant | [0] is_spiking
    const u32* soma_to_axon,// Soma  Axon mapping
    const u32* dendrite_targets,  // Packed: [31..10] Axon_ID | [9..0] Segment
    i16* dendrite_weights,
    u8* dendrite_timers,
    u32* axon_heads
) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    if (tid >= padded_n) return;

    // 1.   +   (1  L1)
    u8 f = flags[tid];
    u8 type_mask = f >> 4;
    u8 variant = type_mask & 0xF;   //  4-7 = Variant (16 )
    VariantParameters p = const_mem.variants[variant];

    // 2.  (Soft Limit) -  ,    
    i32 t_off = threshold_offset[tid];
    i32 decayed = t_off - p.homeostasis_decay;
    t_off = decayed & ~(decayed >> 31);       // Branchless max(0, val)

    // 3.   - Early Exit (~90% )
    u8 ref_timer = refractory_timer[tid];
    if (ref_timer > 0) {
        refractory_timer[tid] = ref_timer - 1;
        threshold_offset[tid] = t_off;
        flags[tid] = f & ~0x1;
        return;
    }

    // 4. GLIF:   (Branchless clamp  rest)
    i32 v = voltage[tid];
    i32 leaked = v - p.leak;
    i32 diff = leaked - p.rest_potential;
    v = p.rest_potential + (diff & ~(diff >> 31));  // max(rest, leaked)

    // 5. Columnar Loop: 128   (Coalesced Access)
    for (int slot = 0; slot < 128; ++slot) {
        u32 col_idx = slot * padded_n + tid;

        // 5a. Refractory Gate (Early Exit -  3 global reads)
        u8 d_timer = dendrite_timers[col_idx];
        if (d_timer > 0) {
            dendrite_timers[col_idx] = d_timer - 1;
            continue;  // ~90% :    skip target/head/weight
        }

        // 5b.   - BREAK,  continue!
        // :  Night Phase (Baking 4.2 Columnar Defrag)  
        //  (target=0)    .
        u32 target_packed = dendrite_targets[col_idx];
        if (target_packed == 0) break;

        // 5c. Branchless Active Tail Check (8 Heads)
        u32 axon_id = target_packed >> 10;
        u32 seg_idx = target_packed & 0x3FF;
        BurstHeads8 h = axon_heads[axon_id]; // 32-byte coalesced read

        u32 prop = p.signal_propagation_length;
        bool hit = ((h.h0 - seg_idx) < prop) |
                   ((h.h1 - seg_idx) < prop) |
                   ((h.h2 - seg_idx) < prop) |
                   ((h.h3 - seg_idx) < prop) |
                   ((h.h4 - seg_idx) < prop) |
                   ((h.h5 - seg_idx) < prop) |
                   ((h.h6 - seg_idx) < prop) |
                   ((h.h7 - seg_idx) < prop);

        if (hit) {
            // 5d. Voltage accumulation
            i16 w = dendrite_weights[col_idx];
            v += (i32)w;
            dendrite_timers[col_idx] = p.synapse_refractory_period;
        }
    }

    // 6. Threshold Check, DDS Heartbeat & Fire (Branchless)
    i32 eff_threshold = p.threshold + t_off;
    i32 is_glif_spiking = (v >= eff_threshold) ? 1 : 0;
    
    // DDS Phase Accumulator (4 neuron_model.md)
    //   : tid * 104729 ( )
    u32 phase = (current_tick * p.heartbeat_m + tid * 104729) & 0xFFFF;
    i32 is_heartbeat = (phase < p.heartbeat_m) ? 1 : 0;

    //   (  Heartbeat)
    i32 final_spike = is_glif_spiking | is_heartbeat;

    //       GLIF- (Heartbeat   )
    v     = is_glif_spiking * p.rest_potential + (1 - is_glif_spiking) * v;
    ref_timer = is_glif_spiking * p.refractory_period;
    t_off += is_glif_spiking * p.homeostasis_penalty;
    
    //       (  GSOP)
    f     = (f & 0xFE) | (u8)final_spike;

    // 7.      (Burst Shift)
    if (final_spike) {
        u32 my_axon = soma_to_axon[tid];
        if (my_axon != 0xFFFFFFFF) {
            BurstHeads8 h = axon_heads[my_axon];
            h.h7 = h.h6; h.h6 = h.h5; h.h5 = h.h4; h.h4 = h.h3;
            h.h3 = h.h2; h.h2 = h.h1; h.h1 = h.h0; h.h0 = 0;
            axon_heads[my_axon] = h; // 32-byte coalesced write
        }
    }

    // 8.   VRAM
    voltage[tid] = v;
    threshold_offset[tid] = t_off;
    refractory_timer[tid] = ref_timer;
    flags[tid] = f;
}
```

### 1.6.  : PropagateAxons

  ****  (Local + Ghost + Virtual).   `A  N` ().    **** `UpdateNeurons`.

**Sentinel:** `AXON_SENTINEL = 0x80000000`.     Night Phase      . `dist = 0x80000000 - seg_idx` =    `is_active = false`. Overflow  0   ~59.6 .

> **[WARN] Sentinel Refresh:**    `night_interval_ticks = 0` (  ) host  ~50    `axon_heads`   `AXON_SENTINEL`. : [07_gpu_runtime.md 2.2](./07_gpu_runtime.md).

```cuda
#define AXON_SENTINEL 0x80000000u

__global__ void propagate_axons_kernel(u32 total_axons, u32* axon_heads, u32 v_seg) {
    u32 idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= total_axons) return;

    // 100% Coalesced Access. 1  IADD. 0 .
    u32 head = axon_heads[idx];
    if (head != AXON_SENTINEL) {
        axon_heads[idx] = head + v_seg;
    }
}
```

- **  (Temporal Sync):**      0,   `(uint32_t)(0 - v_seg)`.
  -  wrap-around  `u32`,   `0xFFFFFFFF - v_seg + 1`.     `Propagate`   `v_seg`,     `0`.    ,        -  .
  -  : UpdateNeurons (Burst Shift)
  -  : `ApplySpikeBatch` (Ghost )
  -  : `InjectInputs` ( )
- **1   :** `signal_propagation_length < soma_refractory_period`          .

---

### 1.6.     (Sentinel Refresh Safety)

     **    **,   .

#### 1.6.1.   SENTINEL_DANGER_THRESHOLD

   `axon_heads`  ,    :

* ** :**   `axon_heads[id]`   `SENTINEL_DANGER_THRESHOLD` (, `0x70000000`),  ,             (Active Tail).   ** **.

* ** :**  " " ,    ,      `AXON_SENTINEL`     `u32` [1, 4].

* **:**   (Active Tails)         Maintenance,     GPU-.

**:**
```cpp
#define SENTINEL_DANGER_THRESHOLD 0x70000000u

void refresh_axon_sentinels(u32* axon_heads, u32 count) {
    for (u32 i = 0; i < count; ++i) {
        u32 head = axon_heads[i];
        //      
        if (head > SENTINEL_DANGER_THRESHOLD) {
            axon_heads[i] = AXON_SENTINEL;
        }
        //   (head <= threshold)  
    }
}
```

### 1.7. Warp-Aggregated Telemetry (Zero Atomics)

 IDs   ( IDE  )       (`atomicAdd`).  100,000   , 100,000    L2 ,     `out_count`.

 Axicor   **Warp-Aggregated Atomics**,      :

1. **Ballot Sync:**        (`is_spiking`)    `__ballot_sync(0xFFFFFFFF, is_spiking)`.

2. **Population Count:**   (lane 0)          `__popc(active_mask)`.

3. **Single Atomic:**      `atomicAdd`   ,   `offset`   .

4. **Shuffle Sync:**   `offset`    `__shfl_sync`.

5. **Write:**     ID   .

```cuda
// axicor-compute/src/cuda/telemetry.cu

__global__ void record_spikes_aggregated(
    const u8* soma_flags,           // [padded_n]
    u32* out_spike_ids,             // (ring buffer)
    u32* out_count,                 // Shared atomic counter
    u32 padded_n
) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    u32 lane = threadIdx.x % 32;
    
    // Step 1: Ballot -      
    bool is_spiking = (tid < padded_n) && (soma_flags[tid] & 0x80);
    u32 active_mask = __ballot_sync(0xFFFFFFFF, is_spiking);
    
    // Step 2:   (lane 0)     
    u32 warp_pop = (lane == 0) ? __popc(active_mask) : 0;
    
    // Step 3:    atomicAdd   
    u32 warp_offset = 0;
    if (lane == 0) {
        warp_offset = atomicAdd(out_count, warp_pop);
    }
    
    // Step 4:  offset   
    warp_offset = __shfl_sync(0xFFFFFFFF, warp_offset, 0);
    
    // Step 5:      ID ( )
    if (is_spiking) {
        u32 local_rank = __popc(active_mask & ((1u << lane) - 1));  // Count popcount before me
        out_spike_ids[warp_offset + local_rank] = tid;
    }
}
```

**:**         **32 **.  1024  (32 ) 99%   ,  1000+ atomicAdd    32  (   ).    ,  HFT-.

---

## 2.   (Input Interfaces)

   ** ** (Virtual Axons).      (`io.toml`), ..       .

### 2.1. : Spatial Input Maps

  -   ,    (Feature Extraction).     **Input Map** - 2D- ,      .

- ** :**      N    (6464, 3232, 12816).  -   .
- **target_type:**        `blueprints.toml` ( `"ALL"`  ). Baker      .

#### Baking:   Seeded Hash

    :

1.   : `region_xy = pixel_xy * (zone_size / map_size)`.  Z  .
2.   ( `target_type`  ).
3.  : `seed = master_seed ^ fnv1a(input_name) ^ pixel_idx`, `chosen = candidates[hash(seed) % len]`.
4.   retry ( ): `seed' = ... ^ (pixel_idx + attempt)`.
5. ****     `target_type` (velocity, propagation_length, steering).     ,    -   1- .
6.    ** **:        ,      seed (- ).

#### Multi-Shard

-       `{shard_name}.gxi`.
- Baker     .
- GhostPacket      ( ).
- Host           .

### 2.2.  : Single-Tick Pulse

     bitmask. :

- **:**    = 1  ** 1 **. `InjectInputs`  `axon_heads[id] = 0` ( ).
- **Virtual Refractory:**  `signal_propagation_length`      . ** ** - runtime  .
- **:**        ( , 1.2).

> **[WARN]    = 1   !** InjectInputs    head = 0,     (    0-1).

### 2.3.   (Feature Pyramid Batching)

 (, 30 fps)    **100+  ** (edges, color, motion, corners, ...).   -   .   : **1  = 1 **.

```
 30fps    100+ 
  batch[0]  = edges_mask         ( 0)
  batch[1]  = color_mask         ( 1)
  batch[2]  = motion_left_mask   ( 2)
  ...
  batch[99] = corners_mask       ( 99)
```

:    ****,    .   -  (  ).     -    .

Host     DMA. Runtime    `batch[tick_in_batch]`.

### 2.4.  : DMA Bitmask Injection + InjectInputs Kernel

**Host (CPU):**     `Input_Bitmask` (1  = 1  ).      flat .        .

**Transfer:** `cudaMemcpyAsync(Input_Bitmask_GPU, Input_Bitmask_Host, size, stream)`   - .

**Kernel InjectInputs ( #1 Day Phase):**

```cuda
__global__ void inject_inputs_kernel(
    u32 *axon_heads,                    // Soma axons to inject into
    const u32 *input_bitmask,           // Dense bitmask from host
    const u32 *map_pixel_to_axon,       // Pixel  Axon ID mapping
    u32 num_pixels,                     // Pixels in this shard
    u32 pixel_offset,                   // Base offset in global pixel counting
    u32 total_num_pixels,               // Total pixels across all shards
    u32 tick_in_batch,                  // Current position in batch
    u32 input_stride,                   // Stride: only run every N ticks
    u32 max_ticks_in_batch              // Batch size
) {
    // Early exit if this tick is not an input tick (input_stride can throttle)
    if (input_stride == 0) return;
    if (tick_in_batch % input_stride != 0) return;

    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    if (tid >= num_pixels) return;

    // Calculate effective tick index within bitmask data
    u32 effective_tick = tick_in_batch / input_stride;
    if (effective_tick >= max_ticks_in_batch) return;

    // Dense packing: words_per_tick = ceil(total_num_pixels / 64) * 2
    // All shards share the same total but have offset within global pixel numbering
    u32 words_per_tick_total = (total_num_pixels + 63) / 64 * 2;
    u32 tick_offset = effective_tick * words_per_tick_total;
    
    // Absolute bit position in global numbering + local offset
    u32 abs_bit = pixel_offset + tid;
    u32 word_idx = abs_bit / 32;
    u32 bit_in_word = abs_bit % 32;

    // Broadcast-read: 32 threads in a warp read same u32 from mask
    u32 mask = input_bitmask[tick_offset + word_idx];
    u32 is_active = (mask >> bit_in_word) & 1;

    // Conditional write: Signal birth (axon_heads[id] = 0)
    if (is_active) {
        axon_heads[map_pixel_to_axon[pixel_offset + tid]] = 0;
    }
}
```

**:**
- `input_stride`:  `= 2`, kernel    (   ).     .
- `words_per_tick_total`:   -      .  `N=64K`   `2K`  u32 per tick.
- **Multi-Shard:**     `pixel_offset` ( ).     ,     .

**  :**
- 1 broadcast-read = 32   1  (   L1).
-    `is_active = 1` (~5-10%   ).
-    atomics.

---

## 3.   (Readout Interface)

   **Direct Memory Access**    .    .

### 3.1. :    

  `io.toml`    (Volume Slice).   .

**Baking Phase (Compiler Tool):**

1.    2D- ( ).
2.           ( ).
3. **  (Z-Sort):**
   -     Z.
   -    ** Z** (    / Z0).
   -   (    ,     ).

**:**   `Map_Channel_to_SomaID[]`.  =    .  1--1,  .

### 3.2. :  (Batch Accumulation)

       .    VRAM.

- **:** `Output_History[Batch_Size  Num_Channels]` (`u8`) -   .
- **Kernel RecordReadout ( ,  ApplyGSOP -  kernel ):**

```cuda
__global__ void record_readout_kernel(
    const u8* flags,                    // Soma spike flags
    const u32* mapped_soma_ids,         // [channel]  soma_id mapping
    u8* output_history,                 // Output buffer
    u32 total_mapped_somas,             // Number of output channels
    u32 current_tick_in_batch,          // Current batch position
    u32 padded_n                        // VRAM padding (for bounds checking)
) {
    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    if (tid >= total_mapped_somas) return;

    // O(1) channel  soma mapping (baked during initialization)
    u32 soma_id = mapped_soma_ids[tid];
    u8 is_spiking = 0;
    
    // Safety bounds check
    if (soma_id < padded_n) {
        is_spiking = flags[soma_id] & 0x01;
    }

    // Coalesced Write: threads 0..31 write consecutive u8 values
    // Stride = 32-thread groups per warp for maximum bandwidth
    output_history[current_tick_in_batch * total_mapped_somas + tid] = is_spiking;
}
```

** u8,   ?** Bit-packing   `atomicOr`   -    race conditions. `u8`   = 32    32      (  ).

****  1--1 (Z-Sort  3.1).   ,  need  atomics.

**:**  `mapped_soma_ids[]`    Baking phase (. [09_baking_pipeline.md](./09_baking_pipeline.md)).  soma IDs   output channel.    .

### 3.3. :  (Flush)

   `sync_batch` (    ):

1. **Transfer:**  `cudaMemcpy`   `Output_History`  RAM .
2. **Latency:**          (, 10 ).
   - ** :**        1020 .   .

### 3.4. : Population Coding (External Hub)

  Hub   `output_history[Time  Channels]` (`u8`: 0  1).

**: Population Coding ( Rate Coding / ).**

    ****  .         .     GSOP   .

|  |  |  |
|---|---|---|
|   | 100   23  | `23 / 100 = 0.23` |
|   | 100   87  | `87 / 100 = 0.87` |

**  Hub:**

```
//   (ESP32 / Teensy / STM32):
let active = popcount(population_mask);   // 1 
let strength = active as f32 / population_size as f32;  // 0.0 .. 1.0
set_motor_output(strength);               //   
```

- ** :**    N     - `popcount`     .
- **:**     =    .
- ** :**           Input     L4 .
- **:** , ,   -   (  ).

### 3.5. Moving Window:   (Control FPS)

`output_history` -   `[sync_batch_ticks  num_channels]`. Hub ** **     .

**  :**

```
window_size = tick_duration_us  sync_batch_ticks / (1_000_000 / target_fps)

// : tick=100, batch=100   = 10 
// target_fps = 60  window = 10.000 / 16.667  6  / 
// target_fps = 120  window = 10.000 / 8.333  1-2  / 
```

Hub  `output_history`      `window_size`       `target_fps`.   `io.toml`.

> ** (Biceps / Triceps  ..):**    -  Hub-,  Axicor-. : `signal = Strength(A) - Strength(B)`    =   .    GSOP    .

---

## 4. Night Phase:  (Maintenance & Synaptic Pruning)

 (      )  **Night Phase** -  .  :

1. **Synaptic Pruning:**    (  `|weight| < threshold`).
2. **Columnar Defrag:**  ,   ()    ,   -  .
3. **Sentinel Refresh:**   `axon_heads`   `AXON_SENTINEL` (   ~60+   ).

### 4.1. Sort & Prune Kernel

    Night:        .

```cuda
#define MAX_DENDRITE_SLOTS 128

struct alignas(8) DendriteSlot {
    u32 target_packed;   // [31..10] Axon_ID | [9..0] Segment
    i16 weight;
    u8 timer;
    u8 _pad;
};

__global__ void sort_and_prune_kernel(
    u32 padded_n,                      // Padded neuron count
    u32 *dendrite_targets,             // Columnar: [slot][neuron]
    i16 *dendrite_weights,             // Columnar: [slot][neuron]
    u8 *dendrite_timers,               // Columnar: [slot][neuron]
    i16 prune_threshold                // Minimum |weight| to keep
) {
    // 32  per block (1 warp). Shared memory = 32  128  8 = 32KB.
    __shared__ DendriteSlot smem[32][MAX_DENDRITE_SLOTS];

    u32 tid = blockIdx.x * blockDim.x + threadIdx.x;
    int lane = threadIdx.x;
    bool active = (tid < padded_n);

    // 1. Coalesced Read  Global  Shared Memory
    for (int slot = 0; slot < MAX_DENDRITE_SLOTS; ++slot) {
        if (active) {
            u32 idx = slot * padded_n + tid;
            smem[lane][slot].target_packed = dendrite_targets[idx];
            smem[lane][slot].weight = dendrite_weights[idx];
            smem[lane][slot].timer = dendrite_timers[idx];
        }
    }
    __syncthreads();

    if (active) {
        // 2. Sequential Insertion Sort  |weight|   
        // Empty slots (target_packed == 0)   
        for (int i = 1; i < MAX_DENDRITE_SLOTS; ++i) {
            DendriteSlot key = smem[lane][i];
            i16 key_abs = (key.weight >= 0) ? key.weight : -key.weight;
            int j = i - 1;

            while (j >= 0) {
                i16 w_j = smem[lane][j].weight;
                i16 abs_j = (w_j >= 0) ? w_j : -w_j;

                bool key_is_empty = (key.target_packed == 0);
                bool j_is_empty = (smem[lane][j].target_packed == 0);

                bool should_swap = false;
                if (j_is_empty && !key_is_empty) {
                    should_swap = true;  //  slot  
                } else if (!j_is_empty && !key_is_empty) {
                    if (key_abs > abs_j) {
                        should_swap = true;  //     
                    }
                }

                if (should_swap) {
                    smem[lane][j + 1] = smem[lane][j];
                    j = j - 1;
                } else {
                    break;
                }
            }
            smem[lane][j + 1] = key;
        }

        // 3. Pruning:    |weight| < threshold
        for (int slot = 0; slot < MAX_DENDRITE_SLOTS; ++slot) {
            i16 w = smem[lane][slot].weight;
            i16 abs_w = (w >= 0) ? w : -w;
            if (abs_w < prune_threshold) {
                smem[lane][slot].target_packed = 0;  //   
            }
        }
    }
    __syncthreads();

    // 4. Coalesced Write  Shared  Global Memory
    for (int slot = 0; slot < MAX_DENDRITE_SLOTS; ++slot) {
        if (active) {
            u32 idx = slot * padded_n + tid;
            dendrite_targets[idx] = smem[lane][slot].target_packed;
            dendrite_weights[idx] = smem[lane][slot].weight;
            dendrite_timers[idx] = smem[lane][slot].timer;
        }
    }
}
```

**  Sort & Prune:**
-  0..K-1   ,   `|weight|`   .
-  K..127    (`target_packed = 0`).
- Day Phase  : `if (target == 0) break`  O(K)  O(128).

**:**
- Per-thread Insertion Sort: $O(K^2)$   , $O(K)$   (    ).
- Per-neuron: ~10  128  (CPU Shared Memory, ).
-    :    ~100K .

---

##  

|  |   |
|---|---|
| [01_foundations.md](./01_foundations.md) | 1: Grundlagen  (GLIF ), Spike definition |
| [03_neuron_model.md](./03_neuron_model.md) | 2: VariantParameters, threshold, refractory_period, GSOP parameters |
| [04_connectivity.md](./04_connectivity.md) | 1.2: Dendrite topology, synapse mapping, columnar layout |
| [06_distributed.md](./06_distributed.md) | 2.5, 2.8: SpikeBatch protocol, Ghost sync, Sender-side mapping |
| [07_gpu_runtime.md](./07_gpu_runtime.md) | 1.5: Constant Memory structure, CUDA stream orchestration |
| [08_ide.md](./08_ide.md) | Visualization: Real-time monitoring of signal propagation, spike heatmaps |
| [09_baking_pipeline.md](./09_baking_pipeline.md) | 4.2: Columnar defrag, Night phase integration, Pruning thresholds |
| [project_structure.md](../project_structure.md) | Overview: Role of Signal Physics in Axicor architecture |

---

## Changelog

|  |  |   |
|---|---|---|
| 2026-02-28 | 2.1 |     (physics.cu, readout.cu, inject_inputs.cu, sort_and_prune.cu):  variant bits (6-7  4-7),   ApplySpikeBatch, UpdateNeurons, PropagateAxons.   CUDA kernel examples  RecordReadout, InjectInputs, SortAndPrune.  Mermaid  Day Pipeline (1.0). |
| TBD | 2.0 |    (30  ) |

