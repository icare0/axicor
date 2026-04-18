# Neuron Model

> Part of the Axicor architecture. The neuron as a unit: how it is born, its properties, and self-regulation.

## 1. Placement and Structure

### 1.1. Stochastic Generation

- **Method:** Neuron coordinates are generated randomly (Stochastic) to avoid grid anisotropy.
- **Goal:** To ensure uniform signal propagation in all directions (isotropy). A regular grid creates preferred directions where signals "run" faster along axes.
- **Density:** Specified globally (`global_density`, see Configuration), but clusters and voids can form locally  this matches biological reality.

### 1.2. Voxel Grid Binding

- **Rule:** 1 Voxel = Maximum 1 Neuron.
- **Guarantee:** During coordinate generation (Baking), *reject-sampling* is used with an Occupancy HashSet. If a randomly chosen voxel is already occupied by another neuron's aggregate, the position is regenerated with a modified seed. Limit  100 attempts, after which a warning is issued (occurs only at extreme densities).
- **Purpose:** The unique voxel index can be used as a Neuron ID for fast neighbor search (Spatial Hashing without additional data structures).

### 1.3. Dual Indexing System (Packed Position / Dense Index)

Coordinates are stored not as floats, but as integer indices. There are two ID spaces for different phases:

**CPU / Night Phase (Baking): Packed Position (u32)**

[C-ABI] Packed into 1 register for Cone Tracing, Spatial Hash, and routing:

```rust
// Unpacking: 1 ALU cycle
let type_mask = (packed >> 28) & 0xF;
let z = (packed >> 20) & 0xFF;  // Z-Mask: strictly 8 bits (0..255)
let y = (packed >> 10) & 0x3FF;
let x = packed & 0x3FF;

// Packing:
let packed = (type_mask << 28) | (z << 20) | (y << 10) | x;
```

**Bit Layout Visualization:**

```text
u32 packed_position = 0xABCDEFGH
+- Bits 31-28: Type (4 bits)     [A]        0..15 (16 types)
+- Bits 27-20: Z (8 bits)        [BC]       0..255 (256 depths = 6.4mm)
+- Bits 19-10: Y (10 bits)       [DEF]      0..1023 (1024 widths = 25.6mm)
+- Bits 9-0:   X (10 bits)       [GH]       0..1023 (1024 lengths = 25.6mm)
```

Example: `0x3A5C80F0 = Type:3, Z:165, Y:598, X:240`

**GPU / Day Phase (Hot Loop): Dense Index (u32)**

- Continuous index `0..N-1` with no "holes" from empty voxels.
- Size N is padded to a number multiple of 64 (Warp Alignment)  guaranteeing 100% Coalesced Access.
- All SoA arrays (`voltage[]`, `flags[]`, `threshold_offset[]`) are addressed by Dense Index.
- The 4-bit type is stored in `flags[dense_index] >> 4`.

**Link:** During Baking, the CPU generates mappings from `DenseIndex  PackedPosition` and `PackedPosition  DenseIndex` (Spatial Hash). In the Hot Loop, the GPU does not know about coordinates  it operates exclusively on the Dense Index.

---

## 2. Composite Typing (4-bit Typing)

The neuron type is determined by a 4-bit index (0..15), which is used as a direct index into the behavior profile table `VARIANT_LUT` in constant GPU memory.

### 2.1. Type Encoding

`type_mask` (4 bits, bits 4-7 in `flags[dense_index]`):

```text
Bits:  [7..4]
Field: Type (0..15)
```

```rust
// Unpacking: 1 ALU cycle
let type_mask = flags[dense_index] >> 4;  // Bits 4-7
let params = const_mem.variants[type_mask];  // Direct index into LUT
```

The neuron type is the ordinal index in the config `blueprints.toml`  `neuron_types[0..15]`.

#### 2.1.1. Full Bit Map in `flags[dense_index]`

All 8 bits with current and reserved usage:

| Bits | Field | Type | Semantics | Status |
| :--- | :--- | :--- | :--- | :--- |
| `[7:4]` | `type_mask` | u4 | Neuron type index (0..15). Used as a direct index into `VARIANT_LUT`. | [OK] Active |
| `[3:1]` | `burst_count` | u3 | [BDP] Serial spike counter (0..7). Incremented on every spike within a single batch. | [OK] Active |
| `[0:0]` | `is_spiking` | u1 | Instant Spike. 1 = soma generates a spike in the current tick. | [OK] Active |

**[INVARIANT] Burst-Dependent Plasticity (BDP) Invariants:** The `burst_count` counter is read by the plasticity kernel for nonlinear STDP multiplication. Flag assembly in CUDA is executed Branchless (O(1)): `vram.soma_flags[tid] = (flags & 0xF0) | (burst_count << 1) | final_spike;`

Counter reset is performed strictly with the mask `0xF1` (11110001_2) to clear bits [3:1] without affecting `type_mask` and the current spike flag.

Extraction example (1 ALU cycle):

```c
uint8_t flags_byte = flags[dense_index];
uint8_t type_mask = flags_byte >> 4;                 // Bits 7-4
uint8_t burst_count = (flags_byte >> 1) & 0x07;      // Bits 3-1
uint8_t is_spiking = flags_byte & 0x1;               // Bit 0
```

### 2.2. LUT Principle (Look-Up Table)

Instead of storing GSOP constants and membrane parameters in every neuron (memory waste), we use `type_mask` as an array index into the constant GPU memory.

```c
// GPU Shader (actual code)
uint8_t type_mask = f >> 4;                    // Bits 4-7
VariantParameters p = const_mem.variants[type_mask];  // Direct index
int32_t new_threshold = threshold + p.homeostasis_penalty;
```

This is a single instruction  constant memory is always in the L1 cache. Supports up to 16 unique profiles.

### 2.3. Behavior Variants (Blueprint-driven)

Each profile (0..15) is defined in the `blueprints.toml` file. The order of types in `blueprints.toml` dictates their index in `type_mask`. There are no restrictions on names or parameter counts within the 0..15 boundary.

### 2.4. Type Space

Up to 16 unique types (0..15), each with a full set of GLIF and GSOP parameters. During Baking, every neuron from `anatomy.toml` is assigned a `type_idx` (0..15). Upon loading into VRAM, `type_idx` is encoded into `flags[dense_index] >> 4`.

---

## 3. Homeostasis

Two defense mechanisms. They do not prevent the neuron from functioning but stop it from entering an epileptic state.

### 3.1. Hard Limit: Refractory Period

- **Variable:** `refractory_counter` (u8).
- **Mechanics:** After a spike, the counter is set to `refractory_period` (from the LUT via Variant ID). It decrements every tick. While > 0, the soma ignores all incoming signals.
- **Purpose:** Hard clamp  prevents physically impossible frequencies (>500 Hz) and protects the engine from infinite loops.

### 3.2. Soft Limit: Adaptive Threshold

Instead of adjusting synapse weights (computationally expensive), we regulate soma sensitivity.

- **Variable:** `threshold_offset` (i32), stored in the neuron.
- **Constants:** `homeostasis_penalty` and `homeostasis_decay`  not stored in the neuron. Fetched from the LUT via Type ID.
- **Mechanics (Integer Math):**
  - Spike: `threshold_offset += penalty` (harder for the neuron to fire repeatedly).
  - Every tick: `threshold_offset = max(0, threshold_offset - decay)`  branchless, no if statements.
  - Effective threshold: `threshold + threshold_offset`.
- **Behavioral effect (Habituation):** Constant stimulus (e.g., fan noise)  neuron reacts initially, then gets "bored" and goes silent. Strong new stimulus  pierces the elevated threshold. This forms the baseline for attention mechanisms.
- **Burst Mode:** If the stimulus is overwhelmingly strong, the neuron fires a burst of spikes (penalty accumulates, but every spike pushes through). Under continuous noise, the threshold skyrockets, silencing the neuron.

---

## 4. Spontaneous Activity (DDS Heartbeat)

Biological networks exhibit a baseline level of background activity (Spontaneous Firing Rate), required for maintaining homeostasis and synapse survival (GSOP).
To generate spontaneous spikes without storing a timer state in every neuron (which would destroy VRAM), we employ a Direct Digital Synthesis (DDS) / Fractional Phase Accumulator pattern.

### 4.1. Math (Zero-Cost Branchless)

The frequency is defined via a 16-bit multiplier `heartbeat_m` (0 = disabled). The phase is calculated mathematically on the fly:

```c
// 104729 - prime number for deterministic Spatial Scattering.
// Prevents the entire warp from firing a volley simultaneously.
uint32_t phase = (current_tick * heartbeat_m + tid * 104729) & 0xFFFF;
bool is_heartbeat = phase < heartbeat_m;
```

This requires exactly 4 ALU cycles (IMUL, IADD, IAND, ICMP) and 0 branches.

### 4.2. Pacemaker Physiological Contract

A spontaneous breakthrough (Heartbeat) is fundamentally different from a standard GLIF spike:

- **Signal goes to axon:** The neuron fires an impulse (`axon_heads[my_axon] = 0`).
- **GSOP is triggered:** The `is_spiking` flag is set to 1 (critical for synapse survival).
- **Membrane is NOT reset:** A Heartbeat DOES NOT reset `voltage`, DOES NOT zero out `refractory_timer`, and DOES NOT add a `homeostasis_penalty`. It is background noise that does not deplete the accumulated membrane potential.

**Final Logic:**

```c
// GLIF Spike (threshold-based)
int32_t is_glif_spiking = (voltage >= effective_threshold) ? 1 : 0;

// Heartbeat Spike (spontaneous)
uint32_t phase = (current_tick * heartbeat_m + tid * 104729) & 0xFFFF;
int32_t is_heartbeat = (phase < heartbeat_m) ? 1 : 0;

// Final Spike
int32_t final_spike = is_glif_spiking | is_heartbeat;

// State resets ONLY from a GLIF spike
voltage    = is_glif_spiking * rest_potential + (1 - is_glif_spiking) * voltage;
ref_timer  = is_glif_spiking * refractory_period;
threshold_offset += is_glif_spiking * homeostasis_penalty;

// Activity flag is set by ANY spike (required for GSOP)
flags = (flags & 0xFE) | (uint8_t)final_spike;
```
