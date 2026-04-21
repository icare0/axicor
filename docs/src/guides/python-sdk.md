# Python SDK & Integration

> Part of the Axicor architecture. High-performance Data-Oriented integration layer.

## 1. The 10ms Rule & Zero-Garbage Law

The SDK (`axicor-client`) is an ultra-thin, Data-Oriented bridge designed to pump bits into VRAM via Zero-Copy IPC and UDP Fast-Path. High-level abstractions (OOP classes), JSON serialization, and heap allocations inside the hot loop are **strictly prohibited**.

Axicor operates on a strict Bulk Synchronous Parallel (BSP) lockstep. A standard environment synchronization step (`sync_batch_ticks`) is 100 ticks (10 ms). Your Python agent has exactly **10 milliseconds** to complete its observation-action loop:
1. Receive network response (UDP rx).
2. Compute environment physics (Gymnasium / Mujoco).
3. Encode new inputs into population codes.
4. Transmit the pulse back (UDP tx).

**The Zero-Garbage Law:**
You MUST NOT trigger the Python Garbage Collector (GC) during the hot loop. A single GC pause takes 15-20 ms, which immediately violates the BSP epoch barrier.
*   **Forbidden:** `np.array()`, `list.append()`, or creating any new Python objects inside `while True`.
*   **Required:** Pre-allocate all NumPy arrays and network buffers before the loop. Use `memoryview` and in-place operations exclusively (e.g., `np.clip(out=...)`, `np.subtract(out=...)`).

### 1.2. Strict BSP & Biological Amnesia
Interaction with the environment relies on **Bulk Synchronous Parallel (BSP)** synchronization (Lockstep).
If your Python script stalls and a packet arrives out of its assigned Epoch, the engine applies **Biological Amnesia**  it silently drops the outdated packet. The network will not wait for a slow agent forever.

---

## 2. The Zero-Garbage Hot Loop (Quickstart)

In the hot loop, you must use **Zero-Cost Facades** and pre-allocated buffers.

```python
import numpy as np
from axicor.client import AxicorMultiClient
from axicor.contract import AxicorIoContract

# 1. Load contracts and pre-allocate (Cold Start)
contract = AxicorIoContract("Axicor-Models/HftAgent/baked/SensoryCortex", "SensoryCortex")
cfg_in = contract.get_client_config(BATCH_SIZE=10)

client = AxicorMultiClient(addr=("127.0.0.1", 8081), **cfg_in)

# Pre-allocate flat arrays (Zero-Garbage)
obs_padded = np.zeros(64, dtype=np.float16)
bounds = np.zeros((64, 2), dtype=np.float16)

# 2. Bind Zero-Cost Facades to raw memory
avatar_in = contract.create_input_facade("sensors", obs_padded)
encoder = contract.create_population_encoder("sensors", vars_count=64, batch_size=10)

while True:
    # --- ZERO-GARBAGE HOT LOOP ---
    
    # 1. Write via Facade (O(1) pointer shift, no dicts)
    avatar_in.pos_x = state["x"]
    avatar_in.pos_y = state["y"]

    # 2. Vectorized normalization
    norm_state = np.clip((obs_padded - bounds[:, 0]) / range_diff, 0.0, 1.0)

    # 3. Transport to VRAM (1 C-ABI call)
    encoder.encode_into(norm_state, client.payload_views)
    rx = client.step(dopamine_signal=0)
```

---

## 3. Data Plane: UDP Fast-Path & C-ABI

Network exchange occurs via UDP flat chunks. Every chunk is prefixed with a strict 20-byte Little-Endian header.

```c
// [C-ABI] Strictly 20 bytes. Little-Endian.
struct alignas(4) ExternalIoHeader {
    uint32_t magic;         // 0x4F495347 ("GSIO") for rx, 0x4F4F5347 ("GSOO") for tx
    uint32_t zone_hash;     // FNV-1a hash of the zone name
    uint32_t matrix_hash;   // FNV-1a hash of the I/O matrix
    uint32_t payload_size;  // Size of the bitmask payload (excluding header)
    int16_t  global_reward; // [DOD] R-STDP Dopamine Modulator (-32768..32767)
    uint16_t _padding;      // Alignment to 20 bytes
};
```

**L7-Chunking and MTU:** If an output matrix exceeds the MTU limit (65507 bytes), the Rust orchestrator automatically slices it into L7-chunks aligned to 64 bytes (L2 Cache Line). `AxicorMultiClient` seamlessly reassembles these chunks in Python.

---

## 4. Encoders & Decoders

The SDK provides Zero-Copy encoders that convert environment float states into raw bitmasks via NumPy flat operations.

### 4.1. PopulationEncoder
Expands a single float variable into a population of M receptors (Gaussian Receptive Fields). Ideal for coordinates and velocities.

### 4.2. PwmEncoder (Rate Coding)
Encodes an f16 value (0.0 - 1.0) into the spike frequency of a virtual axon.
**[INVARIANT] Burst Gating Protection:** `PwmEncoder` applies a hardware phase shift (Golden Ratio Dither) so sensors do not fire simultaneously, preventing dendritic blockade via `synapse_refractory_period`.

### 4.3. RetinaEncoder (Event-Driven Vision)
Converts heavy RGB/Depth video streams into sparse bitmasks via lateral inhibition (Center-Surround Antagonism).
- **Zero-Garbage OpenCV:** All intermediate matrices (`_gray`, `_center`, `_surround`, `_dog`) are pre-allocated.
- **Feature Pyramid Batching:** The frame is sliced into features distributed across the temporal axis of the batch (Tick 0: Edges, Tick 1: Motion, Tick 2: Color opponent).
- **Warp Alignment:** The pixel array is hardware-padded to a 32-bit multiple (`math.ceil(N / 32) * 32`) to prevent out-of-bounds reading during `InjectInputs`.

### 4.4. PwmDecoder
Compresses the temporal sweep of a batch back into dense float values (Duty Cycle) for environment motors without memory allocation (`np.sum` across `axis=0`).

---

## 5. Dopamine Injection (Time-Scaled R-STDP)

Axicor learns via biologically plausible neuromodulation (R-STDP). The reward signal (Dopamine) is a global modulator sent in the header of every Data Plane UDP packet.
To prevent weight cementation, we apply Time-Scaled R-STDP:

- **Background Erosion:** Negative tone for slow destruction of unused connections (LTD).
- **Phasic Reward:** Positive burst for successful actions (LTP).
- **Pain Shock:** Prolonged maximum penalty upon critical failure (e.g., holding -255 for 15 batches to burn away responsible pathways).

The SDK hides byte packing in `client.step()`, but under the hood, the dopamine value (i16) is written directly into the `ExternalIoHeader` at offset 16 via `struct.pack_into("<IIIIhH", ...)`.

---

## 6. The Neurosurgeon (Memory Plane)

The `AxicorSurgeon` module is a Data-Oriented scalpel. It communicates with the network strictly via Zero-Copy mmap of OS files (`/dev/shm/axicor_shard_*`), bypassing the network stack and orchestrator.

**[SAFETY] ARCHITECTURAL LAW:** `AxicorSurgeon` MUST NOT be called inside the environment's hot loop. Operations on half-gigabyte arrays will break the 10ms Lockstep barrier. Use the surgeon only during initialization or offline distillation.

### 6.1. SDK Telemetry Translation (Mass -> Charge)
Weights are stored in the Mass Domain (32-bit integers up to 2.14B). However, `AxicorMemory.get_network_stats()` automatically translates this by dividing by 65536.0. Python always displays the Charge Domain (electrical charge in microvolts).

### 6.2. GABA Incubation (Storm Protection)
During a cold start (Tabula Rasa), the network may fall into an epileptic spike storm. We cure this by incubating Inhibitory synapses:

```python
surgeon.incubate_gaba(baseline_weight=-30000)
```

Synapse sign (Excitatory/Inhibitory) is IMMUTABLE. By setting a hard baseline weight for all synapses where the source neuron is inhibitory, the network stabilizes instantly.

### 6.3. Topology Distillation & Grafting
You can extract a learned skill (reflex path) and transplant it into a clean agent.

```python
# 1. Extraction (Donor)
payload = source_surgeon.extract_reflex_path(motor_soma_ids, prune_threshold=15000)

# 2. Injection & Monumentalization (Recipient)
target_surgeon.inject_subgraph(payload)
```

**Monumentalization:** Transplanted weights are artificially maximized to the 15th inertia rank (`abs(w) = 32767`) so the untrained recipient network does not immediately burn the implant via background depression.

**[C-ABI] The Zero-Index Trap:** When parsing `dendrite_targets` via mmap, the surgeon MUST account for the fact that `target == 0` is a hardware Early Exit trigger for the GPU. The real `axon_id` is always shifted by `+1`.

```python
# Unpacking from VRAM
axon_id = (target_packed & 0x00FFFFFF) - 1
segment_offset = target_packed >> 24

# Packing to VRAM (axon_id must be + 1!)
target_packed = (segment_offset << 24) | ((axon_id + 1) & 0x00FFFFFF)
```

> **FATAL WARNING:** If you pack `axon_id = -1` (empty slot) with a non-zero `segment_offset` without a strict mask, you will write a non-zero value to VRAM. The GPU will attempt to read an out-of-bounds `axon_id` (`0xFFFFFFFF`), resulting in an immediate Segmentation Fault and crashing the node.