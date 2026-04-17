# GPU Runtime & Storage

> Part of the Axicor architecture. Defines how data is laid out in VRAM, loaded, and how the engine phases operate.

## 1. Memory & Data Architecture

**[INVARIANT]** Complete rejection of Array of Structures (AoS) in hot memory. Data is laid out in flat vectors (Structure of Arrays, SoA) to guarantee 100% Coalesced Memory Access on the GPU and enable vectorization on CPU/MCU.

The engine supports a **Dual-Backend** architecture: native CUDA and native HIP. All computations are performed exclusively in integers (**Integer Physics**).

### 1.1. Strict FFI VRAM Contract (Headerless SoA)

The engine does not parse data at startup. The `.state` and `.axons` binary files are raw memory dumps (Headerless), ready for `cudaMemcpyAsync` or `spi_flash_mmap` (ESP32).

**[C-ABI] The 64-Byte Alignment Rule:**
The global padding of `padded_n` to a multiple of 64 mathematically guarantees that the lengths of all internal SoA arrays (4N, 2N, 1N bytes) will be multiples of 64 bytes. This ensures every array perfectly aligns with a new L2 cache line without injecting dirty padding bytes into the blob. This is critical for Coalesced Access on AMD Wavefronts (64 threads) and eliminating L2 cache thrashing.

**ShardVramPtrs Layout (Strict byte-for-byte order in `.state`):**
*   `soma_voltage`       [N] × `i32`   (4N bytes)
*   `soma_flags`         [N] × `u8`    (1N bytes)
*   `threshold_offset`   [N] × `i32`   (4N bytes)
*   `timers`             [N] × `u8`    (1N bytes)
*   `soma_to_axon`       [N] × `u32`   (4N bytes)
*   `dendrite_targets`   [128 × N] × `u32` (512N bytes)
*   `dendrite_weights`   [128 × N] × `i32` (512N bytes)
*   `dendrite_timers`    [128 × N] × `u8`  (128N bytes)

**The 1166-Byte Invariant:** Soma (14) + 128 * (Targets:4 + Weights:4 + Timers:1) = 1166 bytes per neuron.

**Axons (`.axons` blob):**
*   `axon_heads` [A] × `BurstHeads8` (32A bytes)

**[C-ABI] 32-Byte Axon Alignment Invariant:**
The `BurstHeads8` structure must be strictly 32-byte aligned. 8 heads × 4 bytes = exactly 32 bytes. This guarantees that all heads are loaded in a single L1 cache transaction.

```c
// 32-byte alignment guarantees a perfect hit in a 32-byte L1 cache sector.
struct alignas(32) BurstHeads8 {
    uint32_t h0; uint32_t h1; uint32_t h2; uint32_t h3;
    uint32_t h4; uint32_t h5; uint32_t h6; uint32_t h7;
};
```

**Bit Semantics of `soma_flags` (1 byte):**

- `[7:4] type_mask (u4)`: Neuron type index (0..15). Direct index into `VARIANT_LUT`.
- `[3:1] burst_count (u3)`: Serial spike counter for Burst-Dependent Plasticity (BDP).
- `[0:0] is_spiking (u1)`: Spike flag for the current tick (1 = fired).

### 1.2. Constant Memory (VariantParameters)
Membrane and plasticity parameters are loaded into the GPU's constant memory once at startup. The structure is strictly 64 bytes to perfectly fit into an L1 Cache line.

```c
// [C-ABI] Strictly 64 bytes (1 L1 cache line)
struct alignas(64) VariantParameters {
    // === Block 1: 32-bit (Offsets 0..20) ===
    int32_t threshold;                        // 0..4
    int32_t rest_potential;                   // 4..8
    int32_t leak_rate;                        // 8..12
    int32_t homeostasis_penalty;              // 12..16
    uint32_t spontaneous_firing_period_ticks; // 16..20

    // === Block 2: 16-bit (Offsets 20..28) ===
    uint16_t initial_synapse_weight;          // 20..22
    uint16_t gsop_potentiation;               // 22..24
    uint16_t gsop_depression;                 // 24..26
    uint16_t homeostasis_decay;               // 26..28

    // === Block 3: 8-bit (Offsets 28..32) ===
    uint8_t refractory_period;                // 28..29
    uint8_t synapse_refractory_period;        // 29..30
    uint8_t signal_propagation_length;        // 30..31
    uint8_t is_inhibitory;                    // 31..32

    // === Block 4: Arrays (Offsets 32..48) ===
    uint8_t inertia_curve[1];                // 32..48 (ranks: abs(w) >> 11)

    // === Block 5: Adaptive Leak Hardware (Offsets 48..58) ===
    int32_t adaptive_leak_max;                // 48..52
    uint16_t adaptive_leak_gain;              // 52..54
    uint8_t adaptive_mode;                    // 54..55
    uint8_t _leak_pad[2];                     // 55..58

    // === Block 6: Neuromodulation & Pad (Offsets 58..64) ===
    uint8_t d1_affinity;                      // 58..59 (LTP sensitivity)
    uint8_t d2_affinity;                      // 59..60 (LTD suppression sensitivity)
    uint8_t _pad[3];                          // 60..64 (Alignment)
};

struct GenesisConstantMemory {
    VariantParameters variants[1];
};
```

### 1.3. Cross-Platform IPC & Zero-Copy Mmap
Zero-Copy DMA loading is platform-agnostic:

- **Linux:** POSIX `shm_open()` → `/dev/shm/*.state.shm`
- **Windows:** File-backed `mmap` via `CreateFileMapping` in `%TEMP%` enforcing strict 4096-byte OS Page Alignment.

**[C-ABI] SHM Night Phase IPC v3 (Exchange):**

```c
// Strictly 128 bytes (L2 Cache Line x2)
struct alignas(128) ShmHeader {
    uint32_t magic;             // 0x00: 0x47454E53 ("GENS")
    uint8_t  version;           // 0x04: 3
    uint8_t  state;             // 0x05: ShmState enum
    uint16_t _pad;              // 0x06: Padding
    uint32_t padded_n;          // 0x08: Neurons (Warp Aligned!)
    uint32_t dendrite_slots;    // 0x0C: Always 128
    uint32_t weights_offset;    // 0x10: Offset to i32[128 * padded_n]
    uint32_t targets_offset;    // 0x14: Offset to u32[128 * padded_n]
    uint64_t epoch;             // 0x18: Global batch counter (BSP Epoch)
    uint32_t total_axons;       // 0x20: Local + Ghost + Virtual
    uint32_t handovers_offset;  // 0x24: Offset to AxonHandoverEvent queue
    uint32_t handovers_count;   // 0x28: Elements in queue
    uint32_t zone_hash;         // 0x2C: FNV-1a zone hash
    uint32_t prunes_offset;     // 0x30: Offset to AxonHandoverPrune queue
    uint32_t prunes_count;      // 0x34: Outgoing prunes count
    uint32_t incoming_prunes;   // 0x38: Incoming prunes
    uint32_t flags_offset;      // 0x3C: Offset to u8[padded_n]

    // --- Extended Header (v3) ---
    uint32_t voltage_offset;          // 0x40: Offset to i32[padded_n]
    uint32_t threshold_offset_offset; // 0x44: Offset to i32[padded_n]
    uint32_t timers_offset;           // 0x48: Offset to u8[padded_n]
    uint32_t _reserved[4];            // 0x4C..0x80: Pad to 128 bytes
};
```

---

## 2. Day/Night Cycle Architecture
The fundamental solution to resolving conflicts between rigid static memory (GPU Coalesced Access) and structural graph plasticity (dynamic allocations) is temporal separation.

### 2.1. Day Phase (Online / Hot Loop)
Executed exclusively on the GPU. Maximum bandwidth, zero structural logic.

- **Warp-Aggregated Telemetry (Zero-Cost Observer):** The `cu_extract_telemetry_kernel` uses hardware instructions (`__ballot_sync`, `__popc`) to aggregate spikes at the warp level. The warp leader performs a single `atomicAdd`, reducing memory bus contention by a factor of 32. At the end of the batch, the orchestrator triggers a 4-byte `cudaMemcpyAsync` to read the global spike count without blocking.

### 2.2. Night Phase (Per-Zone Offline Maintenance)
Executed on the CPU. Each zone has its own independent sleep cycle.

- **GPU Sort & Prune (The 48 KB Shared Memory Budget):** A Segmented Radix Sort operates on 128 slots for 32 neurons per thread block. `12 bytes * 128 slots * 32 threads = 49,152 bytes` (48 KB). This perfectly fits the L1/Shared Cache limit on both NVIDIA and AMD GPUs. 64 threads would exceed the 64 KB LDS limit on RDNA architectures.
- **Sentinel Refresh:** If a zone operates continuously (`night_interval_ticks = 0`), inactive axon heads may overflow `AXON_SENTINEL` (0x80000000). The host periodically sweeps the array, resetting heads > `SENTINEL_DANGER_THRESHOLD` to `AXON_SENTINEL` without corrupting the Active Tails of recently fired signals.

---

## 3. External I/O Server (UDP)
A dedicated Tokio server routes Data Plane interactions (sensors/motors) without blocking the engine.

```c
// [C-ABI] Strictly 20 bytes. Little-Endian.
struct alignas(4) ExternalIoHeader {
    uint32_t magic;         // 0x4F495347 ("GSIO") or 0x4F4F5347 ("GSOO")
    uint32_t zone_hash;     // FNV-1a hash of the zone name
    uint32_t matrix_hash;   // FNV-1a hash of the I/O matrix
    uint32_t payload_size;  // Size of the bitmask payload (excluding header)
    int16_t  global_reward; // [DOD] R-STDP Dopamine Modulator (-32768..32767)
    uint16_t _padding;      // Alignment to 20 bytes
};
```

**WaitStrategy Profiles:**

- **Aggressive:** `std::hint::spin_loop()`. ~1 ns latency. Used in HFT production.
- **Balanced:** `std::thread::yield_now()`. ~1-15 ms latency. Shared CPU environments.
- **Eco:** `std::thread::sleep(1ms)`. ~0% idle CPU. Used on laptops to preserve battery.