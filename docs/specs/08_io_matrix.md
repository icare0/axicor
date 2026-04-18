# 08.  I/O 

>   [Axicor](../../README.md).   /   .

---

## 1. 

          **** - 2D-   WH.

- ** ** -   =    
- ** ** -   =    
- ** ** -    A    (ghost axons)  B

    :   ID,   `matrix[y * W + x]`.

> **: Baking Freeze.**  I/O ( ,  ,  ) **  Baking**.  ,      runtime.  =  Baking.

1.1.      (EnvRX / EnvTX)
   (Node Editor)          -:
*   **EnvRX ( / World Input):**    .    -.  : **   ** ( ).
*   **EnvTX ( / World Output):**    .    -.  : **   ** ( ).
*   **Shard ():** - .     .

1.2.    (C-ABI Alignment)
  100% Coalesced Access  GPU      PCIe, I/O       (Padding):
*   ** (Input_Bitmask):**  **64-  (8 )**.      ,         8.  ,  CPU  GPU      .
*   ** (Output_History):**  **64-  (L2 Cache Line)**.    (`u8`,  1  = 1 )  ,      64 .           ,   - .

---

## 2.   (Virtual Axons)

### 2.1. 

 WH ****  X-Y  .       :

```
region_x = pixel_x * (zone_width  / W)
region_y = pixel_y * (zone_depth  / H)
```

  =   .       -     ( ,  ,  ).

### 2.2. Spawning  Z

  ****   (x, y)     Z:

```toml
# io.toml
[[input]]                 #  
name = "retina"           #  
zone = "SensoryCortex"    # ,    
width = 64                #    
height = 64               #    
entry_z = "top"           # "top" | "mid" | "bottom" |    um
target_type = "All"       # "All" |   
growth_steps = 1500       #     ( Cone Tracing)
empty_pixel = "skip"      # "skip" | "nearest" -         target_type
stride = 1                #  N- : 0 =  ( 0 ), 1 =  , 2 =  2-, ...
```

 `entry_z`:
- `"top"` - Z_max ( ,  thalamo-cortical)
- `"mid"` - Z_mid (    )
- `"bottom"` - Z_min
-  -     (um)

 `empty_pixel`:
- `"skip"` -       ( ID  )
- `"nearest"` -    target_type   

### 2.3. 

     **   ** -     Cone Tracing    :

-     `growth_steps`   `[[input]]` (io.toml)
- Cone Tracing   -     ,  .   (cone angle, steering weight,  ..)   `blueprints.toml`,  `[virtual_axon_growth]`.      `[cone_tracing]`,       
-   `target_type`   (steering    )
-      -  

> **:**   =    `axon_heads[]` (4 ). `PropagateAxons` =  `IADD`   -    .      `  128`,    .  -     4   .

> **Night Phase:**   **  pruning**.   . ,    ,    (pruning  ),         . Baker     (`is_virtual = true`), Night Phase  .

:    = ,  ** **        .

### 2.4. Spatial Mapping (UV-)
    UV-     ( ).    /    ,      (,        ).

: `uv_rect = [u_offset, v_offset, u_width, v_height]`
    `[0.0, 1.0]`. : `[0.0, 0.0, 1.0, 1.0]` ( ).

** Inverse UV Projection ( Baker):**
1.       : `u_vox = vx / zone_width_vox`.
2.   (Early Exit),  `u_vox < u_offset`  `u_vox >= u_offset + u_width`.
3.      `local_u = (u_vox - u_offset) / u_width`.

**  Runtime:**
 UV-  **   ** (Baking Phase), HFT- (Day Phase)      `mapped_soma_ids`      . Zero-Cost   .

### 2.5.  

   -    :

```
Input_Bitmask[tick][pixel_id / 64] - 64-  (  2x u32 )
pixel_id = matrix_offset + y * W + x
```

*       : `((W * H + 63) / 64) * 8`.*

  =  `axon_heads[virtual_offset + pixel_id] = 0` ( ).      -  ,  , GSOP .

### 2.6. Bulk DMA & Stride (Autonomous Batch Execution)

**Bulk DMA  (. [06_distributed.md 2.9](./06_distributed.md)):**

  ** **  . ** **   VRAM **  ** `cudaMemcpyAsync`    (<1   PCIe 4.0 x16). GPU  **6- Autonomous Loop**    ,    (`tick_input_ptr`)    **O(1)**        .

**Stride Parameter (Intra-Batch Frequency):**

 `stride`      :

```
stride = 0     0  (, Single-Tick Pulse)
stride = 1      ()
stride = 2     2- 
stride = N     N- 
```

 `stride = S`,  `InjectInputs`       S- .     :
$$\text{effective\_ticks} = \lceil \text{sync\_batch\_ticks} / S \rceil$$

 `input_bitmask_buffer`:
$$\text{size} = \lceil \text{total\_virtual\_axons} / 32 \rceil \times 4 \times \text{effective\_ticks}$$

**:**
- `stride = 0, sync_batch_ticks = 100`: 1  (t=0), buffer = `N/32  4  1`  -  ****   
- `stride = 1, sync_batch_ticks = 100`: 100  (), buffer = `N/32  4  100`  - sensor data  
- `stride = 2, sync_batch_ticks = 100`: 50 , buffer = `N/32  4  50`  - downsampling  2- 

**Early Exit:**      `InjectInputs`   128  .  FLOPS.  .

** Bulk:**  DMA   (100 ms): H2D input, H2D schedule, D2H output, D2H activity.     .   .

### 2.7.   (C-ABI)

    ()      UDP.     20- Little-Endian  `ExternalIoHeader`.  JSON  Protobuf.

```cpp
//  20 . C-ABI .
#[repr(C)]
pub struct ExternalIoHeader {
    pub magic: u32,         // 0x4F495347 ("GSIO")  , 0x4F4F5347 ("GSOO")  
    pub zone_hash: u32,     // FNV-1a    (, "SensoryCortex")
    pub matrix_hash: u32,   // FNV-1a  I/O  (, "cartpole_sensors")
    pub payload_size: u32,  //      (   )
    pub global_reward: i16, // [DOD] R-STDP Dopamine Modulator (-32768..32767)
    pub _padding: u16,      //    20 
}
```

: UDP   65507  (MTU)   ( EMSGSIZE  ).

### 2.8.   Feature Pyramid Batching ()

      .   - **Feature Pyramid**,     (, , )     ().

**:**
- ** 0:**    "" (Edges).
- ** 1:**    (Color).
- ** 2:**  .
- ** 3..N:**  (Propagation Tail).

      -       . GSOP ()          .

### 2.9.  : CartPole

   `CartPole-v0` (OpenAI Gym):    . 4  , 2  .

**  (SensoryCortex):**

4    ** ** (Gaussian population code):
- `cart_position`  [-2.4, 2.4]
- `cart_velocity`  [-3.0, 3.0]
- `pole_angle`  [-0.41, 0.41] ( 12)
- `pole_angular_velocity`  [-2.0, 2.0]

   **16 ** (tuning width  = 0.15):

```python
# Encoding ()
def encode_variable(val: float, bounds: list, num_neurons: int) -> list:
    """Gaussian receptive fields,    ."""
    val_norm = (val - bounds[0]) / (bounds[1] - bounds[0])  #   [0, 1]
    centers = [i / (num_neurons - 1) for i in range(num_neurons)]
    sigma = 0.15
    spikes = []
    for center in centers:
        distance_sq = (val_norm - center) ** 2
        prob = math.exp(-distance_sq / (2 * sigma ** 2))
        spike = 1 if prob > 0.5 else 0  # 
        spikes.append(spike)
    return spikes

# : 4   16  = 64  
```

`io.toml`:
```toml
[[input]]
name = "cartpole_sensors"
zone = "SensoryCortex"
width = 4
height = 4
# :  44 = 16 ,
#     4  ( , 1 )
# :  -  0-15   
virtual_axon_count = 64
```

**  (MotorCortex):**

2  (/ )  **population decoding**:

```python
# Decoding ()
def decode_action(output_history: bytes, batch_ticks: int) -> int:
    """Winner-takes-all   ."""
    # output_history - concat ,  = sum(WH) / 8  batch_ticks
    #  2  88 = 64  per action
    
    total_spikes = sum(output_history)
    left_spikes = sum(output_history[:len(output_history)//2])
    right_spikes = total_spikes - left_spikes
    
    return 0 if left_spikes > right_spikes else 1  # action: left=0, right=1
```

`io.toml`:
```toml
[[output]]
name = "motor_left"
zone = "MotorCortex"
width = 8
height = 8
target_type = "Excitatory"     # only excitatory neurons

[[output]]
name = "motor_right"
zone = "MotorCortex"
width = 8
height = 8
target_type = "Excitatory"
```

** ():**

```python
INPUT_PORT = 8081
OUTPUT_PORT = 8082
SYNC_BATCH_TICKS = 100

while True:
    # 1.    CartPole
    cart_x, cart_v, pole_a, pole_av = env.step(action)
    
    # 2.   
    spikes = (
        encode_variable(cart_x, [-2.4, 2.4], 16) +
        encode_variable(cart_v, [-3.0, 3.0], 16) +
        encode_variable(pole_a, [-0.41, 0.41], 16) +
        encode_variable(pole_av, [-2.0, 2.0], 16)
    )
    
    # 3.   (100 )
    bitmask = pack_ticks(spikes, sync_batch_ticks=100)
    send_to_runtime(INPUT_PORT, bitmask)
    
    # 4.  
    output_history = receive_from_runtime(OUTPUT_PORT)
    
    # 5.  
    action = decode_action(output_history, batch_ticks=100)
```

     : **100 ms   10  **.     CUDA-   .

---

## 3.   (Soma Readout)

### 3.1. 

 :  WH   X-Y  .     .

### 3.2.   

      **M  **  :

```toml
[[output]]
name = "motor_left"
zone = "MotorCortex"
width = 8
height = 8
target_type = "Excitatory"

[[output]]
name = "motor_right"
zone = "MotorCortex"
width = 8
height = 8
target_type = "Excitatory"

[[output]]
name = "attention_map"
zone = "MotorCortex"
width = 32
height = 32
target_type = "All"
```

> **:   =  **  `[[output]]`  `io.toml`. Baker  offset'  `.gxo` .

### 3.3.  

   ()   **  **.  - **-** (master_seed + output_name + pixel_index):

```
candidates =  target_type   
seed = master_seed ^ fnv1a(output_name) ^ pixel_index
chosen_soma = candidates[hash(seed) % len(candidates)]
```

 1   1 . ,   Baking.

### 3.4.  

 = **  **  .     1  (`u8`).

**VRAM Layout (GPU):**
   `RecordReadout`     `[Tick][Pixel]`:
```cpp
Output_History[tick][pixel_id] = is_spiking;
```

**Network Layout (Zero-Copy Transpose):**
   `[Tick][Pixel]`   ,  (Python)    `for`,      .    (Cache Miss).    UDP- Rust-      : **[Tick][Pixel]  [Pixel][Tick]**.

 , Python-        .     `memoryview.reshape((N, Batch))`  **O(1)**      .

: `total_output_pixels  sync_batch_ticks` .

   `RecordOutputs`  `flags[soma_id] & 0x01`       .

### 3.5.  

 `Output_History` -   2D-    .    - **  Hub'**:

- **Population Coding**: `popcount(pixels_group)`   
- **Rate Coding**:       N 
- **Spatial Pattern**:     

Axicor   .     .

---

## 4.   (Ghost Axon Matrix)

### 4.1. 

  -  **   **:   -     (ghost axons)  -.

```toml
# brain.toml
[[connection]]
from = "SensoryCortex"
to = "HiddenCortex"
output_matrix = "sensory_out"    #    -
width = 16                       #    -
height = 16
entry_z = "top"
target_type = "All"
growth_steps = 750
```

 :
1.  `SensoryCortex`  `[[output]]`  `name = "sensory_out"` ( 3232)
2. Connection ****      `HiddenCortex`  ghost- 1616
3. Baker  256 ghost axons  `HiddenCortex`,   Cone Tracing
4. Runtime : `output_pixel[i]`  SensoryCortex  `ghost_axon_head[i]`  HiddenCortex

> **:**    (3232)   (1616),  ghost-  ****   (pooling).   - upsampling.  = .

### 4.2.  ghost axons   virtual axons

|  | Virtual Axon () | Ghost Axon () |
|----------|---------------------|---------------------------|
|  |  (Input_Bitmask) | Runtime (sync axon_heads) |
|   |   (UDP) |     |
|  | `cudaMemcpyAsync`  | D2H + H2D (4   N) |
| Night Phase |  pruning |  pruning |
| `soma_idx` | `usize::MAX` ( ) | `usize::MAX` ( ) |

 .    **   **.

---

## 5. 

            (    ).        -   .

---

## 6. 

|  |  (GXI) |  (GXO) | Ghost |
|----------|------------|-------------|-------|
|  | 2D  WH | 2D  WH | 2D  WH |
| - |  |  |  () |
|   |   |   | Ghost  |
|  | Cone Tracing | Seeded  | Cone Tracing |
|  | 4 / | 0 ( ) | 4 / |
| Night Phase |  pruning | N/A |  pruning |
|  | / (in) | u8/ (out) | Sync axon_head |
|   | `.gxi` | `.gxo` | `.ghosts` |

 .   Baking.  .

---

## 7.  (UDP && VRAM)

        .   (warning)   (drop).

|  |  |  |  |  |
|----------|---------|---------|---------|---------|
| **Max UDP Input** | `total_virtual_axons / 32  4  effective_ticks` | < 65507 | IP/UDP MTU |     TCP/SHM |
| **Max UDP Output** | `(WH   [[output]])  sync_batch_ticks` | < 65507 | IP/UDP MTU |      |
| **Input Bitmask Buffer** | `virtual_axon_count / 32  4` (per tick) | 256   | VRAM (input_bitmask) |  total_virtual_axons |
| **Output History Buffer** | ` mapped_soma_ids  sync_batch_ticks` | 256   | VRAM (output_history) |     batch_ticks |
| **Ghost Schedule Buffers** | 2  (max spike_events  1 ) | ~4  | Ring Buffer pinned |     ghost_density |

**:**

- **CartPole (4 inputs 64 neurons)**: Input size = 8 bytes/tick  100 ticks = 800 B [OK] OK
- **Retina (19201080)**: Input size = 64 /tick (  8M ) [OK] OK (64  < 65.5 )
- **4 zones  64 ghost spikes/batch**: Schedule buffer ~256   ring [WARN]   
- **Output: 8 matrice  256256**: Output size = 128 /batch [ERROR] Overflow -    resolution

  ****  `simulation.toml` (VRAM params)  `brain.toml` (virtual_axon_count, ghost_density).

---

## Connected Documents

| Document | Purpose | Status |
|----------|---------|--------|
| [05_signal_physics.md](05_signal_physics.md) | RecordReadout kernel, Input spike injection, Output readout kernels | [OK] MVP |
| [06_distributed.md](06_distributed.md) | BSP sync, output_history aggregation across shards |  MVP |
| [07_gpu_runtime.md](07_gpu_runtime.md) | VramState I/O fields, ExternalIoServer UDP | [OK] MVP |
| [02_configuration.md](02_configuration.md) | io.toml [[input]] / [[output]] schema |  TODO |
| [04_connectivity.md](04_connectivity.md) | Ghost axon placement via Cone Tracing |  TODO |
| [09_baking_pipeline.md](09_baking_pipeline.md) | .gxi / .gxo / .ghosts file format |  TODO |

---

## Changelog

**v1.0 (2026-02-28)**

- Added UDP protocol details (**2.7**): ExternalIoHeader struct, zone_hash/matrix_hash computation (FNV-1a), port conventions, fragmentation behavior
- Added CartPole real example (**2.9**): 4-input Gaussian population coding (16 neurons per variable), 2-output population decoding, TOML config, host synchronization pattern
- Added Constraints table (**7**): UDP payload limits (65507 B), VRAM buffer examples (CartPole, Retina, 8-output), configuration paths
- Clarified stride formula impact on bitmask size (**previous 2.6**): effective_ticks = sync_batch_ticks / stride
- Verified all sections against 07_gpu_runtime.md ExternalIoServer, CartPole client reference

**Known Issues**

- Ghost axon fragmentation (> 65K schedule events) requires explicit mem allocation; not covered in MVP
- TCP/Shared Memory I/O for bandwidth > 1 Gbps; UDP-only in V1
