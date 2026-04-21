# Distributed Architecture

> Part of the Axicor architecture. Defines how multiple shards scale across a cluster using the Bulk Synchronous Parallel (BSP) model.

## 1. The BSP Model (Bulk Synchronous Parallel)

Axicor uses a strict BSP model to synchronize shards across the cluster.
Instead of locking threads or blocking the memory bus on every spike, the simulation runs completely autonomously for a defined `sync_batch_ticks` (e.g., 20 ticks = 2 ms).

1. **Concurrent Computation:** All GPUs process their local Day Phase (hot loops) simultaneously.
2. **Communication (Data Plane):** Spikes targeting network boundaries are accumulated in a local output buffer.
3. **Barrier Synchronization:** At the end of the batch, all nodes hit a strict barrier. Spike batches are exchanged over UDP/TCP.

### 1.1. Ghost Axons & Virtual Projections

A shard does not know about the network. The GPU operates exclusively on flat memory buffers and integer arithmetic.

- **Local Axons:** Belong to neurons residing in the current shard.
- **Ghost Axons:** Virtual representations of axons that belong to neurons in *other* shards, but have dendrites connected to them in *this* shard.

During the synchronization barrier, the orchestrator (`axicor-node`) updates the `BurstHeads8` arrays of ghost axons based on incoming network packets. The GPU simply sees a local array mutation and processes the overlap physics seamlessly in the next batch.

---

## 2. Network Protocol (Spike Batch V2)

[C-ABI] The L7 network protocol is designed for Zero-Copy DMA parsing directly into VRAM. The orchestrator receives the payload and copies it directly into the GPU memory without deserializing JSON or Protobuf.

```c
// [DOD FIX] L7 Protocol Header (Spike Batch V2)
struct alignas(16) SpikeBatchHeaderV2 {
    uint32_t src_zone_hash;
    uint32_t dst_zone_hash;
    uint32_t epoch;
    uint32_t is_last;
};

// [DOD] Network Packet (Strictly 8 bytes)
struct alignas(8) SpikeEvent {
    uint32_t ghost_id;
    uint32_t tick_offset;
};
```

- **`ghost_id`**: The local index of the ghost axon in the receiving shard's VRAM array. The sender maps its local neuron ID to the receiver's `ghost_id` via a routing table.
- **`tick_offset`**: The exact tick within the batch when the spike occurred. This guarantees temporal causality and exact active tail positioning despite the batched network delivery.

---

## 3. Ghost Axon Handover (Structural Plasticity)

During the Night Phase, structural plasticity (axonal growth) may cause an axon to physically cross the spatial boundary of its native shard and enter a neighboring shard. This triggers the Handover Protocol:

1. **Collision Detect:** The CPU identifies that the active axon tip (`PackedPosition`) has coordinates outside the local bounds (`world_offset` + `dimensions`).
2. **Transfer Request:** The local orchestrator packages the axon's structural data (rank, length, active segment) and sends a `HandoverRequest` to the target node.
3. **Ghost Allocation:** The target node allocates a new Ghost Axon in its `ghost_capacity` reserve and replies with the assigned `ghost_id`.
4. **Routing Update:** The source node updates its local routing table. Future spikes from this neuron will now be forwarded over the network to the target node's `ghost_id`.

**[INVARIANT]** The `ghost_capacity` VRAM reserve is strictly calculated and pre-allocated during the Baking pipeline (`ghost_capacity = width * height * 2.0`). Dynamic VRAM allocation (e.g., `cudaMalloc`) during the Day Phase or Night Phase is strictly PROHIBITED to prevent fragmentation and latency spikes.

---

## 4. Hardware Sympathy & Thread Pinning

To achieve high-frequency trading (HFT) speeds in the Day Phase, the `axicor-node` orchestrator employs strict OS Thread Pinning (CPU Affinity).

- Each shard's simulation loop is locked to a dedicated physical CPU core.
- **[SAFETY]** The `libc::sched_setaffinity` wrapper ensures that the OS scheduler does not migrate the thread to another core, keeping the L1/L2 cache hot for the entire simulation epoch.
- Network polling and UDP socket reads are isolated on separate cores to prevent I/O blocking from stalling the Integer Physics calculations.

## Strict BSP & Biological Amnesia

Axicor operates on a Bulk Synchronous Parallel (BSP) model. The cluster does not wait for slow nodes or delayed Python agents. 

**The Biological Amnesia Law:**
Every UDP packet on the Fast-Path contains an `epoch` counter (`SpikeBatchHeaderV2.epoch`). If a packet arrives with an `epoch < current_epoch`, the orchestrator SILENTLY DROPS it. The engine prioritizes the causality of the physics over data completeness. Late data is dead data.

## Self-Healing & RCU Routing

The cluster topology is dynamic and uses Read-Copy-Update (RCU) for Zero-Lock routing table swaps.

**1. Dynamic Routing (RCU):**
When a node joins or changes address, it broadcasts a `ROUT_MAGIC` (0x54554F52) packet containing a `RouteUpdate` struct. The orchestrator's Egress router performs an O(1) atomic pointer swap of the routing table (`Arc<RoutingTable>`). No mutexes block the hot loop.

**2. The Great Resurrection:**
If a node is isolated (BSP timeout > 500ms), it is considered dead. Upon restart, the Orchestrator initiates a Resurrection:
- Re-allocates VRAM.
- Restores the Hot State from `/dev/shm/*.shadow` using Zero-Copy `cudaMemcpyAsync`.
- Enters a 100-tick Warmup loop to stabilize membrane voltages before joining the active epoch.
