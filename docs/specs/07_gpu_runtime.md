# 07. GPU Runtime (GPU & Storage)

>   [Axicor](../../README.md).     VRAM,  ,    .

---

## 1.     (GPU & Storage)

**:**     (AoS)   .     (SoA)   100% Coalesced Memory Access  GPU    CPU/MCU. 

  **Dual-Backend** :  CUDA   HIP.        (Integer Physics).

### 1.1.  FFI- VRAM (Headerless SoA)

     .   `.state`  `.axons`      (Headerless),   `cudaMemcpyAsync`  `spi_flash_mmap` (ESP32).

**  (The 64-Byte Alignment Rule):**
  `padded_n`  ,  64,  ,     SoA- (4N, 2N, 1N )   64 .          L2 -    padding bytes  .    Coalesced Access  AMD Wavefront (64 )   cache thrashing  L2.

** ShardVramPtrs (    .state ):**

    `N` (`padded_n`,   64).
*   `soma_voltage`       [N]  `i32`   (4N bytes)
*   `soma_flags`         [N]  `u8`    (1N bytes)
*   `threshold_offset`   [N]  `i32`   (4N bytes)
*   `timers`             [N]  `u8`    (1N bytes)
*   `soma_to_axon`       [N]  `u32`   (4N bytes)
*   `dendrite_targets`   [128  N]  `u32` (512N bytes)
*   `dendrite_weights`   [128  N]  `i32` (512N bytes)
*   `dendrite_timers`    [128  N]  `u8`  (128N bytes)

**The 1166-Byte Invariant:** Soma (14) + 128 * (Targets:4 + Weights:4 + Timers:1) = 1166   .

*  Edge (ESP32):   128    32 (WTA Distillation).*

** (.axons ):**
   ,     `A` (`total_axons` = Local + Ghost + Virtual)   `N`.
*   `axon_heads`         [A]  `BurstHeads8` (32A bytes)

** 32-  :** 
 `BurstHeads8`     32 . 8   4  =  32 .         L1      32-  .  Xtensa LX7 (ESP32)   32-       . 32-     .

```cpp
// 32-byte alignment     32-  L1  (1 ).
// alignas(64)     50% VRAM (32     ).
struct alignas(32) BurstHeads8 {
    uint32_t h0; uint32_t h1; uint32_t h2; uint32_t h3;
    uint32_t h4; uint32_t h5; uint32_t h6; uint32_t h7;
};
```

  `soma_flags` (1 ):    .       .

*   `[7:4]` type_mask (u4):    (0..15).    VARIANT_LUT.
*   `[3:1]` burst_count (u3):     Burst-Dependent Plasticity (BDP).
*   `[0:0]` is_spiking (u1):      (1 = fired).

     BDP-: `flags[tid] &= ~0x01;`.

### 1.2.   (VariantParameters)
      constant  GPU (   Flash- MCU)    .
   64      L1 Cache.    Coalesced Access.

```cpp
//  64  (1 - L1)
struct alignas(64) VariantParameters {
    // ===  1: 32-bit ( 0..20) ===
    int32_t threshold;                        // 0..4
    int32_t rest_potential;                   // 4..8
    int32_t leak_rate;                        // 8..12
    int32_t homeostasis_penalty;              // 12..16
    uint32_t spontaneous_firing_period_ticks; // 16..20

    // ===  2: 16-bit ( 20..28) ===
    uint16_t initial_synapse_weight;          // 20..22
    uint16_t gsop_potentiation;               // 22..24
    uint16_t gsop_depression;                 // 24..26
    uint16_t homeostasis_decay;               // 26..28

    // ===  3: 8-bit ( 28..32) ===
    uint8_t refractory_period;                // 28..29
    uint8_t synapse_refractory_period;        // 29..30
    uint8_t signal_propagation_length;        // 30..31
    uint8_t is_inhibitory;                    // 31..32

    // ===  4:  ( 32..48) ===
    uint8_t inertia_curve[16];                // 32..48 (: abs(w) >> 11)

    // ===  5: Adaptive Leak Hardware ( 48..58) ===
    int32_t adaptive_leak_max;                // 48..52
    uint16_t adaptive_leak_gain;              // 52..54
    uint8_t adaptive_mode;                    // 54..55
    uint8_t _leak_pad[3];                     // 55..58

    // ===  6: Neuromodulation & Pad ( 58..64) ===
    uint8_t d1_affinity;                      // 58..59 (  LTP)
    uint8_t d2_affinity;                      // 59..60 (   LTD)
    uint8_t _pad[4];                          // 60..64 ()
};

//   16  ( 1024 )
struct AxicorConstantMemory {
    VariantParameters variants[16];
};
```

   : Variant ID   `soma_flags`  1  ALU: `u8 var_id = (flags[tid] >> 4) & 0xF;`          `const_mem.variants[var_id].threshold;`.

### 1.6. Cross-Platform IPC & Zero-Copy Mmap

**:**  Zero-Copy  (1.4)      .   Linux-exclusive  (`/dev/shm`, Unix Sockets).

**:**  Zero-Copy  (1.4)   Dual-Backend : `nvcc`  NVIDIA  `hipcc`  AMD.         feature- `amd`.

#### 1.6.1. : - 

|  |  (Night Phase) |  (Network) |
|---|---|---|
| **Linux** | POSIX `shm_open()`  `/dev/shm/*.state.shm` | Unix Domain Sockets (**UDS**); Fast-Path UDP  Data Plane |
| **Windows** | File-backed mmap  `%TEMP%`   `*.state.bin.mmap` | TCP/IP   `19000 + (hash % 1000)`  Control & Data Plane |
| **Darwin (macOS)** | POSIX `shm_open()` ( Linux) | UDS + TCP fallback  Legacy Systems |

#### 1.6.2. Page-Aligned Memory Guarantee (4096 bytes)

** C-ABI :**  `mmap`    **4096 ** ( page size  ).

```rust
// generic_ipc.rs
pub fn allocate_shared_memory(size: usize) -> Result<SharedMemoryRegion, CAbiBoundaryError> {
    let aligned_size = (size + 4095) / 4096 * 4096; // Align UP to 4096
    
    #[cfg(target_os = "linux")]
    {
        let shm = unsafe { libc::shm_open(name, libc::O_CREAT | libc::O_RDWR, 0o644) }?;
        let addr = unsafe { libc::mmap(
            std::ptr::null_mut(),
            aligned_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            shm,
            0,
        )};
        // addr guaranteed to be % 4096 == 0
        
        assert_eq!(addr as usize % 4096, 0, "FATAL C-ABI BOUNDARY: mmap address not page-aligned!");
    }
    
    #[cfg(target_os = "windows")]
    {
        let file = File::create(temp_path)?;
        file.set_len(aligned_size as u64)?;
        let handle = unsafe { CreateFileMappingA(file, aligned_size)? };
        let addr = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, aligned_size) };
        
        assert_eq!(addr as usize % 4096, 0, "FATAL C-ABI BOUNDARY: MapViewOfFile address not page-aligned!");
    }
    
    Ok(SharedMemoryRegion { addr, size: aligned_size })
}
```

**:**  `.state`  `.axons`   **    4096 **.

```
Mmap:   [0x10000000 aligned to 4096]
         +---------------------------------+
         | SoA Payload (782B  N)          |
         | + Burst Architecture (32B  A)  |  All @ offset % 4096 == 0
         +---------------------------------+  Also @ offset % 4096 == 0
         [ 4096 , no gaps]
```

#### 1.6.3. Legalized bytemuck::cast_slice (Zero-Copy)

**  :**

1. **Mmap  :**    mmap- = `base_addr + offset`.  `base_addr % 4096 == 0`  `offset % align_of::<T>() == 0`,     .

2. **Baking Tool Determinism (Compile-Time):** Baker ,  SoA-       (`align_of::<T>()`).    `VariantParameters` (align 64)  : `offset % 64 == 0`.

3. **Host-Side Zero-Copy:**

```rust
// memory.rs (Night Phase CPU-side)
let shared_region = allocate_shared_memory(total_state_bytes)?;

// ZERO allocations, ZERO copies
let soma_voltage: &[i32] = unsafe {
    bytemuck::cast_slice(std::slice::from_raw_parts(
        (shared_region.addr + offset_soma_voltage) as *const i32,
        neuron_count,
    ))
};

// This is SAFE because:
// 1. (shared_region.addr + offset_soma_voltage) % align_of::<i32>() == 0 (baker-enforced)
// 2. shared_region.addr % 4096 == 0 (mmap-guaranteed)
// 3. bytemuck::Pod trait ensures no padding, no Option, no refcells
```

**CUDA-Side (Device Pointers):**

```cuda
// kernel.cu
__global__ void example_kernel(CudaShardVramPtrs ptrs) {
    int tid = blockIdx.x * blockDim.x + threadIdx.x;
    
    // ZERO device-side alignment overhead
    int32_t v = ptrs.soma_voltage[tid];  // Guaranteed coalesced access: 32 threads read 128 bytes @ 32-byte boundary
}
```

#### 1.6.4.  FATAL C-ABI BOUNDARY ( )

    **  **      `FATAL C-ABI BOUNDARY`:

```rust
// validation.rs (baking_tool output check)
pub fn validate_shard_memory_contract(header: &ShardStateHeader) -> Result<()> {
    let vram_base = header.vram_base_ptr as usize;
    
    if vram_base % 4096 != 0 {
        panic!(
            "FATAL C-ABI BOUNDARY: VRAM base address 0x{:x} is not 4096-byte page-aligned. \
            This violates the Zero-Copy Mmap contract and will cause uncoalesced access & cache thrashing.",
            vram_base
        );
    }
    
    for soa_field in &header.soa_fields {
        let offset = soa_field.offset as usize;
        if offset % soa_field.required_align != 0 {
            panic!(
                "FATAL C-ABI BOUNDARY: SoA field '{}' @ offset 0x{:x} breaks {} alignment. \
                Baker must enforce columnar layout alignment from compile-time.",
                soa_field.name, offset, soa_field.required_align
            );
        }
    }
    
    Ok(())
}
```

**  Node Runtime:**

-  `load_shard(shard_id)`     `cudaMemcpy`.
-     ** **,      .
-  **legalization**    :  ,  `__pack__`,   .

---

### 1.7. SHM Binary Contract (Night Phase IPC v4)

  `axicor-runtime`  `axicor-baker-daemon`   Shared Memory. 
     128- `ShmHeader` (Little-Endian, C-ABI). 
       **64 **.

**The 1025-Byte SHM Invariant:** Header(128) + Weights(N*512) + Targets(N*512) + Flags(N*1).  1025     SHM.

|  |  |  |  |
| :--- | :--- | :--- | :--- |
| `0x00` | `magic` | `u32` | 0x47454E53 ("GENS") |
| `0x04` | `version` | `u8` |   = **3** |
| `0x05` | `state` | `u8` | State Machine (0=Idle, 1=NightStart, 2=Sprouting, 3=NightDone, 4=Error) |
| `0x06` | `_pad` | `u16` |  |
| `0x08` | `padded_n` | `u32` |   ( 64) |
| `0x0C` | `dendrite_slots` | `u32` |  128 |
| `0x10` | `weights_offset` | `u32` |   i32   ( 64) |
| `0x14` | `targets_offset` | `u32` |   u32   ( 64) |
| `0x18` | `epoch` | `u64` |    (BSP Epoch) |
| `0x20` | `total_axons` | `u32` | Local + Ghost + Virtual  |
| `0x24` | `handovers_offset` | `u32` |    `AxonHandoverEvent` ( 64) |
| `0x28` | `handovers_count` | `u32` |     |
| `0x2C` | `zone_hash` | `u32` | FNV-1a    |
| `0x30` | `prunes_offset` | `u32` |    `AxonHandoverPrune` ( 64) |
| `0x34` | `prunes_count` | `u32` |    |
| `0x38` | `incoming_prunes_count`| `u32` |      |
| `0x3C` | `flags_offset` | `u32` |   `soma_flags` ( 64) |
| `0x40` | `voltage_offset` | `u32` |   i32   |
| `0x44` | `threshold_offset_offset` | `u32` |   i32   |
| `0x48` | `timers_offset` | `u32` |   u8   |

** :  128 .     _reserved-.**

####   v3

- **`version = 3`:**      (`voltage`, `threshold`, `timers`)   Biological Amnesia (Tabula Rasa).
- **`voltage_offset` / `threshold_offset_offset`:**    (Optuna)       .


**:**   (offset)    64 . Baker      .

## 2.  :    (Day/Night Cycle)

 :    .       (Coalesced Access  GPU)     ( ).

**:** Night Phase - **    **.     -   .   .

### 2.1.   (Online / Hot Loop)

 **  GPU**.   ,    .

- **Read-Only :**       .  `malloc`/`free`  .
- ** :**   (GSOP), `axon_heads[]`, , , .

**   ( ):**

| # | Kernel |  |  |
|---|---|----|----|
| 1 | `InjectInputs` | Bitmask Injection    (single-tick pulse) | [05_signal_physics.md 2.4](./05_signal_physics.md) |
| 2 | `ApplySpikeBatch` |  Ghost indices  Schedule,  `axon_heads[ghost_id] = 0` | [05_signal_physics.md 1.2.1](./05_signal_physics.md) |
| 3 | `PropagateAxons` |  `axon_heads[tid] += v_seg`  ****  (Local + Ghost + Virtual) | [05_signal_physics.md 1.6](./05_signal_physics.md) |
| 4 | `UpdateNeurons` | GLIF +   +   +   | [05_signal_physics.md 1.5](./05_signal_physics.md) |
| 5 | `ApplyGSOP` | : Timer-as-Contact-Flag  STDP | [05_signal_physics.md 1.3](./05_signal_physics.md) |
| 6 | `RecordReadout` |  spike flags  mapped_soma_ids,   output_history | [05_signal_physics.md 3.2](./05_signal_physics.md) |
| 7 | ExtractTelemetry | Warp-Aggregated Atomics    | 07_gpu_runtime.md 2.1.1 |

#### 2.1.1. Warp-Aggregated Telemetry (Zero-Cost Observer)

      IDE           .

- **:**  `cu_extract_telemetry_kernel`   `soma_flags`.    `__ballot_sync` ( `__ballot`  AMD),   1         (32 ).
- ** :**      `atomicAdd`     VRAM.        32 .
- **Zero-Copy DMA:**       `cudaMemcpyAsync`   **4 **      (Pinned RAM). CPU     1  (`std::ptr::read_volatile`),        .


### 2.2.   (Per-Zone Offline Maintenance)

  **CPU**.    **  ** -   .

** :**
   CPU **    ** `sync_batch_ticks` (   ).   GPU    -   .

|  |  |  |
|---|---|---|
| **** | `night_interval_ticks`    | V1:  5 , :  2  |
| ** ** | `sleep_zone(zone_id)`  API  |    (    ) |
| **** | `night_interval_ticks = 0` |   (,  - Variant = Fixed/Relay, GSOP ) |

> **[WARN] Sentinel Refresh (  `night_interval_ticks = 0`):**
> `AXON_SENTINEL = 0x80000000`  59.6   v_seg=1.  Night Phase      . **:**  ~50  (`SENTINEL_REFRESH_TICKS = 1_800_000_000`) host   :  `axon_heads[id]`   `> SENTINEL_DANGER_THRESHOLD`    `AXON_SENTINEL`.   (head < propagation_length  10)  .

** Maintenance (5 ):**

|  |  |  |  |
|---|---|---|---|
| **1** | **GPU** | **Sort & Prune** | Segmented Radix Sort: 128   `abs(weight)` (descending).   `abs(w) < threshold` .  PCIe   . |
| **2** | **PCIe** | **Download** (VRAM  RAM) | `cudaMemcpyAsync`    ( + targets).     . |
| **3** | **CPU** | **Sprouting & Nudging** |  . Cone Tracing    (Spatial Hash),  ,  Ghost Axons   .      turnover rate. |
| **4** | **CPU** | **Baking** |     `.axons`.  SoA-    64 (L2 Cache Line & AMD Wavefront). |
| **5** | **PCIe** | **Upload** (RAM  VRAM) | `cudaMemcpyAsync`  .           AEP. |

**  Maintenance - .**    , turnover rate   CPU.  CPU =  ,  .    [Structural Determinism](./01_foundations.md) (2.3).

**  :**   `cudaMemcpy` ,       AEP   .

#### 2.2.1. Step 1: GPU Sort & Prune ()

**:** 128    (Columnar Layout, stride = N).  Radix Sort   stride  .

**: Shared Memory Staging (The 48 KB Budget)**

1.   128   **32 **  Shared Memory (AoS [Neuron][Slot])
   - **- 32 :**   AMD ( Wavefront = 64)  ****     32 .    Shared Memory (LDS)      .
   - ** :**  `DendriteSlot` (Target:4 + Weight:4 + Timer:1 + padding)  **12 **.
   -  32 : 12B  128   32  = **49 152  (48 KB)**.     L1/Shared   NVIDIA ( 48 KB   ),   AMD.
   -  64  (AMD default):    96 KB,  **   LDS  64 KB**      RDNA/CDNA,     .

2. **Bitonic Sort**  `abs(weight)` descending - , 32-.

3. **Auto LTM/WM Promotion:**        .

4. **Pruning:**     `abs(weight) < (prune_threshold << 16)`  `target = 0`.

5. **:**      Columnar Layout.  ,   (0)    ,     **Early Exit**  Day Phase.

```
Shared Memory (AoS, per warp):
+-------------------------------------------------+
| Neuron 0: [slot0_w, slot0_t, slot0_tmr] ... 128|
| Neuron 1: [slot0_w, slot0_t, slot0_tmr] ... 128|
| ...                                      32    |
+--------- Sort per neuron, write back -----------+
```

> **`weight = 0`  `target = 0`:**      0  GSOP depression -   ,    (target  0). GSOP    .   (`target = 0`) -  ,  Pruning.

#### 2.2.3. Step 3: Sprouting & Nudging (CPU, f32 )

  :   ,   .

**a) Nudging (Growth Step):**
-   `remaining_length > 0`    `step_and_pack()` (. [04_connectivity.md 4.3](./04_connectivity.md)).
- : `V_global + V_attract + V_noise`  `normalize`  `quantize`  `PackedPosition`.

**b) Boundary Check  NewAxon Handover:**
-         ,  `NewAxon { entry_point, vector, type_mask }`  Slow Path   (. [06_distributed.md 2.5](./06_distributed.md)).

**c) Spatial Grid Rebuild:**
-     3D - (  PackedPosition X|Y|Z).   Sprouting -  `get_in_radius()`    .

**d) Sprouting (Slot Filling):**
- CPU   `targets[]`.  `target_packed == 0` -  .
- Cone Query: `calculate_v_attract()`  Spatial Grid (FOV + Lookahead).
- :    = `seg_val >> 28` (4   PackedPosition).    .
- ** ** -   `sprouting_score()`  , `soma_power_index`  exploratory- (. [04_connectivity.md 1.6.1](./04_connectivity.md)).     .
-  `target_packed` ,  =  (74),    WM ( 80-127).

#### 2.2.4. Step 4: Baking & Defragmentation (CPU)

**a) f32  u32 Quantization:**
- Float-   `step_and_pack()`  `PackedPosition` (4 bytes/segment).

**b) DenseIndex Generation:**
- GPU   dense indices (0..N-1),   PackedPosition.
- CPU  : `PackedPosition  dense_id`   `target_packed`   .
-   `targets[]`  DenseIndex + segment offset.

**c) Columnar Layout Defrag:**
-       `Column[Slot_K]`,    .

**d) Warp Alignment:**
- `padded_n = align_to_warp(neuron_count)`.   `padded_n`  ,  **64** (L2 Cache Line & AMD Wavefront).
- **:**   ,     SoA- (4N, 2N, 1N )   64 .          L2 -   padding bytes  .
-  `.state`  `.axons`  --   VRAM layout  Step 5: `cudaMemcpyAsync`.

### 2.3. External I/O Server (UDP  /)

 Tokio- (  )    External Hub.  I/O .

```rust
pub struct ExternalIoServer {
    sock_in: Arc<UdpSocket>,        // Port N:  Input Bitmasks
    sock_out: Arc<UdpSocket>,       // Port N+1:  Output History
    last_client_addr: Option<SocketAddr>, //   
}

//  
#[repr(C)]
pub struct ExternalIoHeader {
    pub zone_hash: u32,     //  Zone
    pub matrix_hash: u32,   //  Input/Output 
    pub payload_size: u32,  //  
}
```

**:** UDP   65KB   ( EMSGSIZE  ).     .

****:
-    ( `current_tick_in_batch == 0`)   UDP   `output_history`    (, ).
-    `Input Bitmask`  ,   `try_recv_input()`        Virtual Axons (`InjectInputs`).

### 2.3.1. WaitStrategy:  CPU   

**:** - (GPU) ,    (Night Phase)    (Network Phase, . [06_distributed.md 2.10](./06_distributed.md))    OS scheduler.

**:** CPU       BSP-    I/O .    spin/yield    .

**3  ( `--cpu-profile`):**

|  |  |  |  |
|---|---|---|---|
| **Aggressive** | `std::hint::spin_loop()` | ~1  , 100% CPU | Production, HFT,   |
| **Balanced** | `std::thread::yield_now()` | OS  ,    (~115 ) | , SSH-,   |
| **Eco** | `std::thread::sleep(1ms)` | ~0% CPU  ,   | , ,   |

```rust
pub enum WaitStrategy {
    Aggressive,
    Balanced,
    Eco,
}

impl WaitStrategy {
    pub fn poll_neighbors_until_ready(&self) -> Vec<SpikeBatch> {
        loop {
            if let Some(batch) = try_recv_all_neighbors() {
                return batch;
            }
            match self {
                Self::Aggressive => std::hint::spin_loop(),
                Self::Balanced => std::thread::yield_now(),
                Self::Eco => std::thread::sleep(Duration::from_millis(1)),
            }
        }
    }
}
```

**:**

1. **  :** WaitStrategy    runtime in `OnceLock<WaitStrategy>`.  cost   .
2. **:** BSP- -  ,  CPU   .  Mutex,  CAS-loop.
3. **:**   (GSOP, , )    .   OS-level scheduling .


### 2.4.   (Spike Drop)

  ,        (Fast Path).

-     TCP/UDP ,   `SLEEP`  ** Drop**.
-    VRAM.  .   ** **.
- ** :**       .     ,   -.


## Connected Documents

| Document | Connection |
|---|---|
| [05_signal_physics.md](./05_signal_physics.md) | Day Pipeline kernels (1.0), Constant Memory variant parameters |
| [06_distributed.md](./06_distributed.md) | Ring Buffer, Ghost Axons, BSP sync, network I/O |
| [02_configuration.md](./02_configuration.md) | Variant definitions, blueprints, parameter validation |
| [09_baking_pipeline.md](./09_baking_pipeline.md) | .state/.axons file format, Sort&Prune during Night |
| [project_structure.md](../project_structure.md) | Architecture overview |

---

## Changelog

|  |  |  |
|---|---|---|
| 2026-03-17 | 2.2 |    VRAM  VariantParameters.  BurstHeads8 (64-byte alignment).    soma_flags.  inertia_curve (u8[16]). |
| 2026-02-28 | 2.1 |  VramState   memory.rs ( I/O Matrix , readout ).   Day Phase  6 kernels    .   External I/O Server  UDP . |
| TBD | 2.0 |   |

---

## 3.    (Lifecycle Invariants)

### 3.1. Cold Start: Sentinel Assert

> **[WARN] Baking Tool Assert:**   `.state`  Baking Tool      `axon_heads`  `AXON_SENTINEL` (`0x80000000`),    (`calloc`-default).           1 -        .

```rust
// baking_compiler/src/validate.rs
assert!(
    axon_heads.iter().all(|&h| h == AXON_SENTINEL),
    "CRITICAL: axon_heads must be initialized to AXON_SENTINEL, not zero!"
);
```

### 3.2. Reset: O(1)    

  `reset_zone(zone_id)`:

1. **   (Night Phase):**  **** - CPU   Maintenance pipeline ( Step 5 Upload ).     VRAM with   .
2. **Ring Buffer  (O(1)):**   `counts`  Ping-Pong .  `ghost_id`   - GPU   `counts[tick]` .      .

```rust
// O(1) -   ,   
memset(schedule_a.counts, 0, batch_size * size_of::<u32>());
memset(schedule_b.counts, 0, batch_size * size_of::<u32>());
```

> **Phantom Signals & Input Bleed:**    Ring Buffer   - **  ** (   ). Input Bleed    - .   .

### 3.3. Hot Checkpoint (   )

     Night Phase,   ** ** (`dendrite_weights` + `dendrite_targets`)   :

```rust
const CHECKPOINT_INTERVAL_BATCHES: u32 =
    300_000_000 / TICK_DURATION_US / SYNC_BATCH_TICKS; //  5 

if batch_counter % CHECKPOINT_INTERVAL_BATCHES == 0 {
    cudaMemcpyAsync(host_buf, vram_weights, ..., DeviceToHost);
    //  :  .tmp,  rename() -   
    write_to_disk("checkpoint_weights.bin.tmp");
    rename("checkpoint_weights.bin.tmp", "checkpoint_weights.bin");
}
```

|   |  |  |
|---|---|---|
| **** (`axons`) |   Night Phase | `.axons` |
| **** (`weights` + `targets`) |  ~5  | `checkpoint_weights.bin` |

### 3.4. Crash Tolerance (Mmap Page Cache Flush)

   Zero-Copy Mmap (`.geom`, `.paths`)   Page Cache.       Kernel Panic, "" (dirty)         (NVMe SSD),     .

**  (Flush Async):**
      Night Phase ( `run_sprouting_pass`), Baker Daemon  `flush_async()`  `mmap_geom`  `mmap_paths`.      ,         ,     .
