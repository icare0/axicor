# Baking Pipeline

> Part of the Axicor architecture. Compilation algorithms for I/O and runtime initialization.
> Depends on: I/O Matrix.

## 1. Baker Pipeline

Baker compiles text configurations into binary files. The execution order is strict: zones first, then outputs, then inputs, then connections.

### 1.1. Phase A: Input matrices  `.gxi`

For each `[[input]]` in `io.toml`:

```python
def bake_inputs(zone, io_config) -> GxiFile:
    all_axons = []
    matrix_headers = []

    for input in io_config.inputs:
        offset = len(all_axons)
        matrix_headers.append({
            "name_hash": fnv1a(input.name),
            "offset": offset, "width": input.width, "height": input.height,
            "stride": input.stride,
        })

        for py in range(input.height):
            for px in range(input.width):
                # Region center in zone coordinates
                spawn_x = px * zone.width / input.width
                spawn_y = py * zone.height / input.height
                spawn_z = resolve_z(input.entry_z, zone)

                # Grow (standard Cone Tracing)
                seed = master_seed ^ fnv1a(input.name) ^ pixel_idx
                axon = grow_virtual_axon(
                    spawn=(spawn_x, spawn_y, spawn_z),
                    steps=input.growth_steps,
                    target=input.target_type,
                    cone=blueprints.virtual_axon_growth,
                    seed=seed,
                )

                # Handle empty pixel
                if axon.contacts == 0:
                    if input.empty_pixel == "nearest":
                        # retry with expanded radius
                        pass
                    else:  # "skip"
                        axon.local_id = INVALID

                all_axons.append(axon.local_id)

    return GxiFile(matrix_headers, all_axons)
```

**[INVARIANT]** Pixel order is strictly row-major (`pixel_id = py * width + px`). Matrix order matches the declaration order in `io.toml`.

### 1.2. Phase B: Output matrices  `.gxo`

For each `[[output]]` in `io.toml`:

```python
def bake_outputs(zone, io_config) -> GxoFile:
    all_pixels = []
    matrix_headers = []

    for output in io_config.outputs:
        offset = len(all_pixels)
        matrix_headers.append({
            "name_hash": fnv1a(output.name),
            "offset": offset, "width": output.width, "height": output.height,
        })

        for py in range(output.height):
            for px in range(output.width):
                # Pixel region (entire Z column)
                rx = px * zone.width / output.width
                ry = py * zone.height / output.height

                # Candidates: all somas in column, filtered by type
                candidates = zone.somas_in_column(rx, ry).filter(
                    lambda s: output.target_type == "All" or s.type_name == output.target_type
                )

                # Deterministic selection
                seed = master_seed ^ fnv1a(output.name) ^ pixel_idx
                soma_id = INVALID if not candidates else candidates[hash(seed) % len(candidates)]

                all_pixels.append(soma_id)

    return GxoFile(matrix_headers, all_pixels)
```

### 1.3. Phase C: Inter-zone links  `.ghosts`

**[INVARIANT]** Phase C MUST be executed after Phases A and B because it depends on the `.gxo` of the source zone.

For each `[[connection]]` in `brain.toml`:

```python
def bake_connection(conn, src_zone, dst_zone) -> GhostFile:
    # 1. Read GXO of the source zone
    src_gxo = load_gxo(src_zone, conn.output_matrix)

    # 2. For each ghost pixel
    ghosts = []
    for gy in range(conn.height):
        for gx in range(conn.width):
            # Scaling: ghost pixel  src pixel
            src_px = gx * src_gxo.width / conn.width
            src_py = gy * src_gxo.height / conn.height
            paired_src_soma = src_gxo.pixel_to_soma[src_py * src_gxo.width + src_px]

            # Spawn ghost axon in the destination zone
            spawn_x = gx * dst_zone.width / conn.width
            spawn_y = gy * dst_zone.height / conn.height
            spawn_z = resolve_z(conn.entry_z, dst_zone)

            seed = master_seed ^ fnv1a(conn.from) ^ fnv1a(conn.to) ^ ghost_idx
            ghost = grow_ghost_axon(
                spawn=(spawn_x, spawn_y, spawn_z),
                steps=conn.growth_steps,
                target=conn.target_type,
                seed=seed,
            )

            ghosts.append({
                "local_axon_id": ghost.local_id,     # index in axon_heads[] of destination
                "paired_src_soma": paired_src_soma,  # soma index in the source zone
            })

    return GhostFile(src_zone, dst_zone, width, height, ghosts)
```

**[INVARIANT]** `paired_src_soma` is a specific soma extracted from the GXO, not a random ID. The mapping is deterministic. The Ghost Axon strictly knows whom it listens to.

### 1.4. Phase D: Neurogenesis and Sprouting (In-place Growth)

The algorithm for forming new connections (both during initial Baking and by the Baker Daemon at night) MUST adhere to two hard invariants:

#### 1.4.1. Sprouting Density Invariant (GPU Visibility)

When searching for a free dendrite slot for a new connection, the CPU loop **MUST go forward (0..127)**.

- **Why:** The Day Phase kernel on the GPU relies on the Early Exit optimization (`if (target == 0) break;`).
- **Mechanics:** Night sorting always compacts live connections to the beginning of the array. If the Sprouting algorithm writes a new synapse to a random empty slot (e.g., 127) while slot 10 is zero (end of the dense block), the GPU will stop reading at slot 10 and will never see the new synapse.
- **Law:** Always write to the first encountered `target == 0`.

#### 1.4.2. Dead on Arrival Protection (Mass Domain Shift)

The starting weight (`initial_synapse_weight`) is fetched from `blueprints.toml` as a u16 (e.g., 1500). When written to VRAM/SHM, it **MUST be shifted into the Mass Domain**: `initial_weight << 16`.

- **Protection:** If `(initial_weight << 16) <= (prune_threshold << 16)`, the synapse is forcibly granted a survival capital (forced weight increase) to guarantee it survives the first night. Without this shift, the starting weight (1500) would be mathematically insignificant compared to the millions of units in the pruning threshold, causing the synapse to die immediately after birth.

---

## 2. Binary Formats

All files are strictly little-endian, uncompressed. Loading equals one `read()` + cast.

### 2.1. `.gxi` (Input Mapping)

[C-ABI] (synced with `axicor-core/src/ipc.rs`):

```text
+--------------------------------------+
| Header (32 bytes)                    |
|   magic:         u32 = 0x47584900    |  "GXI\0"
|   zone_hash:     u32                 |  FNV-1a of zone name
|   matrix_hash:   u32                 |  FNV-1a of matrix name
|   input_count:   u32                 |  Number of virtual axons
|   total_pixels:  u32                 |  W  H
|   _padding:      u32[1]              |  Reserved; always zero
+--------------------------------------+
| Axon Array (u32 per pixel)           |
| total_pixels  4 bytes               |
|   axon_local_id: u32                 |  index in axon_heads[] GPU array
+--------------------------------------+
```

### 2.2. `.gxo` (Output Mapping)

[C-ABI] (synced with `axicor-core/src/ipc.rs`):

```text
+--------------------------------------+
| Header (32 bytes)                    |
|   magic:         u32 = 0x47584F00    |  "GXO\0"
|   zone_hash:     u32                 |  FNV-1a of zone name
|   matrix_hash:   u32                 |  FNV-1a of matrix name
|   output_count:  u32                 |  Number of mapped somas
|   _padding:      u32[2]              |  Reserved; always zero
+--------------------------------------+
| Soma Array (u32 per pixel)           |
| total_pixels  4 bytes               |
|   soma_id:       u32                 |  index in flags[] / voltage[] GPU
+--------------------------------------+
```

### 2.3. `.ghosts` (Inter-zone Links)

[C-ABI]

```text
+-----------------------------------------+
| Header (20 bytes)                       |
|   magic:           u32 = 0x47485354     |  "GHST"
|   version:         u8  = 1              |
|   _padding:        u8[1]                |
|   width:           u16                  |
|   height:          u16                  |
|   _padding:        u8[2]                |
|   src_zone_hash:   u32                  |
|   dst_zone_hash:   u32                  |
+-----------------------------------------+
| Ghost Array (widthheight  8 bytes)    |
|   local_axon_id:   u32                  |  index in axon_heads[] of destination
|   paired_src_soma: u32                  |  soma index in the source zone
+-----------------------------------------+
```

### 2.4. `.paths` (Axon Full Geometry)

Stores the full geometry of axons as a flat 2D matrix. The axon length is hardware-limited to 256 segments because the `Segment_Offset` occupies exactly 8 bits in the `PackedTarget` structure.

[C-ABI] (Little-Endian, Zero-Copy Mmap Ready):

```text
+--------------------------------------+
| Header (16 bytes)                    |
|   magic:         u32 = 0x50415448    | "PATH"
|   version:       u32 = 1             |
|   total_axons:   u32                 |
|   max_segments:  u32 = 256           | Hardcoded limit
+--------------------------------------+
| Lengths Array                        |
|   lengths:       u8[total_axons]     | Current length of each axon
|   *padding:      align up to 64B     | L2 Cache Line alignment
+--------------------------------------+
| Segments Matrix (Flat SoA)           |
| total_axons  256  4 bytes          |
|   positions:     u32[A * 256]        | PackedPosition (X|Y|Z|Type)
+--------------------------------------+
```

**[INVARIANT] The 64-Byte Alignment Rule:** Global alignment of the length array up to a multiple of 64 mathematically guarantees that the lengths of all internal SoA arrays perfectly align with the L2 cache line without dirty padding bytes inside the blob. This is critical for Coalesced Access on AMD Wavefronts (64 threads) and eliminating cache thrashing.

---

## 3. WTA Distillation & Dual-Memory Split (Edge / ESP32)

To deploy compiled brains onto severely memory-constrained devices (like the ESP32-S3 with 520 KB SRAM), the engine uses WTA (Winner-Takes-All) Distillation.

- **Pruning:** The 128 slots are reduced to 32 slots.
- **Repacking:** The remaining 32 slots are repacked into new flat SoA arrays (`32 * padded_n`). Empty slots **MUST** be filled with zeros (`target = 0`  hardware Early Exit trigger).
- **Dual-Memory Split:** The data is split into two separate binaries:
  - `shard.sram` (Hot State): `voltage`, `flags`, `threshold_offset`, `timers`, `dendrite_weights` (32 slots), `dendrite_timers` (32 slots), `axon_heads`. Mapped/loaded strictly into fast SRAM.
  - `shard.flash` (Read-Only): `dendrite_targets` (32 slots) and `soma_to_axon`. Mapped via MMU directly from SPI Flash memory (XIP).

**[C-ABI] MMU INVARIANT (64KB):** The size of `shard.flash` MUST be padded with zeros to a boundary of 65536 bytes (64 KB). The `spi_flash_mmap` function on the ESP32 operates strictly in 64 KB pages. Any offset or unaligned file will cause hardware address shifting, causing the physics core to read garbage instead of topology.

---

## 4. Slow Path Protocol (TCP GeometryServer)

The protocol for transmitting structural graph changes (Night Phase) between nodes. Operates over TCP (default port 8010). Data is serialized via `bincode`.

**`GeometryRequest`** (Client  Server) Enum describing the intent:

- `BulkHandover(Vec<AxonHandoverEvent>)`: Transfers a batch of sprouted axons that crossed the shard boundary.
- `BulkAck(Vec<AxonHandoverAck>)`: Confirms the creation of Ghost Axons on the destination side (returns the allocated `dst_ghost_id` back to the source).
- `Prune(u32)`: Notification of connection deletion (passes `dst_ghost_id` for cleanup on the destination shard).

**`GeometryResponse`** (Server  Client) Enum, server response:

- `Ack(AxonHandoverAck)`: Returns a confirmation with the reserved slot (for single packets).
- `Ok`: Universal confirmation of successful processing (for `BulkHandover` and `Prune`).
