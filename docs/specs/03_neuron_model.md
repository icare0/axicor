# 03.   (Neuron Model)

>   [Axicor](../../README.md).   :  ,  ,   .

---

## 1.    (Placement)

### 1.1.  

- **:**     (Stochastic),    .
- **:**        ().      -     .
- **:**   (`global_density`, . [02_configuration.md 5.2](./02_configuration.md)),        -    .

### 1.2.    (Voxel Grid)

- **:** 1  =  1 .
- **:**   (Baking)    *reject-sampling*    (`Occupancy HashSet`).          -     seed.  - 100 ,     (    ).
- **:**       ID      (Spatial Hashing    ).

### 1.3.    (Packed Position / Dense Index)

    `float`,    .   ID   :

**CPU /   (Baking): Packed Position (`u32`)**

  1   Cone Tracing, Spatial Hash  :

```rust
// : 1  ALU
let type_mask = (packed >> 28) & 0xF;
let z = (packed >> 20) & 0xFF; // Z-:  8  (0..255)
let y = (packed >> 10) & 0x3FF;
let x = packed & 0x3FF;

// :
let packed = (type_mask << 28) | (z << 20) | (y << 10) | x;
```

** (Bit Layout):**

```
u32 packed_position = 0xABCDEFGH
+-  31-28: Type (4 )     [A]        0..15 (16 )
+-  27-20: Z (8 )       [BC]       0..255 (256  = 6.4)
+-  19-10: Y (10 )      [DEF]      0..1023 (1024  = 25.6)
+-  9-0:   X (10 )      [GH]       0..1023 (1024  = 25.6)
```
: 0x3A5C80F0 = Type:3, Z:165, Y:598, X:240

**GPU /   (Hot Loop): Dense Index (`u32`)**

-   `0..N-1`     .
-  `N`    , ** 64** (Warp Alignment) -  100% Coalesced Access.
-  SoA- (`voltage[]`, `flags[]`, `threshold_offset[]`)  Dense Index.
- 4-    `flags[dense_index] >> 4` (. [07_gpu_runtime.md 1.1](./07_gpu_runtime.md)).

**:**  Baking CPU   `DenseIndex  PackedPosition`  `PackedPosition  DenseIndex` (Spatial Hash).  Hot Loop GPU     -    Dense Index.

---

## 2.   (4-bit Typing)

   **4- ** (0..15),          `VARIANT_LUT[16]`  `__constant__`  GPU.

### 2.1.  

`type_mask` (4 ,  4-7  `flags[dense_index]`):

```
:  [7..4]
:  Type (0..15)
```

```rust
// : 1  ALU
let type_mask = flags[dense_index] >> 4;  //  4-7
let params = const_mem.variants[type_mask];  //    LUT
```

  -  **   ** `blueprints.toml`  `neuron_types[0..15]`.

#### 2.1.1.     `flags[dense_index]`

 8      :

|  |  |  |  |  |
|---|---|---|---|---|
| `[7:4]` | `type_mask` | `u4` |    (0..15).      `VARIANT_LUT[3]` (2.2). | [OK]  |
| `[3:1]` | `burst_count` | `u3` | **[BDP]**    (0..7).        . | [OK]  |
| `[0:0]` | `is_spiking` | `u1` | **Instant Spike**. 1 =      . | [OK]  |

** BDP (Burst-Dependent Plasticity):**
 `burst_count`       STDP.    CUDA  **Branchless** (O(1)):
`vram.soma_flags[tid] = (flags & 0xF0) | (burst_count << 1) | final_spike;`

     `0xF1` (`11110001_2`),    `[3:1]`,   `type_mask`    .

**  (1  ALU):**
```cpp
u8 flags_byte = flags[dense_index];
u8 type_mask = flags_byte >> 4;                 //  7-4
u8 burst_count = (flags_byte >> 1) & 0x07;      //  3-1
u8 is_spiking = flags_byte & 0x1;               //  0
```

### 2.2.  LUT (Look-Up Table)

    GSOP-    **  ** ( ),  `type_mask`     `__constant__`  GPU.

```cuda-cpp
// GPU Shader ( )
uint8_t type_mask = f >> 4;                    //  4-7
VariantParameters p = const_mem.variants[type_mask];  //  
int32_t new_threshold = threshold + p.homeostasis_penalty;
```

 ** ** - constant memory    L1.  **16  **.

### 2.3.   (Blueprint-)

  (0..15)    `blueprints.toml`:

```toml
[[neuron_types]]
name = "PyramidL5"                  #   ( )
threshold = 500
rest_potential = -70
leak_rate = 2
homeostasis_penalty = 15
homeostasis_decay = 1
gsop_potentiation = 74
gsop_depression = 2
# ...  6 
```

   `blueprints.toml`   index  `type_mask`.          0..15.

### 2.4.    

 **16  ** (0..15),      GLIF  GSOP:

- **:** 0 -    `[[neuron_types]]`  `blueprints.toml`
- **:** threshold, rest_potential, leak_rate, homeostasis_penalty, homeostasis_decay, gsop_potentiation, gsop_depression, refractory_period, synapse_refractory_period, signal_propagation_length, adaptive_leak_max, adaptive_leak_gain, adaptive_mode, d1_affinity, d2_affinity
- **:**   ,   .      {ExcitatoryFast, InhibitorySlow},  - {Relay, Memory, Burst, Regular}

 Baking    `anatomy.toml`  `type_idx` (0..15).    VRAM `type_idx`   `flags[dense_index] >> 4`.


---

## 3.  (Homeostasis)

  .    ,       .

### 3.1. Hard Limit:  

- **:** `refractory_counter` (`u8`).
- **:**      `refractory_period` ( LUT  Variant ID).   .  `> 0`,     .
- **:**  clamp -     (>500 )      .

### 3.2. Soft Limit:  

       (),   .

- **:** `threshold_offset` (`i32`),   .
- **:** `homeostasis_penalty`  `homeostasis_decay` - ** **  .   LUT  Type ID (2.2, constant memory GPU).
- ** (Integer Math):**
  - **:** `threshold_offset += penalty` (   ).
  - ** :** `threshold_offset = max(0, threshold_offset - decay)` - branchless,  `if`.
  - ** :** `threshold + threshold_offset`.
- **  (Habituation):**   ( )    ,    .       .    .
- **Burst Mode:**     -     (penalty ,    ).        .

---

## 4.   (DDS Heartbeat)

        (Spontaneous Firing Rate),        (GSOP). 

           (   VRAM),   **Direct Digital Synthesis (DDS) / Fractional Phase Accumulator**.

### 4.1.  (Zero-Cost Branchless)

   16-  `heartbeat_m` (0 = ).     :

```cpp
// 104729 -       (Spatial Scattering).
//     .
uint32_t phase = (current_tick * heartbeat_m + tid * 104729) & 0xFFFF;
bool is_heartbeat = phase < heartbeat_m;
```

   **4  ALU** (IMUL, IADD, IAND, ICMP)  **0 **.

### 4.2.   

  (Heartbeat)     GLIF-:

- **   :**    (`axon_heads[my_axon] = 0`).
- **GSOP :**  `is_spiking`   1 (   ).
- **  :** Heartbeat ****  `voltage`, ****  `refractory_timer`  ****  `homeostasis_penalty`.   ,      .

**  (. [05_signal_physics.md 1.5](./05_signal_physics.md))**

```cuda
//   ()
i32 is_glif_spiking = (voltage >= effective_threshold) ? 1 : 0;

//  Heartbeat ()
u32 phase = (current_tick * heartbeat_m + tid * 104729) & 0xFFFF;
i32 is_heartbeat = (phase < heartbeat_m) ? 1 : 0;

//  
i32 final_spike = is_glif_spiking | is_heartbeat;

//     GLIF-
voltage    = is_glif_spiking * rest_potential + (1 - is_glif_spiking) * voltage;
ref_timer  = is_glif_spiking * refractory_period;
threshold_offset += is_glif_spiking * homeostasis_penalty;

//       (  GSOP)
flags = (flags & 0xFE) | (u8)final_spike;
```
