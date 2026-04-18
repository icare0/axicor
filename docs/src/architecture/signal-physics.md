# Signal Physics

> Part of the Axicor architecture. The complete path of the signal: input  propagation  output.

## 1. Day Phase Pipeline

Every tick executes a strictly sequential chain of CUDA kernels. The order is critical: each kernel depends on the results of the previous one.

```mermaid
sequenceDiagram
    participant Host
    participant GPU as GPU / CUDA Stream

    Host->>GPU: 1. InjectInputs (virtual axons control)
    GPU->>GPU: Reads input_bitmask, Writes axon_heads[virtual_axons] = 0
    GPU-->>GPU: Signal birth: virtual spikes

    GPU->>GPU: 2. ApplySpikeBatch (network spikes)
    GPU->>GPU: Reads ghost_indices from neighbors, Writes axon_heads[ghost_axons] = 0
    GPU-->>GPU: Signal birth: network spikes

    GPU->>GPU: 3. PropagateAxons (signal propagation)
    GPU->>GPU: Reads all axon_heads, Computes head += v_seg, Writes updated axon_heads
    GPU-->>GPU: Signals move along axons

    GPU->>GPU: 4. UpdateNeurons (GLIF kernel)
    GPU->>GPU: Reads voltage, flags, axon_heads, dendrite struct.
    GPU->>GPU: Executes: homeostasis  refractoriness  GLIF leak  columnar dendrite loop  threshold check  fire
    GPU->>GPU: Writes: voltage, flags (new spikes!). Triggers own axons.
    GPU-->>GPU: Soma fires or stays silent

    GPU->>GPU: 5. ApplyGSOP (plasticity)
    GPU->>GPU: Reads flags (spikes) + dendrite_timers (contacts)
    GPU->>GPU: Computes: potentiation/depression via causality. Writes updated dendrite_weights
    GPU-->>GPU: Synapses are potentiated or depressed

    GPU->>GPU: 6. RecordReadout (output accumulation)
    GPU->>GPU: Reads flags (soma spikes), Writes output_history[current_tick] = spikes
    GPU-->>GPU: Outputs accumulated in batch

    Host<<-GPU: End of tick (stream synchronization point)
```

**Execution Order (Dependency Chain):**

1. `InjectInputs`  virtual axons.
2. `ApplySpikeBatch`  network axons.
3. `PropagateAxons`  all axons move.
4. `UpdateNeurons`  dendrites collect  soma fires  own axons are born.
5. `ApplyGSOP`  weights updated (based on spike flag from UpdateNeurons).
6. `RecordReadout`  results written to output buffer.

## 2. Burst Train Model & Inference Pipeline
The signal is not a single point, but a burst of impulses sliding along the axon segments.

- **Axon state:** Stored as an array of 8 heads `BurstHeads8` (32 bytes, 1 cache line).
- **Constant:** `v_seg` (speed in segments/tick).
- **Update Loop:** Every tick for each head $h_i$: if $h_i \neq AXON\_SENTINEL$, then $h_i += v\_seg$.
- **Active Tail:** Segment $i$ is considered active if it falls within the tail of at least one of the 8 heads: $(h_k - propagation\_length) \le i \le h_k$

When a new soma spike occurs, the head register shifts: the oldest head $h_7$ is erased, and 0 is written to $h_0$.

### 2.1. Inference Pipeline (Early Exit)
Executed every tick for every dendrite. Uses an Early Exit strategy to offload the memory bus.

**Step 1. Refractory Gate (Cut-off on first read):**

```cpp
// Refractory timer - 1 byte per dendrite. 32 threads  1 byte = 32 bytes (1 L1 sector)
uint8_t timer = refractory_timer[slot * N_padded + tid];
if (timer > 0) {
    refractory_timer[slot * N_padded + tid] = timer - 1;
    return; // Early Exit: ~90% of ticks the dendrite "sleeps"  DO NOT read Global Memory of the axon
}
```

**Step 2. Overlap Check:**

- Read `Axon_Head_Index` from the global axon array.
- Check if the connected segment falls within the Active Tail interval.
- If yes:
  - `Soma_Voltage += Synapse_Weight`
  - `dendrite.timer = const_mem.variants[variant_id].synapse_refractory`

**[INVARIANT]** Synapse sign (Excitatory/Inhibitory) is IMMUTABLE.

## 3. Mass vs Charge Domain (Fixed-Point Downscale)
The engine separates the synaptic weight into two physical entities: structural mass (for learning) and electrical charge (for membrane dynamics).

- **Mass Domain (i32):** Synaptic weight is stored and mutated up to 2,140,000,000. All STDP gradients (GSOP) apply directly to this mass.
- **Charge Domain (16-bit shift):** In the hot loop of the UpdateNeurons kernel, the mass is converted to an electrical charge (microvolts) via a single arithmetic shift: `int32_t charge = w >> 16;`.

## 4. The Main Tick: UpdateNeurons (GLIF Kernel)
The kernel that gathers all physics in a single pass: GLIF leak, homeostasis, Early Exit, dendrite summation, threshold check, fire/reset. Parameters are read from `AxicorConstantMemory`.

```cpp
// 5. Columnar Loop: 128 dendrite slots (Coalesced Access)
for (int slot = 0; slot < 128; ++slot) {
    uint32_t col_idx = slot * padded_n + tid;

    // 5a. Refractory Gate (Early Exit - saves 3 global reads)
    uint8_t d_timer = dendrite_timers[col_idx];
    if (d_timer > 0) {
        dendrite_timers[col_idx] = d_timer - 1;
        continue;
    }

    // 5b. Empty slot - BREAK, not continue!
    // [INVARIANT]: After Night Phase Columnar Defrag, all empty slots (target=0) are at the tail.
    uint32_t target_packed = dendrite_targets[col_idx];
    if (target_packed == 0) break;

    // 5c. Branchless Active Tail Check (8 Heads)
    uint32_t axon_id = target_packed >> 10;
    uint32_t seg_idx = target_packed & 0x3FF;
    BurstHeads8 h = axon_heads[axon_id]; // [C-ABI] 32-byte coalesced read

    uint32_t prop = p.signal_propagation_length;
    bool hit = ((h.h0 - seg_idx) < prop) |
               ((h.h1 - seg_idx) < prop) |
               ((h.h2 - seg_idx) < prop) |
               ((h.h3 - seg_idx) < prop) |
               ((h.h4 - seg_idx) < prop) |
               ((h.h5 - seg_idx) < prop) |
               ((h.h6 - seg_idx) < prop) |
               ((h.h7 - seg_idx) < prop);

    if (hit) {
        int16_t w = dendrite_weights[col_idx];
        v += (int32_t)w;
        dendrite_timers[col_idx] = p.synapse_refractory_period;
    }
}
```

## 5. Warp-Aggregated Telemetry (Zero Atomics)
Extracting fired neuron IDs requires global atomic operations. To avoid blocking the L2 cache bus with 100,000 threads, Axicor uses Warp-Aggregated Atomics:

- **Ballot Sync:** Each thread in a warp exposes its spike bit via `__ballot_sync(0xFFFFFFFF, is_spiking)`.
- **Population Count:** The warp leader (lane 0) counts the total spikes via `__popc(active_mask)`.
- **Single Atomic:** Only the leader performs a single `atomicAdd` to global memory.
- **Shuffle Sync:** The leader distributes the offset via `__shfl_sync(0xFFFFFFFF, warp_offset, 0)`.
- **Write:** Threads write their IDs in parallel at the computed offsets.

**Result:** Global memory atomic transactions are reduced by a factor of 32.
