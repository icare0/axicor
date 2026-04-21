# Configuration

> Part of the Axicor architecture. File structure, data pipeline, and TOML specifications.

## 1. Configuration Hierarchy (Human Readable)

Configurations are divided into three levels of responsibility. We separate them so that the "hardware" of the neuron can be changed without breaking the layer structure.

**Laws of Physics  Zone Specification  Instance Parameters**

### 1.1. Global Physics (`simulation.toml`)
Located in the root. These are constants that **cannot differ** between zones, otherwise spatial and temporal synchronization will break.

- `tick_duration_us` - Time step (in microseconds).
- `voxel_size_um` - Voxel size (spatial quantum).
- `signal_speed_um_tick` - Signal propagation speed.
- `sync_batch_ticks` - Number of autonomous calculation ticks between shard synchronizations.
- `master_seed` - Global seed (String, hashed into u64 at startup) for deterministic generation.
- `max_dendrites` - Hard limit (128).

**Memory Impact:** These are global `const` variables (Uniforms in CUDA). Loaded once, always in L1 cache.

### 1.2. Zone Config (`Axicor_Models/{project}/zones/V1/`)
All files of a specific zone reside in its directory. This allows parallel loading of multiple zones in Runtime.
Defines **what** we are building.

1. **`anatomy.toml`** - Layers (L1L6) in height percentages. Allows scaling (`world.height`) without rewriting the config.
2. **`blueprints.toml`** - Neuron types: membrane parameters (threshold, leak) and connectivity rules (matrix).
3. **`io.toml`** - Input/Output map.

### 1.3. Instance Config (`runtime/shard_04.toml` or CLI args)
Defines **where** and **what part** this process calculates.

- `zone_id` - "V1" (link to zone folder).
- `world_offset` - `{ x: 1000, y: 0, z: 0 }` (offset of this shard in the global brain).
- `dimensions` - `{ w: 500, h: 2000 }` (dimensions of this piece).
- `neighbors` - List of addresses (IP:Port or Shared Memory ID) for neighbors (X+, X-, Y+, Y-).

#### 1.3.1. `[settings]` Section (Runtime Optimization)
Parameters affecting the behavior of a specific instance, but not the biological logic.

- `save_checkpoints_interval_ticks` - Interval (in ticks) between hot state saves (`checkpoint.state` / `.axons`). Default: `100000`.
- `ghost_capacity` (u32) - VRAM reserve for inter-shard connections (Ghost Axons). Generated automatically by the `axicor-client` SDK based on `SUM(width * height) * 2.0`.

---

## 2. Baking Pipeline

**[INVARIANT] The Engine (Runtime) NEVER reads TOML files directly.**

Parsing text configs on the GPU destroys performance. The "Baking" stage separates "Human Configs" (TOML) and "Machine Data" (Binary Blobs). All results are saved in `Axicor_Models/`.

### 2.1. Pipeline
`TOML Configs` -- `[Compiler Tool (CPU)]` -- `Binary Blobs` -- `[Runtime (GPU)]`

- **Zero-Copy Loading:** The engine performs `mmap` or direct `cudaMemcpy` from the binary into VRAM. No parsing at startup.

### 2.2. Alignment
- Data on disk is laid out **byte-for-byte** exactly as required by the video card.
- If a data structure occupies 28 bytes, it is padded to **32 bytes** for Coalesced Access.

### 2.3. Configuration Validation (Baking Tool Asserts)
When parsing `simulation.toml`, the Baking Tool calculates derived quantities and verifies invariants:

```rust
// axicor-baker/src/physics_constants.rs

let segment_length_um = config.voxel_size_um * config.segment_length_voxels;
let v_seg = config.signal_speed_um_tick / segment_length_um;

// Runtime assert: v_seg MUST be an integer
assert!(
    config.signal_speed_um_tick % segment_length_um == 0,
    "CRITICAL: signal_speed_um_tick must be a multiple of segment_length_um!"
);
```

### 2.4. .state Format: ShardStateSoA
The `.state` binary file is a flat concatenation of arrays. The structure matches the VRAM layout byte-for-byte.

```rust
// axicor-baker/src/layout.rs

/// Warp Alignment: size is a multiple of 32
pub const fn align_to_warp(count: usize) -> usize {
    (count + 31) & !31
}

pub struct ShardStateSoA {
    pub padded_n: usize,       // Neurons (aligned to 32)
    pub total_axons: usize,    // Local + Ghost + Virtual (aligned to 32)

    // Soma arrays (size = padded_n)
    pub voltage: Vec<i32>,           // 4 bytes  N
    pub flags: Vec<u8>,              // 1 byte  N (upper nibble: Type, bit 0: Is_Spiking)
    pub threshold_offset: Vec<i32>,  // 4 bytes  N
    pub refractory_counter: Vec<u8>, // 1 byte  N
    pub soma_to_axon: Vec<u32>,      // 4 bytes  N (mapping Soma  Local Axon ID)

    // Dendrite arrays - Columnar Layout (size = 128  padded_n)
    // CUDA access: data[slot * padded_n + tid]
    pub dendrite_targets: Vec<u32>,  // 4 bytes  128  N (Packed: 22b Axon_ID | 10b Segment_Index)
    pub dendrite_weights: Vec<i16>,  // 2 bytes  128  N (Signed: sign = E/I)
    pub dendrite_timers: Vec<u8>,    // 1 byte  128  N

    // Axon arrays (size = total_axons, NOT padded_n!)
    pub axon_heads: Vec<u32>,        // 4 bytes  A (PropagateAxons: += v_seg)
}
```

Seed-deterministic generation: `PackedPosition` (u32) is used as `entity_id` for hashing:

```rust
pub fn entity_seed(master_seed: u64, packed_pos: u32) -> u64 {
    wyhash::wyhash(&packed_pos.to_le_bytes(), master_seed)
}
```

---

## 3. Data Storage Law: SoA (Structure of Arrays)
Complete rejection of objects in GPU memory. This is not a recommendation, it is an architectural law.

### 3.1. The Problem: AoS (Array of Structures)
If you read voltage for all neurons in an AoS pattern, the GPU loads `pos` and `type_id` into the cache  garbage that is not needed. Cache line payload  15%.

### 3.2. The Solution: SoA
A GPU warp (32 threads) reads 32 consecutive f32 values in one memory transaction. Cache line payload = 100%.

### 3.3. Transposed Dendrites (Columnar Layout)
The narrowest memory bottleneck: 128 dendrites per neuron.

- Not `Neuron.Dendrites[0..127]` (row-major).
- But `Column[Slot_K]` - K-th dendrite for all neurons consecutively (column-major).

In the loop `for slot in 0..128`, all GPU threads access `Slot_K`  perfectly linear read. Bandwidth is utilized at 100%.

---

## 4. Specification: `simulation.toml`
Defines the "sandbox" where the system lives.

```toml
[world]
width_um = 1000
depth_um = 1000
height_um = 2500

[model_id_v1]
id = "1cf8a9c1-37ef-4f6d-8d7c-dd911257fb91"

[simulation]
tick_duration_us = 100
voxel_size_um = 25
segment_length_voxels = 2
signal_speed_um_tick = 50
sync_batch_ticks = 100
total_ticks = 0
master_seed = "AXICOR"
axon_growth_max_steps = 500
```

[DOD FIX] `global_density` REMOVED. Density is now calculated strictly per-layer in `anatomy.toml` (bottom-up allocation).

### 4.2. Architectural Constants (Immutable)
Parameters that are "baked" at startup and do not change at runtime:

| Parameter | Type | Value | Note |
| :--- | :--- | :--- | :--- |
| `tick_duration_us` | u32 | 100 | 0.1 ms |
| `voxel_size_um` | f32 | 25.0 | Spatial quantum |
| `signal_speed_um_tick` | u16 | 50 | Calculated at startup. `int` for fast addition. |
| `max_dendrites` | const | 128 | Hard Constraint. Guarantees memory alignment by 128 bytes (cache lines + warps). |
| `master_seed` | String | "AXICOR" | Hashed into u64 at startup. |

### 4.3. Warp Alignment Rule
**Padding Rule:** The number of active neurons in the shard binary (`.state`) is always padded with dummies to a number multiple of 32.

---

## 5. Specification: `anatomy.toml`
Defines the anatomy of a specific zone: layers, density, composition.

### 5.1. Vertical Metric (Relative Height)

- Layers are specified strictly in percentages (`height_pct`) from 0.0 to 1.0.
- Invariant: L1 + L2 + ... + L6 = 1.0.

### 5.2. Absolute Layer Density
Density is specified strictly for each layer:

- `density`: The percentage of filled voxels in this specific physical volume. Global neuron budget is no longer used.

### 5.3. Type Composition (Hard Quotas)

- From (draft): "Spawn probability of type A = 80%"
- To: "Budget of type A = exactly 80% of the layer population." Types are mixed in space using a deterministic algorithm (Shuffle based on `master_seed`).

---

## 6. Specification: `blueprints.toml`
Defines physical specifications for each neuron type. Uses Integer Physics to guarantee determinism and reduce FPU load.

**Dale's Law & Sign Invariance:** Synapse sign (Excitatory/Inhibitory) is IMMUTABLE. The sign is strictly determined by the pre-synaptic neuron's type definition.

### 6.1. Configuration Structure

```toml
[[neuron_type]]
# ID 0 (Bits: 00) - Primary Excitatory
name = "Vertical_Excitatory"

# --- Membrane Parameters (Units: microVolts / absolute integers) ---
threshold = 42000
rest_potential = 10000
leak_rate = 1200

# --- Timings (Units: Ticks) ---
refractory_period = 15
synapse_refractory_period = 15

# --- Signal Physics (Units: Integer geometry) ---
conduction_velocity = 200
signal_propagation_length = 10

# --- Homeostasis (Adaptive Threshold) ---
homeostasis_penalty = 5000
homeostasis_decay = 10

# --- Adaptive Leak Hardware ---
adaptive_leak_max = 500
adaptive_leak_gain = 10
adaptive_mode = 1

# --- Neuromodulation (R-STDP) ---
d1_affinity = 128
d2_affinity = 128

# --- Axon Growth (Steering & Arborization) ---
steering_fov_deg = 60.0
arborization_target_layer = "+1"
arborization_radius_um = 150.0
arborization_density = 0.8

sprouting_weight_distance = 0.5
sprouting_weight_power = 0.4
sprouting_weight_explore = 0.1

# --- Spontaneous Activity (Background noise) ---
spontaneous_firing_period_ticks = 500
```

### 6.3. DDS Heartbeat Compilation
`axicor-baker` converts the human-readable `spontaneous_firing_period_ticks` into a fractional multiplier `heartbeat_m` for the GPU:

```rust
// axicor-baker/src/compile_heartbeat.rs

pub fn compile_dds_heartbeat(period_ticks: u32) -> u16 {
    if period_ticks == 0 {
        return 0;  // Disabled
    }

    // DDS phase: 0..65535 (16 bits)
    let heartbeat_m = (65536 / period_ticks.max(1)).min(65535) as u16;

    // Bounds check
    assert!(heartbeat_m > 0, "period_ticks > 65536 is invalid (heartbeat too rare)");
    assert!(heartbeat_m <= 65535, "Critical: heartbeat_m overflow");

    heartbeat_m
}
```

### 6.4. Algorithmic Derivation of D1/D2 Receptors

- `d1_affinity` (LTP-like): Is_Excitatory  High (1.5x), Is_Inhibitory  Low (0.5x).
- `d2_affinity` (LTD-like): Is_Excitatory  Medium (1.0x), Is_Inhibitory  High (2.0x).

---

## 7. Specification: `brain.toml` (Multi-Zone Architecture)
Defines the topology of the multi-zone brain: which zones are present and how they synchronize.

### 7.1. Structure and Hierarchy
**Invariant:** Every zone references a `baked_dir` folder inside `Axicor_Models`. The absence of a file in `baked_dir` yields a critical initialization error (Must have: `.state`, `.axons`, `.gxo`).

### 7.3. `[[zone]]` Section

```toml
[[zone]]
name = "SensoryCortex"
blueprints = "Axicor_Models/mouse_agent/zones/SensoryCortex/blueprints.toml"
baked_dir = "Axicor_Models/mouse_agent/baked/SensoryCortex/"
```

### 7.4. `[[connection]]` Section: Inter-Zone Links
Defines Ghost Axon Projections.

```toml
[[connection]]
from = "SensoryCortex"
to = "HiddenCortex"
output_matrix = "sensory_out"
width = 64
height = 64
entry_z = "top"
target_type = "All"
growth_steps = 1000
```
