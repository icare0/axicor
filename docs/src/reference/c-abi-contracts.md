# C-ABI Memory Contracts & Data-Oriented Layout

Axicor is built on strict Data-Oriented Design (DOD). We completely reject OOP. Neurons do not exist as objects. The entire simulation state is mapped directly into VRAM and POSIX Shared Memory (`/dev/shm`) as flat, Headerless Structure of Arrays (SoA).

Any external tool (Python SDK, C++ drivers, visualization) must respect these exact C-ABI byte alignments. All structures are strictly Little-Endian.

## 1. The 1166-Byte Invariant (VRAM / `.state`)

The memory cost of a single neuron inside the GPU/CPU hot loop is exactly **1166 bytes**. The arrays are allocated as flat contiguous memory blocks. 

*   **Soma Hot State (14 Bytes):**
    *   `voltage` (i32) -> 4 bytes
    *   `flags` (u8) -> 1 byte (Bits [7..4]: Variant ID | Bits [3..1]: Burst Count | Bit 0: Is Spiking)
    *   `threshold_offset` (i32) -> 4 bytes
    *   `refractory_timer` (u8) -> 1 byte
    *   `soma_to_axon` (u32) -> 4 bytes (O(1) mapping to Axon ID)
*   **Dendrite Matrix (1152 Bytes - Columnar Layout):**
    *   Hard limit of 128 dendritic slots. Stored transposed (Column-Major) so all 32 threads in a Warp read the same slot consecutively.
    *   `targets` (u32) -> 4 bytes * 128 = 512 bytes
    *   `weights` (i32) -> 4 bytes * 128 = 512 bytes (Mass Domain representation)
    *   `timers` (u8) -> 1 byte * 128 = 128 bytes

> **[CRITICAL INVARIANT] Warp Alignment:** The total number of neurons (`padded_n`) is ALWAYS padded to a multiple of 64 bytes (L2 Cache Line & AMD Wavefront). If you read this data via FFI, you must account for the padding bytes at the end of the array.

## 2. The 1025-Byte Invariant (Night Phase IPC)

During the Night Phase, the GPU transfers a subset of its memory to the CPU via POSIX `/dev/shm` for structural plasticity (sprouting, pruning). The transfer payload per neuron is exactly **1025 bytes**.

*   `weights` (i32) -> 512 bytes
*   `targets` (u32) -> 512 bytes
*   `soma_flags` (u8) -> 1 byte (Used as an Activity Gate to skip silent neurons)

### SHM Header (128 Bytes Aligned)

The shared memory file begins with a strictly 128-byte aligned header to prevent cache-line tearing.

```c
// [C-ABI] Strictly 128 bytes (L2 Cache Line x2)
struct alignas(128) ShmHeader {
    uint32_t magic;             // 0x41584943 ("AXIC")
    uint8_t  version;           // 3
    uint8_t  state;             // ShmState (0=Idle, 1=NightStart...)
    uint16_t _pad;              
    uint32_t padded_n;          // Total neurons (Warp-aligned)
    uint32_t dendrite_slots;    // Always 128
    uint32_t weights_offset;    // Offset to i32[128 * padded_n]
    uint32_t targets_offset;    // Offset to u32[128 * padded_n]
    uint64_t epoch;             // Global BSP Epoch
    uint32_t total_axons;       // Local + Ghost + Virtual
    uint32_t handovers_offset;  // Offset to AxonHandoverEvent array
    uint32_t handovers_count;   
    uint32_t zone_hash;         // FNV-1a hash
    uint32_t prunes_offset;     // Offset to AxonHandoverPrune array
    uint32_t prunes_count;      
    uint32_t incoming_prunes;   
    uint32_t flags_offset;      // Offset to u8[padded_n]
    
    // v3+ Extensions
    uint32_t voltage_offset;          // Offset to i32[padded_n]
    uint32_t threshold_offset_offset; // Offset to i32[padded_n]
    uint32_t timers_offset;           // Offset to u8[padded_n]
    uint32_t _reserved[13];          // Padding to reach exactly 128 bytes (76 + 13 * 4 = 128)
};
```

## 3. GPU Memory Invariants

### 3.1. Constant Memory (VariantParameters)

```c
// [C-ABI] Strictly 64 bytes (1 L1 cache line)
struct alignas(64) VariantParameters {
    int32_t threshold;                        // 0..4
    int32_t rest_potential;                   // 4..8
    int32_t leak_rate;                        // 8..12
    int32_t homeostasis_penalty;              // 12..16
    uint32_t spontaneous_firing_period_ticks; // 16..20

    uint16_t initial_synapse_weight;          // 20..22
    uint16_t gsop_potentiation;               // 22..24
    uint16_t gsop_depression;                 // 24..26
    uint16_t homeostasis_decay;               // 26..28

    uint8_t refractory_period;                // 28..29
    uint8_t synapse_refractory_period;        // 29..30
    uint8_t signal_propagation_length;        // 30..31
    uint8_t is_inhibitory;                    // 31..32

    uint8_t inertia_curve[16];               // 32..48

    int32_t adaptive_leak_max;                // 48..52
    uint16_t adaptive_leak_gain;              // 52..54
    uint8_t adaptive_mode;                    // 54..55
    uint8_t _leak_pad[3];                     // 55..58

    uint8_t d1_affinity;                      // 58..59
    uint8_t d2_affinity;                      // 59..60
    uint32_t heartbeat_m;                     // 60..64
};
```

### 3.2. Axon Heads

```c
// [C-ABI] Strictly 32 bytes (Half L1 cache line)
struct alignas(32) BurstHeads8 {
    uint32_t h0; uint32_t h1; uint32_t h2; uint32_t h3;
    uint32_t h4; uint32_t h5; uint32_t h6; uint32_t h7;
};
```

## 4. Network & Data Plane (UDP Fast-Path)

### 4.1. Spike Batch V2 (Inter-Shard)
Used for inter-shard communication. The `SpikeBatchHeaderV2` is followed by an array of `SpikeEventV2`.

```c
// [C-ABI] Strictly 16 bytes.
struct alignas(16) SpikeBatchHeaderV2 {
    uint32_t src_zone_hash;    // FNV-1a Hash of the sending zone
    uint32_t dst_zone_hash;    // FNV-1a Hash of the receiving zone
    uint32_t epoch;            // Global batch counter (BSP Epoch)
    uint16_t chunk_idx;        // 0xFFFF = ACK
    uint16_t total_chunks;     // 0 = Empty heartbeat / ACK
};

// [C-ABI] Strictly 8 bytes.
struct alignas(8) SpikeEventV2 {
    uint32_t ghost_id;         // DIRECT index in the receiving shard's VRAM
    uint32_t tick_offset;      // Offset within the batch
};
```

### 4.2. External I/O (Sensors/Motors)
Used for environment interaction (sensors, motors) and R-STDP dopamine injection.

```c
// [C-ABI] Strictly 20 bytes.
struct alignas(4) ExternalIoHeader {
    uint32_t magic;         // 0x4F495347 ("GSIO") input, 0x4F4F5347 ("GSOO") output
    uint32_t zone_hash;     // FNV-1a hash of the target zone name
    uint32_t matrix_hash;   // FNV-1a hash of the matrix name
    uint32_t payload_size;  // Size of following data
    int16_t  global_reward; // Global R-STDP Dopamine Modulator (-32768..32767)
    uint16_t _padding;      // Padding to 20 bytes
};

// [C-ABI] Strictly 8 bytes. Aliases SpikeEventV2 in the network buffer.
struct alignas(8) ControlPacket {
    uint32_t magic;     // MUST be 0x41504F44 ("DOPA")
    int16_t dopamine;   // R-STDP injection (-32768..32767)
    uint16_t _pad;      // Alignment to 8 bytes
};
```

## 5. Binary File Formats (Baking Artifacts)

All binary files produced by `axicor-baker` follow strict Little-Endian headers.

| Format | Magic (u32) | String | Header Size | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| `.gxi` | `0x47584900` | `GXI\0` | 32 bytes | Input Matrix Mapping |
| `.gxo` | `0x47584F00` | `GXO\0` | 32 bytes | Output Matrix Mapping |
| `.ghosts` | `0x47485354` | `GHST` | 16 bytes | Inter-zone Links & Topology |
| `.paths` | `0x50415448` | `PATH` | 16 bytes | Axon Full 3D Geometry |
