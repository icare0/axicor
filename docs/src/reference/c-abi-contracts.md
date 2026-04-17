# C-ABI Contracts

> Reference manual for all binary structures and memory layouts in the Axicor engine.
> All structures are strictly Little-Endian.

## 1. Network & Data Plane (UDP Fast-Path)

### 1.1. Spike Batch V2
Used for inter-shard communication. The `SpikeBatchHeaderV2` is followed by an array of `SpikeEventV2`.

```c
// [C-ABI] Strictly 16 bytes. alignas(16) is mandatory for Zero-Cost vectorization
// when reading directly from LwIP buffers on microcontrollers and GPUs.
struct alignas(16) SpikeBatchHeaderV2 {
    uint32_t src_zone_hash;    // FNV-1a Hash of the sending zone
    uint32_t dst_zone_hash;    // FNV-1a Hash of the receiving zone
    uint32_t epoch;            // Global batch counter (BSP Epoch)
    uint32_t is_last;          // 0 = Chunk, 1 = Last chunk (Heartbeat), 2 = ACK
};

// [C-ABI] Strictly 8 bytes.
struct alignas(8) SpikeEventV2 {
    uint32_t ghost_id;         // DIRECT index in the receiving shard's VRAM
    uint32_t tick_offset;      // Offset within the batch (0..sync_batch_ticks)
};
```

### 1.2. External I/O & Control Plane
Used for environment interaction (sensors, motors) and R-STDP dopamine injection.

```c
// [C-ABI] Strictly 20 bytes.
struct alignas(4) ExternalIoHeader {
    uint32_t magic;         // 0x4F495347 ("GSIO") for RX, 0x4F4F5347 ("GSOO") for TX
    uint32_t zone_hash;     // FNV-1a hash of the zone name
    uint32_t matrix_hash;   // FNV-1a hash of the I/O matrix
    uint32_t payload_size;  // Size of the bitmask payload (excluding header)
    int16_t  global_reward; // [DOD] R-STDP Dopamine Modulator (-32768..32767)
    uint16_t _padding;      // Alignment to 20 bytes
};

#define CTRL_MAGIC_DOPA 0x41504F44 // "DOPA" in Little-Endian

// [C-ABI] Strictly 8 bytes. Aliases SpikeEventV2 in the network buffer.
struct alignas(8) ControlPacket {
    uint32_t magic;     // MUST be CTRL_MAGIC_DOPA
    int16_t dopamine;   // R-STDP injection (-32768..32767)
    uint16_t _pad;      // Alignment to 8 bytes
};
```

---

## 2. Shared Memory IPC (Night Phase)

### 2.1. ShmHeader v3
Used for POSIX `/dev/shm` or Windows `%TEMP%` file-backed memory mapping.

```c
// [C-ABI] Strictly 128 bytes (L2 Cache Line x2)
struct alignas(128) ShmHeader {
    uint32_t magic;             // 0x00: 0x47454E53 ("GENS")
    uint8_t  version;           // 0x04: 3
    uint8_t  state;             // 0x05: ShmState enum
    uint16_t _pad;              // 0x06: Padding
    uint32_t padded_n;          // 0x08: Neurons (Warp Aligned!)
    uint32_t dendrite_slots;    // 0x0C: Always 128 (32 for Edge/ESP32)
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
    uint32_t _reserved[1];           // 0x4C..0x80: Pad to 128 bytes
};
```

---

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

    uint8_t inertia_curve[2];                // 32..48 (ranks: abs(w) >> 11)

    int32_t adaptive_leak_max;                // 48..52
    uint16_t adaptive_leak_gain;              // 52..54
    uint8_t adaptive_mode;                    // 54..55
    uint8_t _leak_pad[3];                     // 55..58

    uint8_t d1_affinity;                      // 58..59 (LTP sensitivity)
    uint8_t d2_affinity;                      // 59..60 (LTD suppression sensitivity)
    uint8_t _pad[4];                          // 60..64 (Alignment)
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

---

## 4. Binary File Formats (Baking Artifacts)

All binary files produced by `axicor-baker` follow strict Little-Endian headers.

| Format | Magic (u32) | String | Header Size | Purpose |
| :--- | :--- | :--- | :--- | :--- |
| `.gxi` | `0x47584900` | `GXI\0` | 32 bytes | Input Matrix Mapping |
| `.gxo` | `0x47584F00` | `GXO\0` | 32 bytes | Output Matrix Mapping |
| `.ghosts` | `0x47485354` | `GHST` | 20 bytes | Inter-zone Links & Topology |
| `.paths` | `0x50415448` | `PATH` | 16 bytes | Axon Full 3D Geometry |
