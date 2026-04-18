# 06.   (Distributed)

>   [Axicor](../../README.md). , BSP, , -.

**:** [MVP] =   V1, [PLANNED] =   V1.0 release.

---

## 1.   (Tile-Based Sharding)

### 1.1.  [MVP]

1. ** :**  -   (Sheet). 24   (Z)    (X/Y).  4     - .      - .
2. ** :**    (Columnar Layout, . [02_configuration.md 3](./02_configuration.md))   ,    (L1L6)    .      (L4  L2/3  L5  L6).  Z =      .
3. **  :**   V1 ()  PFC ()     (3),     .

### 1.2.   [MVP]

> [!NOTE]
> **[Planned: Macro-3D]**     3D (6 ).
//    4  (X+, X-, Y+, Y-).   -   `Z+ (Roof)`  `Z- (Floor)`.
// 
//  Z-Handovers:
//      `Z_max` ( ),      . 
//    `GhostPacket`    -  (`Z+`).

  -   (Tile).  **4 :**

```
        [North (Y+)]
            |
[West (X-)]-+-[East (X+)]
            |
        [South (Y-)]
```

  .    Z (L2  L5, L4  L6) - ** ** (Zero-Copy).

### 1.3. Ghost Connections (Ghost Axon Metadata) [MVP]

         (Connection).   MVP  (V1)     :

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GhostConnection {
    pub local_axon_id: u32,    //      (Ghost)
    pub paired_src_soma: u32,  // Soma ID   ( )
}
```

** :**
- **Baking Phase:** Baker   `.ghosts`   Connections,    .
- **Runtime:**  Connection  Ghost Axon,   ** **    (   ).
- **Sender-Side Mapping:**   soma  ,     `ghost_id` (  ),   ,          `axon_heads`.

**[MVP]  AxonHandover (V2):**
      ,  20- :

```rust
#[repr(C, packed)]
pub struct AxonHandoverEvent {
    pub origin_zone_hash: u32, //  ID -
    pub local_axon_id: u32,    // ID  -
    pub entry_x: u16,          // 3D-    
    pub entry_y: u16,
    pub vector_x: i8,          //   ( + )
    pub vector_y: i8,
    pub vector_z: i8,
    pub type_mask: u8,         // Geo(1) | Sign(1) | Var(2)
    pub remaining_length: u16, // TTL  ( )
    pub entry_z: u8,           // Z- 
    pub _padding: u8,          //   20 
} //  20 bytes

:  V1     .  V2      .        SHM.

#### Dynamic Capacity Routing (DCR)

   VRAM      (Ghost Axons)   ,     (Dynamic Capacity Routing).
 : `ghost_capacity = SUM(width * height) * 2.0` (      `brain.toml`   ,   2      ).
*:*  `200_000`    .     .

### 1.4. Ghost Axons ( ) [PLANNED]

> [!NOTE]
> **[Planned: Macro-3D]**   .
//     / RDMA, `BspBarrier`    : 
//  ->   -> Top-of-Rack Switch ->   ->  .

  [04_connectivity.md 1.7](./04_connectivity.md)    2D-:

- **:**   `AxonHandover`   Ghost Axon   .
- **:**  Ghost   ****  (X-  X+),  .
- ** :** Ghost Axon   /  Z  -   ,   .

### 1.5.   (Periodic Boundaries) [PLANNED]

    (,  )  **  **.     .

**:** GPU -   .   ** :**

- **1  (    GPU):** `Neighbor_X+ = Self`, `Neighbor_X- = Self`, `Neighbor_Y+ = Self`, `Neighbor_Y- = Self`. ,    ,     Ghost Axon **  **.
- **  (, 22):**     `Neighbor_X+ =   ` ( ).

** Z:**    Z   (L1 , L6 ) -  ****.    ( `height_pct = 1.0`   ) - Z   X/Y.

**  Ouroboros ( ):**  `remaining_segments`  `AxonHandover` (1.3)      .  `remaining_segments == 0` -  .       ,      .

**:**     .  .     Hot Loop.

---

## 2.   (Time Protocol) [MVP]

     Asynchronous Epoch Projection (AEP).

### 2.1. : Autonomous Epoch Execution [MVP]

-   GPU        .   .
-           (         ).
-       PCIe.    GPU -       .

### 2.2. Latency Hiding ( ) [MVP]

       ().

-        `ElasticSchedule` (2.8).
- **:**      .      .

#### 2.2.1. HFT Log Throttling & BSP Timings

**:** Day Phase  GPU  100+    (sync_batch_ticks = 100, 10  ).    ,  Egress-,  Self-Heal event     stdout   .

**: Atomic Throttling (Lock-Free)**

    `AtomicUsize` (thread-safe  Mutex)   **   100 **:

```rust
pub struct HftLogger {
    dopamine_count: AtomicUsize,
    egress_packet_count: AtomicUsize,
    self_heal_event_count: AtomicUsize,
    throttle_rate: usize,  // 100
}

impl HftLogger {
    pub fn log_dopamine(&self) {
        let count = self.dopamine_count.fetch_add(1, Ordering::Relaxed);
        if count % self.throttle_rate == 0 {
            eprintln!("[Dopamine] Batch #{}, cumulative: {}", batch_id, count);
        }
    }
    
    pub fn log_egress(&self, packet_size: u32) {
        let count = self.egress_packet_count.fetch_add(1, Ordering::Relaxed);
        if count % self.throttle_rate == 0 {
            eprintln!("[Egress] {} packets sent, last size: {} bytes", count, packet_size);
        }
    }
    
    pub fn log_self_heal(&self, node_id: u32) {
        let count = self.self_heal_event_count.fetch_add(1, Ordering::Relaxed);
        if count % self.throttle_rate == 0 {
            eprintln!("[SelfHeal] Node {} recovered, cumulative: {} events", node_id, count);
        }
    }
}
```

** :**

1. ** :** `fetch_add(Ordering::Relaxed)` -  CAS-  ALU, ~2 .  ,  .
2. ** :**   100-    stderr.  100  (1  )      ,   s.
3. **  logs:**    .    (cumulative: 1247),   .

#### 2.2.2. BSP Synchronization Timeout

**:** **BSP_SYNC_TIMEOUT_MS = 500 ms** ( 50 ms).

** :**

-        1020   (, I/O, CPU scheduler jitter).
-  `sync_batch_ticks = 100` (10  )  = 12  .
-   50  = 5 ,      timeout  false panic.
- **500  = ~50  **        CPU thrashing.

**  :**

```rust
const BSP_SYNC_TIMEOUT_MS: u64 = 500;
const MAX_BATCHES_LATENCY: u64 = BSP_SYNC_TIMEOUT_MS / BATCH_DURATION_MS;  // ~50 

pub fn wait_for_neighbors(
    wait_strategy: WaitStrategy,
    start_time: Instant,
) -> Result<Vec<SpikeBatch>, BspTimeoutError> {
    loop {
        if let Some(batch) = try_recv_all_neighbors() {
            return Ok(batch);
        }
        
        if start_time.elapsed().as_millis() > BSP_SYNC_TIMEOUT_MS {
            return Err(BspTimeoutError {
                timeout_ms: BSP_SYNC_TIMEOUT_MS,
                batch_count_allowed: MAX_BATCHES_LATENCY,
            });
        }
        
        match wait_strategy {
            WaitStrategy::Aggressive => std::hint::spin_loop(),
            WaitStrategy::Balanced => std::thread::yield_now(),
            WaitStrategy::Eco => std::thread::sleep(Duration::from_millis(1)),
        }
    }
}
```

**:**

|  | 50  | 500  |
|---|---|---|
|   | 5 | 50 |
|   - |  (  ) |  (   0.5 ) |
| Performance Impact |  |  (timeout -  ) |

** ?**      > 500     deadlock    .  ,    .

### 2.3. Fast Path & L7 Fragmentation (V2)

UDP    MTU (    `MAX_UDP_PAYLOAD = 65507`).      .       (L7)  .

2.3.1.  L7  (Dynamic MTU)

  `MAX_EVENTS_PER_PACKET = 8186` .     (High-End PC  )         `mtu`  `RouteUpdate`.

*   ** :** `max_spikes = (peer_mtu - sizeof(SpikeBatchHeaderV2)) / sizeof(SpikeEventV2) = (peer_mtu - 16) / 8`
*   **Tier 1 (PC / Server):**  MTU = 65507 ( 8186    UDP-).
*   **Tier 2 (ESP32 LwIP):**  MTU = 1400 ( 173   ).    SRAM  LwIP   `ERR_MEM`   IP- 65- .       .

### 2.3.2   V2: SpikeBatchHeaderV2 & SpikeEventV2

```rust
//  16 . align(16)   Zero-Cost 
//      LwIP    GPU.
#[repr(C, align(16))]
pub struct SpikeBatchHeaderV2 {
    pub src_zone_hash: u32,    // Hash  
    pub dst_zone_hash: u32,    // Hash  
    pub epoch: u32,            //    (Epoch)
    pub is_last: u32,          // 0 = , 1 =   (Heartbeat), 2 = ACK-
}

//  8 .
#[repr(C, align(8))]
pub struct SpikeEventV2 {
    pub ghost_id: u32,    //      
    pub tick_offset: u32, //    (0..sync_batch_ticks)
}
```

 Heartbeat:         ,       is_last = 1.   BSP-       .

#### 2.3.3. Zero-Copy Pipeline (Legacy MVP,   )

**:** `cudaMemcpy (VRAM  RAM)`  UDP  ( )  RAM        `&[SpikeEventV2]`.

    `SpikeBatchHeaderV2` + `SpikeEventV2[]` ( 8186 ).    > `MAX_EVENTS_PER_PACKET`,   ,     `is_last = 0`,  - `is_last = 1`.

2.3.4.   (Zero-Cost Transport)

**:**   Axicor Data Plane (Fast Path UDP)     RFC (Network Byte Order / Big-Endian).
      (`SpikeBatchHeaderV2`, `SpikeEventV2`)       **  Little-Endian** (   ).

**:** x86_64, ARM   Xtensa LX7 (ESP32)   Little-Endian.  `ntohl`/`htonl`   100 000      `pro_core_task`     (The 10ms Rule).       LwIP   Zero-Cost cast: `(SpikeEvent*)rx_buffer`

### 2.4. Sender-Side Mapping & Dynamic Capacity Routing [MVP]

  **  **    (Hot Loop).      ID  `receiver_ghost_id` (   VRAM ).

     (      )    VRAM,   **Dynamic Capacity Routing**.

1. ** VRAM (Pre-allocation):**
      (IntraGPU  InterNode),   (`src_indices_d`  `dst_ghost_ids_d`)      ,   - `max_capacity` ( `ghost_capacity`    `manifest.toml`).
           `count`.

2. **Hot-Patching (Sprouting):**
         Ghost Axon ( `AxonHandoverAck`), :
   -    `(src_axon, dst_ghost)`     .
   -  `count += 1`.
   -  -DMA (`cudaMemcpyAsync`)  8  (2 u32) **   **.
   -     `extract_outgoing_spikes`     `count`.

3. **Swap-and-Pop (Pruning):**
      (`AxonHandoverPrune`),    VRAM (O(N))   -   .
    O(1) :
   -        (Swap).
   -  `count -= 1`.
   -  -DMA (8 )       VRAM.

**:**  `cudaFree` / `cudaMalloc`  .       .       BSP,  6Day- .

### 2.5.   Ghost Axon [MVP:  + , PLANNED: ]

4 :       .

** 1:  (Slow Path / Night Phase)**

** 1:    (Slow Path / Night Phase)**

1.   A  3D- (X, Y  Z).   ,    SHM  `AxonHandoverEvent`.
2.    ( `run_sprouting_pass`)  B    `handovers_count`.
3. CPU  B  ** Ghost Axons**:  O(N)   `padded_n .. padded_n + total_ghosts` (   L1 )     (`tip == 0`,    ).
4.   ,   `entry_x/y/z`  `vector_x/y/z`,    `axon_tips_uvw`  `axon_dirs_xyz`     0.  Ghost Axon      .

** 2:  (Fast Path / Day Phase)**

1.   A   CPU  `SpikeBatch`   `ghost_id` (Sender-Side Mapping).
2.  B   `ghost_indices[]`   `ApplySpikeBatch`: `axon_heads[ghost_id] = 0`  O(1).
- ** :**  `ApplySpikeBatch`   `ghost_id`      (`axon_heads[ghost_id]`),    `tid` .   O(1)     .

### 2.5.1. Intra-GPU Ghost Sync (Zero-Copy L2 Routing)

   ,      GPU (, 8   ),  Fast Path   PCIe    .

- ** (Temporal Age Extraction):**  `cu_ghost_sync_kernel`  32-  `BurstHeads8`  -    L1 .
- **Sender-Side Filtering:**    `BurstHeads8`  Ghost- **       ** (`head / v_seg < sync_batch_ticks`).          .
- ** :**   8       ( `h7`  `h0`).  ,     (Burst)           .   (White Matter Delay)     1 .
- **Zero-Cost:** 0 , 0    PCIe.           L2.

** 3: **

`PropagateAxons`   Ghost  (`+= v_seg`). `UpdateNeurons`   B  Active Tail  `dist = head - seg_idx` -    .

** 4:  (Night Phase)**

  `PruneAxon(ghost_id)`,  B        ,     `AXON_SENTINEL` (0x80000000)  `axon_heads[ghost_id]`  VRAM.         GPU- PropagateAxons.     B       .

### 2.6. Slow Path:  [PLANNED]

 `AxonHandover` (1.3)  **  K ** (,   1050 ).   -  .    100   .    .

### 2.7. Main Loop [MVP: Compute + Network, PLANNED: Geometry]

```python
while simulation_running:
    # 1. Compute Phase (GPU)
    for t in range(sync_batch_ticks):
        current_tick += 1
        apply_remote_spikes(current_tick)   #  Schedule (Ring Buffer)
        physics_step()
        collect_outgoing_spikes()

    # 2. Network Phase (CPU/IO) - 
    send_buffers_to_neighbors()             # Zero-Copy Flush
    incoming_data = wait_for_neighbors()    # 

    # 3. Map Phase
    schedule_spikes(incoming_data)           #  Ring Buffer
```

### 2.8. Ring Buffer Schedule [MVP]

 2D  -  Priority Queue,   :

```rust
//    ( CPU,   VRAM)
struct SpikeSchedule {
    // schedule[tick_offset][slot] = ghost_id
    buffer: Vec<Vec<u32>>,      // [sync_batch_ticks][MAX_SPIKES_PER_TICK]
    counts: Vec<u32>,           //     
}
```

** (Map Phase, CPU):**   `SpikeBatch`   `tick_offset`  `SpikeEvent`:

```rust
for event in incoming_batch {
    let ghost_id = event.receiver_ghost_id as usize;
    let tick = event.tick_offset as usize;
    
    //    ID (/ /)
    //   1   CPU,  Hot Loop  GPU
    if ghost_id >= local_axons && ghost_id < (local_axons + ghost_axons) && tick < sync_batch_ticks {
        schedule.buffer[tick][schedule.counts[tick]] = ghost_id as u32;
        schedule.counts[tick] += 1;
    } else {
        // Log telemetry: dropped invalid spike (security/corruption warning)
    }
}
```

** (Day Phase, GPU):**    `ApplySpikeBatch`  ** ** `schedule[current_tick_in_batch]`,    `counts[tick]` .

### 2.8.1. Epoch Synchronization & Biological Amnesia

  -, BSP-     `epoch` ( ).  UDP    ,     :

#### Biological Amnesia (Drop)

    `header.epoch < current_epoch`,  ** **.

```rust
fn process_spike_batch(
    packet: &SpikeBatchHeaderV2,
    current_epoch: Atomic<u32>,
) {
    let curr = current_epoch.load(Ordering::Acquire);
    
    if packet.epoch < curr {
        //  " " -  
        // :  ,  ,  DROP
        return;
    }
    
    //   
    apply_spike_events_to_schedule(packet);
}
```

**  :**  " "        -  GSOP (      ).  -  **  ** (    ).

#### Self-Healing (Fast-Forward)

    `header.epoch > current_epoch`,  ,  **     Heartbeat**.     ,        ,    :

```rust
fn self_heal_from_network(
    packet: &SpikeBatchHeaderV2,
    current_epoch: &AtomicU32,
) {
    let curr = current_epoch.load(Ordering::Acquire);
    
    if packet.epoch > curr {
        //  ""  . Fast-forward:
        eprintln!("[SelfHeal] Epoch jumped from {} to {}", curr, packet.epoch);
        
        //  Ring Buffer (     )
        flush_spike_schedule();
        
        // Atomically  epoch
        current_epoch.store(packet.epoch, Ordering::Release);
    }
}
```

**:**   ,     "" ,   .   (Biological Amnesia) -   ,   **  ** (       ).

**:** `epoch`     ( ).  ,      ,    .

### 2.9. Bulk DMA & Autonomous Batch Execution [MVP]

  Tokio  GPU    Ping-Pong  (`BspBarrier`).  PCIe    .

**:   Host-Device**

1. **Bulk H2D (Host-to-Device):**
      `Input_Bitmask`  `SpikeSchedule`   VRAM  ****   `cudaMemcpyAsync`. 
   - : `(input_bitmask_size + schedule_size)` 
   - : <1   PCIe 4.0 x16  100  
   - : `cudaMemcpyAsync(d_input, h_input, size, 0, stream)`  non-blocking 

2. **Autonomous GPU Loop:**
   GPU  **6- ** (`sync_batch_ticks` )    :
   ```
   for tick_in_batch in 0..sync_batch_ticks:
       InjectInputs        (Virtual Axon pulse)
       ApplySpikeBatch     (Ghost Axon activation)
       PropagateAxons      ( IADD   )
       UpdateNeurons       (GLIF + spike check)
       ApplyGSOP           (Plasticity)
       RecordReadout       (Output snapshot)
   ```
   - **   `cudaDeviceSynchronize()`**  
   -     
   - **:**   GPU SM,  

3. **Pointer Offsetting (O(1)):**
      CUDA        :
   ```cuda
   __global__ void PropagateAxons(...) {
       int tid = blockIdx.x * blockDim.x + threadIdx.x;
       if (tid >= total_axons) return;
       
       // tick_input_ptr = base_ptr + (tick_in_batch * stride)
       // O(1) ,  
       uint32_t *tick_schedule = schedule_base + (tick_in_batch * max_spikes_per_tick);
       int head = axon_heads[tid];
       axon_heads[tid] = head + v_seg[tid];  // Propagate
   }
   ```

4. **Bulk D2H (Device-to-Host):**
      -   :
   ```cuda
   cudaMemcpyAsync(h_output, d_output_history, output_size, cudaMemcpyDeviceToHost, stream);
   ```
   - :  host   N, GPU   N+1
   - Network Phase     BSP barrier

**:**  .  -  .  4 -    (100 ms  4 DMA).

### 2.10. WaitStrategy: CPU Profiles [MVP]

**:**       (BSP Barrier,  ).   -, yield       .

**3  ( `--cpu-profile`):**

|  |  |  |  CPU |  |
|---|---|---|---|---|
| **Aggressive** | `spin_loop()` | ~1  | 100%  | Production / HFT |
| **Balanced** | `yield_now()` | ~115  | .  OS | ,   |
| **Eco** | `sleep(1ms)` | ~15  | ~0%  | ,  |

**  Network Phase:**

```rust
pub enum WaitStrategy {
    Aggressive,  // spin_loop()
    Balanced,    // yield_now()
    Eco,         // sleep(1ms)
}

pub fn wait_for_neighbors(wait_strategy: WaitStrategy) -> Vec<SpikeBatch> {
    loop {
        //      
        if let Some(batch) = try_recv_all_neighbors() {
            return batch;
        }
        
        match wait_strategy {
            WaitStrategy::Aggressive => std::hint::spin_loop(),
            WaitStrategy::Balanced => std::thread::yield_now(),
            WaitStrategy::Eco => std::thread::sleep(std::time::Duration::from_millis(1)),
        }
    }
}
```

** :**

1. **Spin-loop :** BSP  -  ,      ( ).  Mutex,  atomics-loop.
2. **   GPU:** Network Phase -  CPU, GPU   (Autonomous Loop, 2.9).
3. **:**   - runtime,   config  CLI.      .

### 2.11.    [MVP]

|  |  |
|---|---|
| **** |        100 ,  100  |
| **** |      `sync_batch_ticks` - --  ,     |
| **** |    -   (Wall Clock Speed ),   (GSOP, )    |
| **Zero-Copy** | Hot Loop : O(1)    ,   |

---

## 3.   (White Matter Routing) [PLANNED]

 (,   FLNe- ) - **  **.    CPU    Baking.    (Hot Loop)  GPU   .    ** **.

### 3.1.  (Hard Quotas)

     .  ** **.

-   ,   V1    V2  17%,    100 000  - Compiler Tool   ** 17 000** -  V1.
-    ** **   `master_seed` (. [02_configuration.md 5.3](./02_configuration.md)).

### 3.2.   (UV-Projection)

    .    .

- **UV-:**  -    `0.0..1.0` (`U`, `V`).
- **:**   = `U  Target_Width`, `V  Target_Height`.
- **:**     - ,    V1,    V2.

### 3.3.   (Jitter)

    Ghost Axons   .

-          ,     ** :**

```
Target_X += Hash(master_seed + soma_id) % jitter_radius
Target_Y += Hash(master_seed + soma_id + 1) % jitter_radius
```

-      `master_seed` -  .

### 3.4. : Zero-Cost Routing

1. **  (V1):**    (`Port Out`).
2. **  (V2):**   Ghost Axons    (`Target_X`, `Target_Y`).   ** **   ,    Cone Tracing (4  [04_connectivity.md](./04_connectivity.md)).
3. **:** `Delay_Ticks`         .    () ****     -    .

### 3.5. Dynamic Routing (Read-Copy-Update)

      .        (Resurrection)   .

** ** (`RoutingTable`)    **RCU (Read-Copy-Update)**,  **0    :**

- **Read:** Egress-    `AtomicPtr` (O(1),  ).
- **Copy-Update:**    `RouteUpdate` (Magic: `0x54554F52` / "ROUT"),   -,  IP:Port,    `swap` .
- **Deferred Cleanup:**     `tokio::spawn`   100 , ,    Egress-   .

####   RouteUpdate

```rust
#[repr(C)]
pub struct RouteUpdate {
    pub magic: u32,      // 0x54554F52 ("ROUT")
    pub zone_hash: u32,  // ID  
    pub new_ipv4: u32,   //  IP  u32 (network byte order)
    pub new_port: u16,   //  
    pub mtu: u16,        // [DOD FIX]  MTU  L7-
    pub cluster_secret: u64, //    
}  // = 24 bytes
```

** MTU:**
-  `mtu`      L7-   .
-  `mtu`  0    (legacy),       (65507).

#### RCU Implementation

```rust
pub struct RoutingTable {
    //     
    //     load(Ordering::Acquire)
    ptr: AtomicPtr<HashMap<u32, SocketAddr>>,
}

impl RoutingTable {
    pub fn lookup(&self, zone_hash: u32) -> Option<SocketAddr> {
        // O(1)   
        let table = unsafe { &*self.ptr.load(Ordering::Acquire) };
        table.get(&zone_hash).copied()
    }
    
    pub fn update(&self, zone_hash: u32, new_addr: SocketAddr) {
        // Copy:   
        let old_ptr = self.ptr.load(Ordering::Acquire);
        let mut new_table = unsafe { (*old_ptr).clone() };
        
        // Update:   
        new_table.insert(zone_hash, new_addr);
        let new_ptr = Box::into_raw(Box::new(new_table));
        
        // Swap:   
        let old_ptr = self.ptr.swap(new_ptr, Ordering::Release);
        
        // Cleanup (deferred):     100 
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            drop(unsafe { Box::from_raw(old_ptr) });
        });
    }
}
```

**:**

1. **  (Egress):** `lookup()` -    ,  . CAS + Acquire ordering = ~3   x86.
2. **  (Control Plane):** `update()`   (  );   CPU  copy+swap,      .
3. **:** Deferred cleanup ,   Use-After-Free:            (100  >> max jitter).

---

## 4.    V1 (MVP)

### 4.1.    [MVP]

**Multi-Node (Cluster) & Single-Node:**
-      GPU    UDP Fast-Path (`InterNodeChannel`).
-   GPU    Zero-Copy VRAM  (`IntraGpuChannel`).
- L7-  (`SpikeBatchHeaderV2`)    MTU     (PC + ESP32).
- Zero-Lock RCU- (`ROUT_MAGIC`)  Hot-Reload      .

**Autonomous Epoch Projection (AEP):**
- `BspBarrier`     I/O.
-      (Latency Hiding  Lock-Free ).
- Biological Amnesia (Spike Drop)    -   ,  VRAM.

### 4.2.     [PLANNED]

|  |  |  |
|---|---|---|
| **Dynamic AxonHandover** | PLANNED |     ;    |
| **Periodic Boundaries (Toroidal)** | PLANNED |  Z   X/Y    |
| **Slow Path Geometry Sync** | PLANNED |      Baking |
| **Pruning & Night Phase Networking** | PLANNED |  Ghost Axons      |
| **Latency Hiding Variance** | PLANNED |   1 ;     |
| **Atlas-Based Routing** | PLANNED |      `.ghosts`  |

---

## 5.   ( )

### 5.1. SpikeBatchHeader & SpikeEvent

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SpikeBatchHeader {
    pub batch_id: u32,      //   (  )
    pub spikes_count: u32,  //    
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SpikeEvent {
    pub receiver_ghost_id: u32,  // Ghost Axon ID    ( )
    pub tick_offset: u8,         //    (0..sync_batch_ticks)
    pub _pad: [u8; 3],           //   8 
}
```

**:** 8    (Coalesced Access  GPU).

**:**  (8 ) +  SpikeEvent (8   count).

### 5.2. BspBarrier Internals

```rust
pub struct BspBarrier {
    pub schedule_a: SpikeSchedule,              //  1
    pub schedule_b: SpikeSchedule,              //  2
    pub writing_to_b: bool,                     //   
    pub outgoing_batches: HashMap<u32, Vec<SpikeEvent>>,  //    
    pub socket: Option<NodeSocket>,             //  (None  MVP)
    pub peer_addresses: HashMap<u32, SocketAddr>,  // : shard_id  IP:port
}
```

**  sync_and_swap():**
1. Flip `writing_to_b` (GPU   ,    ).
2.  `socket.is_some()`:  `outgoing_batches`   (async).
3.      (await).
4. `ingest_spike_batch()`      Schedule.
5.  Schedule   .

### 5.3. GhostConnection File Format

 `{shard_name}.ghosts` (, Little-Endian):

```
[0..3]   magic: u32 = 0x47485354 ("GHST")
[4]      version: u8 = 1
[5]      padding: u8
[6..7]   width: u16 ( X/Y )
[8..9]   height: u16 ( Z    )
[10..11] padding: u16
[12..15] src_zone_hash: u32 ( Zona-)
[16..19] dst_zone_hash: u32 ( Zona-)
[20..]   connections: [GhostConnection; width  height]
         : local_axon_id (u32) + paired_src_soma (u32) = 8 
```

**:**  Connections   (Y, Z)  X-  (X, Z)  Y-.   GPU  `ApplySpikeBatch`    .

---

##  

|  |   |
|---|---|
| [04_connectivity.md](./04_connectivity.md) | 1.7: Ghost Axons, Cone Tracing, Axon Growth |
| [05_signal_physics.md](./05_signal_physics.md) | 1.2.1: ApplySpikeBatch kernel, Ghost indices |
| [07_gpu_runtime.md](./07_gpu_runtime.md) | 2: Batch Orchestration, sync_and_swap timing, Stream management |
| [09_baking_pipeline.md](./09_baking_pipeline.md) | 3: Ghost Connection generation, Z-sort for boundaries |
| [02_configuration.md](./02_configuration.md) | 5.3: master_seed, deterministic topology |
| [project_structure.md](../project_structure.md) | Distributed arch role in overall Axicor design |

## 6. Autonomous Node Recovery (The Great Resurrection)

 Axicor     .      **Zero-Downtime Shard Recovery**,     .

### 6.1. Shadow Buffers ( )

            POSIX Shared Memory.

1. **:**  500    `replicate_shards`.

2. **Zero-Copy Transfer:**  `dendrite_weights`  `dendrite_targets`    `/dev/shm/axicor_shard_{zone_hash}`  `/dev/shm/{zone_hash}.shadow`    .

3. **OS-Level Transport:**  `tokio::io::copy`,   Linux     `sendfile` ( `splice`),          ,  user-space .

```rust
// axicor-runtime/src/recovery.rs

pub async fn replicate_shards(
    shards: &[ShardState],
    backup_nodes: &[BackupNode],
) -> Result<Vec<ReplicaStatus>> {
    for shard in shards {
        let shm_path = format!("/dev/shm/axicor_shard_{:x}", shard.zone_hash);
        let shadow_path = format!("/dev/shm/{:x}.shadow", shard.zone_hash);
        
        // Open source (shared memory)
        let mut src = tokio::fs::File::open(&shm_path).await?;
        
        // Send to backup nodes asynchronously
        for backup in backup_nodes {
            let mut dst = TcpStream::connect(backup.addr).await?;
            tokio::io::copy(&mut src, &mut dst).await?;  // kernel-level sendfile
        }
    }
    Ok(replicas)
}
```

### 6.2. 500ms Isolation Detection

`BspBarrier`      .

-       `BSP_SYNC_TIMEOUT_MS` (500 ),   .
-    `BspError::NodeIsolated(dead_zone_hash)`.
- ,    (`.shadow`)  ,   .

```rust
// axicor-runtime/src/orchestrator.rs

pub fn detect_isolation(
    current_epoch: u32,
    barrier_start_epoch: u32,
    timeout_ms: u64,
) -> Option<IsolatedZone> {
    let elapsed = (current_epoch - barrier_start_epoch) * TICK_DURATION_US as u32 / 1000;
    
    if elapsed > timeout_ms as u32 {
        return Some(IsolatedZone {
            zone_hash: last_missing_peer,
            detection_epoch: current_epoch,
        });
    }
    None
}
```

### 6.3. The Great Resurrection ( )

     :

1. **VRAM Re-allocation:**     GPU  `padded_n`  `total_axons`.

2. **Shadow Restore:**   `/dev/shm/*.shadow`   VRAM  `cudaMemcpyAsync`.

3. **RCU Route Patching:**       `RouteUpdate` (. [06_distributed.md 3.5](./06_distributed.md#35-dynamic-routing-read-copy-update), `ROUT_MAGIC = 0x54554F52`).     Egress- (RCU Swap),     IP/Port. **    **.

```rust
// axicor-runtime/src/recovery.rs

pub struct ResurrectionCoordinator {
    shadow_path: String,
    zone_hash: u32,
}

impl ResurrectionCoordinator {
    pub async fn execute(&self) -> Result<()> {
        // Step 1: VRAM allocation
        let new_vram = allocate_vram(padded_n, total_axons)?;
        
        // Step 2: Shadow restore
        let shadow_data = read_shadow_buffer(&self.shadow_path)?;
        cuda_memcpy_async(new_vram, shadow_data)?;
        cuda_synchronize()?;
        
        // Step 3: RCU route patching
        self.broadcast_route_update(new_addr).await?;
        
        Ok(())
    }
}
```

### 6.4. Stabilization (Warmup Loop)

 `.shadow`       ,  **   ** ( ) -          PCIe .

     `voltage = 0`.      ,    -    , ,  .

**:**

 `ComputeCommand::Resurrect(zone_hash)`     **Warmup**  100  (Day Phase):

1. **  **   (`ApplySpikeBatch`).
2. **  ** (External Outputs  Virtual Axons) **** (   ).
3. ** ** (`voltage`  `threshold_offset`)        .
4. **  100 **     Normal      .

```rust
// axicor-runtime/src/compute.rs

pub struct ShardMode {
    pub variant: ShardVariant,
    pub warmup_ticks_remaining: u32,
}

pub enum ShardVariant {
    Normal,
    Warmup,  //   ,  
}

pub fn record_readout_warmup(
    out_spike_ids: &mut Vec<u32>,
    out_count: &mut u32,
    shard_mode: &ShardMode,
) {
    if shard_mode.variant == ShardVariant::Warmup {
        //    
        out_spike_ids.clear();
        *out_count = 0;
        return;
    }
    // Normal mode:   
    // ...  spike_ids  out_spike_ids
}
```

**:**  100  (110   )  ,     ,              .

---

## Changelog

|  |  |   |
|---|---|---|
| 2026-03-18 | 1.3 | **AEP Transition:**    BSP  WaitStrategy.  ElasticSchedule    . |
| 2026-03-02 | 1.2 |    Autonomous Node Recovery (Fault Tolerance) |
| 2026-02-28 | 1.1 |   [MVP] vs [PLANNED] .  GhostConnection. |
| TBD | 1.0 |    |

