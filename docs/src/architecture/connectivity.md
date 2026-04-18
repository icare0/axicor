# Connectivity

> Part of the Axicor architecture. Defines how neurons find each other, and how connections are born, live, and die.

## 1. Growth Rules and Wiring

### 1.1. Architecture: Pub/Sub

| Role | Analogy | Behavior |
|---|---|---|
| **Axon (Publisher)** | Radio Station | Broadcasts a signal into space (Active Tail + Global Buffer). It does not care if 0 or 1,000,000 dendrites are listening. **Does not store a list of listeners** (this would destroy memory). |
| **Soma/Dendrite (Subscriber)** | Radio Receiver | Decides which Axon ID to "tune into". Channel limit = 128 slots. |

The limit of 128 is a strict constraint on the **input bandwidth** of a specific neuron, not the output popularity of an axon.

### 1.2. Axon Classes

In `anatomy.toml` and `blueprints.toml`, axons are explicitly divided into three classes to avoid calculating redundant physics:

| Class | Description | Physics |
|---|---|---|
| **Local** | Local connections, inhibition clouds | Full 3D physics, collisions |
| **Horizontal** | Connections in the XY plane | Full physics constrained to the plane |
| **Projectors** (White Matter) | Long-range / Inter-zone links | Port Out + Port In + `Delay_Counter` only |

- **Projectors Optimization:** We simulate only the entry point (into the cable) and the exit point. The axon trunk **does not exist** in collision physics (saving VRAM and ALU cycles)  it exists solely as a `Delay_Counter`.

### 1.3. Terminal Arborization

Axon growth is a two-phase process. This biologically accurate branching mechanics radically increases the chances of dendritic contact without any GPU overhead during the Day Phase.

**Phase A: Trunk (Projection)**
The axon grows from the soma toward the target layer (`Target_Z`) strictly directionally. The `V_global` vector dominates  the axon flies to the target with minimal deviation (Cone Tracing).

**Phase B: Crown (Terminal Arborization)**
Upon reaching `Target_Z`, the behavior changes drastically:
- **`V_global` is disabled**  the global target is reached.
- **`V_noise` (jitter) is maximized**  the axon enters a chaotic looping mode.
- The axon tip wanders within `arborization_radius_um`, creating a dense cloud of 50100 segments inside the target layer.

**Zero-Cost for GPU:** 
3D voxel geometry exists **exclusively on the CPU**  during Baking and the Night Phase. In the Day Phase, the GPU sees the axon simply as a **flat 1D shift register (`BurstHeads8`)**. All arborization work is done by `axicor-baker` at night and baked into `shard.paths`. The runtime cost is exactly 0.

### 1.4. Dendrites & En Passant Synapses

- **Sprouting Candidate Search:** During the Night Phase, a growing dendrite looks for contacts not with axon tips, but with **any of their segments** (En Passant synapses).
- **O(K) Spatial Hashing:** The search uses a 3D Spatial Hash Grid containing the full history of axon paths (from `shard.paths`), replacing an O(N) random search with a Zero-Cost Spatial Search.
- **Hebbian Structural Rule:** A dendrite sprouts (looks for new synapses) **only if its soma was active during the day** (`flags[i] & 0x01 != 0`). This prunes 90-99% of idle CPU iterations during the Night Phase.

**[INVARIANT] Sprouting Density Invariant (The Dense Rule):**
When searching for a free slot for a new synapse, the loop on the CPU **MUST go forward (0..127)**. The `sort_and_prune` GPU kernel compacts live connections at the beginning of the array. The first encountered `target == 0` is the guaranteed end of the dense block and the only valid place to insert. Writing to the end of the array (e.g., slot 127) will make the synapse invisible to the Day Phase due to the Early Exit optimization (`if (target == 0) break;`).

**[INVARIANT] Dead on Arrival Protection (Mass Domain Shift):**
When a synapse is born, its initial weight (`initial_synapse_weight`) is automatically shifted into the Mass Domain (`<< 16`). If the resulting value is less than the pruning threshold (`prune_threshold << 16`), the synapse is forcibly granted additional "capital" (weight is increased) to guarantee it survives the first Night Phase and has time to learn.

### 1.5. Power Compensation and Dale's Law

The lack of multi-connectivity (limited to 128 slots) is compensated by a wide dynamic weight range (`i16`: -32768 to +32767). 

Synapse sign (Excitatory/Inhibitory) is IMMUTABLE. The sign is strictly determined by the Sender-Side Type (pre-synaptic neuron's type definition in `blueprints.toml`), NEVER by the receiving dendrite.

- **Scale:** A weak connection (just born) = `initial_synapse_weight` (default 74). A strong connection = up to 32,767. One "super-synapse" mathematically replaces a bundle of weak ones.
- **Dale's Law Invariant:** During Night Phase sprouting, if the axon's owner type has `is_inhibitory = true`, the dendrite MUST record a **negative weight** (e.g., -74). Inhibitory neurons can never form an excitatory synapse. GSOP only modifies the absolute value: `new_abs = abs(w) + delta`, result = `sign(w) * new_abs`.

### 1.6. Living Axons (Tip Nudging)

The graph is not static after Baking. During the Night Phase, axons continue to physically grow through the voxel grid (Structural Plasticity). To prevent this from locking the CPU with millions of axons, we use **Activity-Based Nudging**:

1. **Local Axons (Activity Gate):** The Baker Daemon reads `soma_flags`. If the 0th bit is 1 (the neuron spiked during the day), its axon tip is calculated and shifted by 1 segment. If silent, the CPU skips it in O(1).
2. **Ghost Axons (Inertial Nudging):** Ghost axons have no local soma (`soma_idx = usize::MAX`). They are nudged unconditionally every night along their inertia vector until `remaining_steps` reaches zero.

---

## 2. Plasticity & Math (Integer Physics)

### 2.1. The Learning Trigger: Active Tail Check

Learning is triggered **only** when the soma generates a spike (`is_spiking == true`). Instead of analyzing timestamps ("who fired 5 ms ago?"), we analyze the **current spatial state** of the Active Tail.

- **Causality:** The signal physically arrived  the soma fired  GSOP is triggered. Causality is verified through spatial overlap, not temporal tracking.
- **Rules:**
  - Soma spikes, dendrite contributed voltage (overlaps Active Tail)  `Weight += potentiation`.
  - Soma spikes, dendrite did NOT contribute  `Weight -= depression`.
  - Weight drops below 1  Synapse is removed (Pruning).

### 2.2. GSOP via LUT (Variant-Dependent Learning)

Specific GSOP values are **not hardcoded**. They are loaded into the **constant** memory of the GPU as a LUT by Type ID (strictly 16 types from `blueprints.toml`).

```cpp
// GPU Shader (Integer Physics)
uint8_t type_mask = flags[tid] >> 4;
VariantParameters p = const_mem.variants[type_mask];
```

### 2.3. Inertia Curves (Ranked Stability)
Instead of a binary "permanence bit", we use nonlinear resistance to change.

- **Mechanics:** The current `abs(weight)` is divided into 16 ranks (2048 units each). The curve is defined via the `inertia_curve: [u8; 16]` parameter in the config.
- **Concept:** The stronger the connection, the harder it is to change. "Young" synapses are highly plastic; "old" synapses are monumental.

| Rank | abs(weight) Range | Multiplier | Behavior |
| :--- | :--- | :--- | :--- |
| 03 | 0  8,191 | High | Fast learning, fast death |
| 47 | 8,192  16,383 | Medium | Stabilization |
| 811 | 16,384  24,575 | Low | Hard to change |
| 1215 | 24,576  32,767 | Minimal | Almost monumental synapse |

```cpp
// GPU (Integer Physics)
uint32_t rank = abs(weight) >> 11;               // 16 ranks (2048 each)
uint32_t inertia = p.inertia_curve[rank];        // Coefficient from constant memory
int32_t delta = gsop.potentiation * inertia;   // Higher rank = smaller delta
weight = sign(weight) * clamp(abs(weight) + delta, 0, 32767);
```

### 2.4. Nightly Sort (Columnar Defragmentation)
During the Night Phase (CPU), memory consolidation occurs:

- **GPU Sort & Prune:** A Segmented Radix Sort organizes the 128 slots by `abs(weight)` descending. Slots with `abs(w) < threshold` are zeroed out (`target = 0`).
- **Implicit Promote / Evict:** Because empty slots are guaranteed to sink to the end of the array, and live connections are sorted by strength, strong connections automatically occupy slots `0..ltm_slot_count` (LTM - Long Term Memory), while weak ones are pushed to the tail (WM - Working Memory).
- **Result:** After the night, the array is perfectly defragmented, allowing the Day Phase to leverage the Early Exit optimization.
