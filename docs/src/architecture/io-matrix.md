# I/O Matrix

> Part of the Axicor architecture. Unified abstraction for input/output and inter-zone connections.

## 1. Concept

The external world and neighbor zones interact with a brain zone through **matrices**  2D grids of fixed size `WH`.

- **Input matrix:** Each pixel = a virtual axon inside the zone.
- **Output matrix:** Each pixel = a bound soma inside the zone.
- **Inter-zone connection:** Output matrix of Zone A  input matrix (ghost axons) of Zone B.

Externally, all interfaces look identical: a flat array of IDs, addressed as `matrix[y * W + x]`.

> **[INVARIANT] Baking Freeze:** The I/O topology (which matrices, which axons, which somas) is **static after Baking**. Matrices cannot be added, removed, or resized at runtime. Recompilation = re-Baking.

### 1.1. Visual and Hardware Asymmetry (EnvRX / EnvTX)

In the graphical interface (Node Editor), environment matrices are strictly differentiated to protect macro-routing:
*   **EnvRX (Sensors / World Input):** Absolute sources of spikes. Color code: dark green. Hardware invariant: **physical absence of input ports** (left column).
*   **EnvTX (Motors / World Output):** Absolute sinks of spikes. Color code: dark red. Hardware invariant: **physical absence of output ports** (right column).
*   **Shard (Compute):** Blue-gray node. Has ports on both sides.

### 1.2. Hardware Alignment Contracts (C-ABI)

To ensure 100% Coalesced Access in the GPU and prevent PCIe bus crashes, I/O matrices must obey strict padding rules:
*   **Inputs (`Input_Bitmask`):** Strict **64-bit alignment (8 bytes)**. Bitmasks of virtual axons are padded with zeros so the payload size in bytes is a multiple of 8. This guarantees CPU and GPU read the mask via machine words without offsets.
*   **Outputs (`Output_History`):** Strict **64-byte alignment (L2 Cache Line)**. Raw spike arrays (`u8`, 1 byte = 1 spike) are padded with dummies so the matrix size is a multiple of 64 bytes. This allows the OS kernel and NIC to send data in monolithic transactions without tearing processor cache lines.

---

## 2. Input Matrix (Virtual Axons)

### 2.1. Placement

A `WH` matrix is **stretched** over the X-Y plane of the zone. Each pixel maps to a spatial region of the zone:

```text
region_x = pixel_x * (zone_width / W)
region_y = pixel_y * (zone_depth / H)
```

One pixel = one cluster of neurons. The number of somas falling into one pixel depends on zone architecture (zone size, density, matrix resolution).

### 2.2. Z-Spawning

A virtual axon is spawned at the computed (x, y) position at a configurable Z height:

```toml
# io.toml
[[input]]
name = "retina"           # Input name
zone = "SensoryCortex"    # Target zone
width = 64                # Matrix width in pixels
height = 64               # Matrix height in pixels
entry_z = "top"           # "top" | "mid" | "bottom" | specific value in um
target_type = "All"       # "All" | specific neuron type
growth_steps = 1500       # Max growth length (Cone Tracing steps)
empty_pixel = "skip"      # "skip" | "nearest" (fallback if region is empty)
stride = 1                # 0 = snapshot (tick 0 only), 1 = every tick, 2 = every 2nd tick...
```

### 2.3. Growth

From the spawn point, the virtual axon grows normally using the same Cone Tracing mechanism as local axons.
**Cost:** A virtual axon = one entry in `axon_heads[]` (4 bytes). PropagateAxons = one IADD per axon (microseconds even for millions). Dendritic contacts are limited by somas  128, not by axon count.
**Night Phase:** Virtual axons are immune to pruning. They are permanent infrastructure.

### 2.4. Spatial Mapping (UV-Projection)

Matrix mapping supports normalized UV-projection onto the physical space of the zone (in voxels). Parameter: `uv_rect = [u_offset, v_offset, u_width, v_height]`. Values are normalized [0.0, 1.0]. Default: `[0.0, 0.0, 1.0, 1.0]` (full coverage).

**Inverse UV Projection Algorithm (in Baker):**

1.  Compute physical soma position: `u_vox = vx / zone_width_vox`.
2.  Soma is culled (Early Exit) if `u_vox < u_offset` or `u_vox >= u_offset + u_width`.
3.  Local pixel coordinates: `local_u = (u_vox - u_offset) / u_width`.

**Runtime Isolation:** Since UV-projection is computed only at compile time (Baking Phase), the Day Phase HFT loop works with the generated flat array `mapped_soma_ids` and knows nothing about geometry. Zero-Cost abstraction.

### 2.5. External Interface

Externally, input matrices are a single flat bit array: `Input_Bitmask[tick][pixel_id / 64]` (64-bit blocks).
Setting a bit = resetting `axon_heads[virtual_offset + pixel_id] = 0` (signal birth). Everything else follows standard physics.

### 2.6. Bulk DMA & Stride (Autonomous Batch Execution)

The input mask is not streamed per tick. The entire batch is loaded into VRAM via a single asynchronous `cudaMemcpyAsync` operation before computations begin. The GPU spins a 6-kernel Autonomous Loop completely independent of the host, using offset pointers (`tick_input_ptr`) internally in O(1).

**Stride Parameter:** `stride = S` means the InjectInputs kernel receives an offset pointer to the mask every S ticks. Effective injections per batch = `sync_batch_ticks / S`.

### 2.7. Network Contract (C-ABI)

Data exchange between the environment (host) and node is strictly via UDP. Every packet must be prefixed with a 20-byte Little-Endian `ExternalIoHeader`. No JSON or Protobuf.

```c
// [C-ABI] Strictly 20 bytes. Little-Endian.
struct alignas(4) ExternalIoHeader {
    uint32_t magic;         // 0x4F495347 ("GSIO") for input, 0x4F4F5347 ("GSOO") for output
    uint32_t zone_hash;     // FNV-1a hash of the zone name
    uint32_t matrix_hash;   // FNV-1a hash of the I/O matrix
    uint32_t payload_size;  // Size of the bitmask payload (excluding header)
    int16_t  global_reward; // [DOD] R-STDP Dopamine Modulator (-32768..32767)
    uint16_t _padding;      // Alignment to 20 bytes
};
```

**Defragmentation:** UDP packets larger than 65507 bytes (MTU) are silently dropped.

### 2.8. Feature Pyramid Batching

The client formats inputs asymmetrically relative to the environment frequency. The recommended pattern is Feature Pyramid, where a multi-layer feature matrix (edges, colors, motion) is unrolled across the time axis (ticks).

-   Tick 0: Edges filter mask activation.
-   Tick 1: Color features activation.
-   Tick 2: Semantic tokens.
-   Ticks 3..N: Biological silence (Propagation Tail).

---

## 3. Output Matrix (Soma Readout)

### 3.1. Soma Capture

Each pixel covers a spatial region containing multiple somas. Selection is deterministically random (`master_seed ^ fnv1a(output_name) ^ pixel_index`). Mapping is 1 pixel  1 soma. Static, computed at Baking.

### 3.2. Output Data

Output = a deep spike batch of selected somas. Each spike takes exactly 1 byte (`u8`).

-   **VRAM Layout (GPU):** `Output_History[tick][pixel_id] = is_spiking;`
-   **Network Layout (Zero-Copy Transpose):** Sending `[Tick][Pixel]` over the network forces the Python client to loop and rebuild data per-motor (destroying cache performance). Before sending the UDP packet, the Rust orchestrator performs an ultra-fast CPU-cache transpose: `[Tick][Pixel]  [Pixel][Tick]`. This allows decoders to use `memoryview.reshape((N, Batch))` in O(1) without heap allocations.

### 3.3. Moving Window (Control FPS)

`output_history` is a 2D matrix. The External Hub reads it row-by-row with a `window_size` step to issue motor commands at `target_fps`.

---

## 4. Inter-Zone Connections (Ghost Axons Matrix)

An inter-zone connection is the exact same matrix interface: the output matrix of the source zone is projected as the input matrix (ghost axons) into the destination zone.

```toml
[[connection]]
from = "SensoryCortex"
to = "HiddenCortex"
output_matrix = "sensory_out"    # output matrix name in source zone
width = 16                       # projection resolution in destination
height = 16
entry_z = "top"
target_type = "All"
growth_steps = 750
```

**Scaling:** If the output matrix (3232) is larger than the projection (1616), each ghost-pixel captures a group of output pixels (pooling). If smaller  upsampling. Mapping is deterministic.

---

## 5. Limitations (UDP & VRAM)

Batch and matrix sizes are limited by network and memory physics.

-   **Max UDP Input:** `total_virtual_axons / 32  4  effective_ticks` must be < 65507 (IP/UDP MTU limit). Requires host fragmentation or TCP/SHM for massive streams.
-   **Max UDP Output:** `(WH for all [[output]])  sync_batch_ticks` must be < 65507.
-   **Input Bitmask Buffer:** `virtual_axon_count / 32  4` (per tick) allocated in VRAM.
-   **Output History Buffer:** ` mapped_soma_ids  sync_batch_ticks` (typically ~256 KB) in VRAM.
