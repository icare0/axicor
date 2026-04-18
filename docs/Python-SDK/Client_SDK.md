# Axicor Client SDK: Architecture & Internal Contracts

  Data-Oriented  Axicor.       (Python, RL-, )  HFT-  (Rust + CUDA/HIP).

## 1.    ( )

SDK (`axicor-client`) -    ML-   .    ,   -    VRAM    ,  GIL      .

> [!IMPORTANT]
> ** :**     `class Neuron`, `class SynapseGroup`,    JSON/Protobuf    REST/gRPC API -  .

      `numpy.ndarray`,  `memoryview`  `struct.pack`. Python     .

### 1.1.   (The 10ms Rule)

     100  (1 ).    (`sync_batch_ticks`)  100 .  ,    Python-   **10 **   :

1.     (UDP rx).
2.    (Gymnasium / Mujoco).
3.        (NumPy `packbits`).
4.     (UDP tx).

  -    JSON       (GC),     15-20      .      (Pre-allocation).

### 1.2.   (PyTorch vs Axicor)

        `loss.backward()`.       .

|  ML / RL | Axicor DOD  |      |
| :--- | :--- | :--- |
| `env.step(action)` | **UDP Fast-Path** |  float    (Population Coding)    UDP-   .  HTTP. |
| `loss.backward()` | **R-STDP (Dopamine)** |      20- C-ABI     ( `global_reward`).   . |
| `model.parameters()` | **Zero-Copy Introspection** |    `/dev/shm/axicor_shard_*`.     O(1)    ,   Rust. |
| TensorBoard /  | **WS Telemetry Stream** |  push-  ID   WebSocket   3D-  matplotlib  JSON-. |

 - GIL (Global Interpreter Lock)    -     Python    SDK.     Data-Oriented Design (DOD) Python     ,    ,        C-      NumPy.  Axicor SDK.

** 10- :**  Axicor     ()  100 .      (`sync_batch_ticks`)  100 .  ,    Python-   **10 **, :

1.    .
2.     (,   CartPole).
3.       .
4.    .

**   ( ):**

*   ** 100 000   JSON**  ~1520 .     ,     TPS  .
*   ** -** (, `class Spike`)      (GC),      .

**DOD- Axicor SDK:**

*      `np.packbits`    `np.frombuffer`.
*   Zero-Copy `struct.pack_into`      (Pre-allocation).
*   **:**    ~0.05 ,  9.95     RL-.

### 1.3.  :  Lockstep (Strict BSP)

     (  `model.forward()`   ),  Axicor     .        **BSP (Bulk Synchronous Parallel)**,    **Lockstep**.

   -,  ,        ,      .

**  :**
1.  **Tx (Send):**     (  + )     UDP-.
2.  **Barrier (Wait):**   `sock.recvfrom_into()`   ,    .
3.  **Autonomous GPU Compute:**  Axicor  ,    VRAM (Zero-Copy DMA) and     GPU (Day Phase)     .
4.  **Rx (Receive):**   ,  `RecordReadout`     (`Output_History`)   UDP.      .

> [!CAUTION]
> **  (Biological Amnesia):**
>   Python- "" (, GC Gen2   `env.step()`)       ,    Biological Amnesia -    .       .    Axicor.

---

## 2. Data Plane: UDP Fast-Path & C-ABI

    UDP.    .     20-  Little-Endian.  HTTP, gRPC   .

### 2.1.  `ExternalIoHeader` (20 )

   C++      struct.     .

```c
struct alignas(4) ExternalIoHeader {
    uint32_t magic;         // 0x4F495347 ("GSIO")  , 0x4F4F5347 ("GSOO")  
    uint32_t zone_hash;     // FNV-1a    (, "SensoryCortex")
    uint32_t matrix_hash;   // FNV-1a   I/O  (, "cartpole_sensors")
    uint32_t payload_size;  //      (   )
    int16_t  global_reward; //    R-STDP (-32768..32767)
    uint16_t _padding;      //    20 
};
```

**  Python SDK (struct format):** `<IIIIhH`

### 2.2.  (Cold Start Auto-Wiring)

      .    **Auto-Wiring**: SDK    `io.toml`       C-ABI   L7-    .

```python
from axicor.contract import AxicorIoContract
from axicor.client import AxicorMultiClient

# 1.  I/O  (   Hot Loop)
zone_dir = "Axicor-Models/MyAgent/baked/SensoryCortex"
contract = AxicorIoContract(zone_dir, "SensoryCortex")
client_cfg = contract.get_client_config(BATCH_SIZE)

# 2.  HFT 
# SDK    bytearray     
client = AxicorMultiClient(
    addr=("127.0.0.1", 8081),
    **client_cfg
)
```

### 2.3. Zero-Copy L7 Assembler ( )

     MTU (65507 ),   Rust     L7-    64  (L2 Cache Line). 
 `AxicorMultiClient`         `step()`.

** :**
*   **  :**      `SO_RCVBUF`  **8 **.  ,   Linux   " " L7-    ,  Python   .
*       `_rx_arena` .
*        `recvfrom_into`  `memoryview`.
*             O(1)   .

> [!WARNING]
> **Receive Buffer Overflow:**     `[AxicorClient] UDP Timeout`, ,   `net.core.rmem_max`        8 .

---

## 3. Memory Plane: Zero-Copy mmap (Telemetry & Surgery)

 `AxicorMemory`  `AxicorSurgeon`   VRAM- ,   . 

### 3.1. SDK Telemetry Translation (Mass -> Charge)

    **Mass Domain** (32-    2.1 ).     `AxicorMemory`      .

*    `get_network_stats()`  `avg_weight`  `max_weight`,    **65536.0**.
*    ,  Python   **Charge Domain** (   ),    ,   "" .

####  `ShmHeader` (64 bytes)
//  128  (L2 Cache Line x2)
struct alignas(128) ShmHeader {
    uint32_t magic;             // 0x00: 0x47454E53 ("GENS")
    uint8_t  version;           // 0x04: 3
    uint8_t  state;             // 0x05: ShmState enum
    uint16_t _pad;              // 0x06: Padding
    uint32_t padded_n;          // 0x08:  ( 32  Warp Alignment!)
    uint32_t dendrite_slots;    // 0x0C:  128
    uint32_t weights_offset;    // 0x10:   i32[128 * padded_n]
    uint32_t targets_offset;    // 0x14:   u32[128 * padded_n]
    uint64_t epoch;             // 0x18:  
    uint32_t total_axons;       // 0x20: Local + Ghost + Virtual
    uint32_t handovers_offset;  // 0x24:   
    uint32_t handovers_count;   // 0x28:   
    uint32_t zone_hash;         // 0x2C: FNV-1a 
    uint32_t prunes_offset;     // 0x30:   
    uint32_t prunes_count;      // 0x34:  
    uint32_t incoming_prunes;   // 0x38:  
    uint32_t flags_offset;      // 0x3C:   u8[padded_n]

    // --- Extended Header (v3) ---
    uint32_t voltage_offset;          // 0x40:   i32[padded_n]
    uint32_t threshold_offset_offset; // 0x44:   i32[padded_n]
    uint32_t timers_offset;           // 0x48:   u8[padded_n]
    uint32_t _reserved[1];           // 0x4C..0x80:   128 
};
```

> [!CAUTION]
> ** :**       `sizeof`!   - L2-.   `weights_offset`  `targets_offset`   64- .

### 3.2.  : Zero-Index Trap

  Axicor  `target_packed == 0`     **Early Exit**  GPU  MCU.    0,         ,     127 .     (Branchless Early Exit).

-   `axon_id`    `+1`. 

**=== PACKED TARGET C-ABI (32-bit) ===**
```text
[31 ....................... 24] [23 .......................... 0]
      Segment Offset (8-bit)           Axon ID + 1 (24-bit)
```

       `mmap` (,    `distill_esp32.py`   `AxicorSurgeon`),           `0x00FFFFFF` (24 ).

**Zero-Cost  (Python):**
```python
#   32-  
axon_id = (target_packed & 0x00FFFFFF) - 1
segment_offset = target_packed >> 24
```

**Zero-Cost  (Python):**
```python
#    VRAM
#    axon_id   +1!
#  0    Early Exit.
target_packed = (segment_offset << 24) | ((axon_id + 1) & 0x00FFFFFF)
```

     `axon_id`   (`+1`)        -1 (`0xFFFFFFFF`).  GPU  **CUDA Illegal Memory Access**,  ESP32     - **LoadStoreError**.

---

## 4. - (Multiple Instances)

          (),     `Base_Port + N * 10`,  N -   .

|  |  |   (N=0) |   |  |
| :--- | :--- | :--- | :--- | :--- |
| External In | UDP | 8081 | 8081 + N * 10 |   (  SDK) |
| External Out | UDP | 8082 | 8082 + N * 10 |   ( SDK) |
| Geometry | TCP | 9002 | 9002 + N * 10 |  3D- (IDE/Viz) |
| Telemetry | WS | 9003 | 9003 + N * 10 |   (IDE/Dashboards) |

> [!IMPORTANT]
> **  (Thread Isolation):**   (WebSocket  9003)   (TCP  9002)      asyncio .  RL-  (UDP Fast-Path)     ,     I/O.
