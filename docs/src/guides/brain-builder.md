# Brain Builder

> Part of the Axicor architecture. HFT-Connectome generation and offline compilation.

In Axicor, you do not create neurons in Python RAM via `new Neuron()`. This would destroy the Garbage Collector (GC) and make Zero-Copy GPU loading impossible.

Instead, we use the **"Brain DNA"** pattern. Using the Python `BrainBuilder` class, you generate a strict topology as TOML configurations. Then, the `axicor-baker` utility "bakes" this DNA into flat binary C-ABI arrays (`.state` and `.axons`). This is the foundation of the Axicor Engine.

## 1. Lifecycle (Pipeline Clarity)

1. **Python Script (`builder.build()`):** Generates the directory hierarchy and TOML files (`anatomy.toml`, `blueprints.toml`, `io.toml`, `shard.toml`).
2. **Axicor Baker (CPU Compiler):** `cargo run -p axicor-baker -- --brain config/brain.toml`. Reads TOML, places neurons, "grows" axons through 3D space (Cone Tracing), and saves Warp-Aligned VRAM dumps.
3. **Axicor Node (GPU Runtime):** `cargo run -p axicor-node`. Performs `mmap` of the baked dumps directly into OS memory, bypassing allocations.

---

## 2. Creating an HFT Connectome (Reference)

When operating in a High-Frequency Trading (HFT) loop (100+ Hz) with environments like Gymnasium, baseline plasticity from the biological library will destroy the weights. We must suppress background growth and switch the network into **Reward-Gated Plasticity** mode (growth only upon dopamine injection).

```python
from genesis.builder import BrainBuilder

# 1. Initialize the architect
builder = BrainBuilder(project_name="HftAgent", output_dir="Genesis-Models/")

# Fine-tune physics for the HFT loop (1 ms barrier)
builder.sim_params["sync_batch_ticks"] = 10
builder.sim_params["tick_duration_us"] = 100

# 2. Create a zone (Width, Depth, Height in voxels)
# Default voxel size = 25 um
cortex = builder.add_zone("SensoryCortex", width_vox=64, depth_vox=64, height_vox=63)

# 3. Load types and HFT-tune plasticity
# In HFT mode, weight growth occurs ONLY upon dopamine injection (pot=0)
# Background depression (dep=2) slowly burns away random noise. This is critical for Axicor.
exc_type = builder.gnm_lib("VISp4/141").set_plasticity(pot=0, dep=2)
inh_type = builder.gnm_lib("VISp4/114").set_plasticity(pot=0, dep=2)

# 4. Populate layers (Bottom-Up design)
# WARNING: The sum of layer height_pct MUST strictly equal 1.0!
cortex.add_layer("L4_Input", height_pct=0.1, density=0.3) \
      .add_population(exc_type, fraction=0.8) \
      .add_population(inh_type, fraction=0.2)

cortex.add_layer("L23_Hidden", height_pct=0.6, density=0.15) \
      .add_population(exc_type, fraction=0.8) \
      .add_population(inh_type, fraction=0.2)

cortex.add_layer("L5_Motor", height_pct=0.3, density=0.1) \
      .add_population(exc_type, fraction=1.0)

# 5. Define Inputs and Outputs (I/O Matrix)
cortex.add_input("sensors", width=8, height=8, entry_z="top")
cortex.add_output("motors", width=16, height=8, target_type="All")

# 6. Compile DNA
builder.build()
```

---

## 3. How it Works Under the Hood (Data-Oriented Design)

- **No RNG for Populations (Hard Quotas):** The `add_population(fraction=0.8)` method establishes a strict quota. If the layer accommodates 1000 neurons based on volume and density, the engine is guaranteed to create exactly 800 neurons of one type and 200 of the other. Types are mixed using a deterministic shuffle based on the `master_seed`.
- **Matrix Mapping:** Inputs and outputs (`add_input`, `add_output`) are stretched across the 3D grid of the zone (UV-projection). A single matrix pixel covers a population of physical neurons. The selection of a specific soma for I/O is deterministically resolved by the Spatial Hashing algorithm.
- **C-ABI Alignment:** Baker automatically calculates `padded_n` and pads the neuron count with zeros up to a number multiple of 32 (Warp Alignment). The GPU will read this memory array without Divergence or Cache Misses.
- **Strict Dale's Law:** Synapse sign (Excitatory/Inhibitory) is IMMUTABLE. The sign is strictly determined by the pre-synaptic neuron's type definition (source). If a type is marked as `is_inhibitory = true` in `blueprints.toml`, all its axons will exclusively carry inhibitory signals. This decision is "cemented" by the Baker during binary dump generation. This restriction allows GPU kernels to perform GSOP plasticity without a single branch (Branchless Math), which is critical for the HFT loop.

---

## 4. Shift-Left Validation & Ergonomics

The Python SDK (`BrainBuilder`) acts as the first line of defense for C-ABI contracts. Validation must occur before `.toml` files are generated.

### 4.1. Interactive Auto-Fix
If the script is executed in an interactive terminal (`sys.stdout.isatty()`), upon detecting an architectural violation, the SDK MUST halt execution, display the current erroneous value, mathematically calculate the nearest valid options, and prompt the user to choose (number input or Enter for default auto-fix). In non-interactive environments (CI/CD), the script must crash with a `ValueError`.

### 4.2. Integer Physics Validation (`v_seg`)
The signal propagation speed MUST be a multiple of the segment length.

- **Invariant:** `v_seg = (signal_speed_m_s * 1000 * (tick_duration_us / 1000)) / (voxel_size_um * segment_length_voxels)`. The `v_seg` value MUST be strictly an integer.
- **UX:** If `v_seg` is fractional, the SDK suggests modifying either `signal_speed_m_s` or `segment_length_voxels`, displaying the exact recalculated valid values.

### 4.3. Topological Auto-Routing (MTU Fragmentation)
The user operates solely with logical matrix dimensions (e.g., `width=256`, `height=256`).

- **Rule:** The SDK calculates the payload size: `(width * height) / 8` bytes.
- **Fragmentation:** If the payload exceeds the specified MTU (default 65507 for PC, 1400 for ESP32), `BrainBuilder` automatically splits the logical matrix into N physical sub-matrices (chunks).
- **Mapping:** Each chunk is automatically assigned a spatial offset `uv_rect = [u_offset, v_offset, u_width, v_height]`, enabling `axicor-baker` to assemble them into a unified grid without overlaps.

### 4.4. 4-Bit Type Limit
Within a single zone (Shard), there can be a maximum of 16 unique neuron types, as `type_mask` occupies exactly 4 bits in the `soma_flags` array. Attempting to add a 17th type via `add_population` is instantly aborted with an error.

---

## 5. Defining Inputs and Outputs (I/O Matrix & Blueprint Wiring)

Inputs and outputs are bound to the physical layers of the zone. To avoid OOP overhead on the client side, we use semantic layout markup.

```python
# 8x8 matrix (64 virtual axons)
# The first 3 slots are bound to readable names, the remaining 61 are padded with zeros
cortex.add_input("sensors", width=8, height=8, entry_z="top", layout=[
    "pos_x", "pos_y", "angle_joint_0"
])

# 16x8 output matrix
cortex.add_output("motors", width=16, height=8, target_type="All", layout=[
    "motor_left", "motor_right"
])
```

**Layout Physics:** The Rust compiler (Data Plane) ignores these strings. It takes `width=8`, `height=8` and hardware-allocates exactly 64 slots, aligned to the cache line boundary (The 64-Byte Alignment Rule). The Python SDK (Control Plane) reads these strings from `io.toml` and dynamically generates a Zero-Cost Facade, where `pos_x` is a direct closure to a specific index in the `memoryview`.
