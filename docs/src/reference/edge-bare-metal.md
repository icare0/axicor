# Edge Bare Metal

> Part of the Axicor architecture. Specification of the HFT engine for microcontrollers (Tier 2: ESP32-S3 / RISC-V).
> Goal: Autonomous Embodied AI with < 2 ms latency and a memory budget of 520 KB SRAM.

## 1. Fundamental Constraints and Trimming

Unlike server GPUs with gigabytes of VRAM, every byte is critical on the ESP32. To prevent the HFT loop from falling out of caches, we introduce strict architectural limits:

1. **Dendrite Limit:** `MAX_DENDRITE_SLOTS` is reduced from 128 to **32**. This radically reduces memory consumption (the Columnar Layout takes 4x less space).
2. **Integer Physics:** Strictly 100% Branchless integer math. No FPU instructions in the hot loop.
3. **Alignment:** All structures are aligned to a **32-byte** boundary (`alignas(32)`) for perfect D-Cache hits (cache lines of Xtensa/RISC-V processors).

---

## 2. Memory Architecture (Hybrid SRAM / Flash)

520 KB of SRAM cannot physically fit a connectome of several thousand neurons. We use a hybrid memory architecture relying on Memory-Mapped Flash (XIP - eXecute In Place).

### 2.1. Dual-Memory Artifacts (`.sram` and `.flash`)

The ESP32-S3 has a hard limit of fast memory (520 KB SRAM) and a large volume of slow memory (16 MB SPI Flash). The desktop monolithic `.state` file is hardware-incompatible. SoA arrays are split into two binary artifacts:

**1. `shard.flash` (Read-Only Topology)**
Flashed and mapped via `spi_flash_mmap` (D-Bus). Data never mutates in the hot loop.
*[C-ABI] MMU Law:* The file MUST have a size strictly multiple of 65536 bytes (64 KB ESP32 MMU page size), otherwise the mapping will fail.
Composition (Columnar):
*   `soma_to_axon` [4 bytes × N]
*   `dendrite_targets` [4 bytes × 32 × N] (reduced from 128 to 32 slots)

**2. `shard.sram` (Hot State)**
Loaded into DRAM (internal chip memory). Contains only mutating arrays.
Composition (Columnar):
*   `padded_n` [4 bytes] + `total_axons` [4 bytes] (Header, 8 bytes)
*   `soma_voltage` [4 bytes × N]
*   `soma_flags` [1 byte × N]
*   `threshold_offset` [4 bytes × N]
*   `refractory_timer` [1 byte × N]
*   `dendrite_weights` [2 bytes × 32 × N]
*   `dendrite_timers` [1 byte × 32 × N]
*   `axon_heads` [32 bytes × A] (BurstHeads8, extracted from `.axons`)

### 2.2. Flash-Mapped DNA (Read-Only)
Topology that does not change in the Day Phase is mapped directly from Flash memory (via QSPI interface) using `mmap` / `PROGMEM`:
- `dendrite_targets` (PackedTarget, X|Y|Z|Type)
- `soma_to_axon` (routing)
*These data are hardware-cached by the microcontroller's D-Cache upon reading.*

### 2.3. Hot State SRAM (Read/Write)
Fast 520 KB SRAM stores only real-time mutating arrays:
- `voltage`, `flags`, `refractory_timer`, `threshold_offset` (Hot Soma)
- `axon_heads` (Shift register `BurstHeads8` - 32 bytes)
- `dendrite_weights` (Synapse weights for GSOP plasticity)

---

## 3. Asymmetric Dual-Core Cycle (FreeRTOS)

The ESP32-S3 has two cores. FreeRTOS allows us to rigidly isolate HFT physics from slow network interrupts.

### 3.1. Core 1 (App Core): Day Phase
Core 1 is strictly reserved for the Hot Loop.
- Executes only: `Propagate`, `UpdateNeurons` (GLIF), `ApplyGSOP`.
- **No mutexes.**
- **Watchdog Protection:** Every N ticks (e.g., every 10 ms of simulation), the loop MUST call `vTaskDelay(1 / portTICK_PERIOD_MS)`, yielding 1 tick to the scheduler to reset the hardware Task WDT. Otherwise, the controller will reboot.

### 3.2. Core 0 (Pro Core): I/O & Night Phase
Core 0 performs all "dirty" work:
- Wi-Fi / ESP-NOW maintenance.
- Reading sensors via I2C/SPI using DMA and converting `float` -> `spikes` (Sensory Encoding).
- **Night Phase:** Synapse sorting, Pruning, and Sprouting. Inter-core data transfer uses Lock-Free queues, not mutexes.

### 3.3. Visual Data Flow

```text
[ Physical World ]                               [ ESP32-S3 Chip ]
      │                                                │
      ▼                                                ▼
  Sensors (I2C)     ──(float)──>  [ Core 0 ] Population Encoder (float -> 8-bit SpikeEvent)
(Gyroscope, Lidar)                    │
                                      ▼
                             [ SRAM: Lock-Free Ring Buffer ] (alignas 32, std::atomic)
                                      │
                                      ▼
                              [ Core 1 ] Hot Loop (Day Phase)
                              1. Inject: Reset axon heads (h0 = 0)
                              2. Propagate: Vector shift (hX += v_seg)
                              3. GLIF: Current integration, thresholds, soma spikes
                              4. GSOP: Dendrite weight mutation
                              5. Readout: ++ to motor neuron counters
                                      │
                                      ▼
                           [ SRAM: MotorOut Struct ] (std::atomic)
                                      │
                                      ▼
   Servo drives      <──(Duty Cycle)──┘
  (Motors, LEDs)                      
      │                               
      └───────────────────────────────┘
          (Environment / Angle Change)
```

---

## 4. Micro-Networking and Sensors

### 4.1. LwIP UDP Profile (Micro-Networking)
The standard LwIP UDP Profile is used for cluster communication.

**Profile Characteristics:**

- **MTU:** Hard limit of 1400 bytes. This allows transmitting up to 173 spikes in one UDP packet `((1400 - 16) / 8)`.
- **Fragmentation:** IP-level fragmentation is disabled to minimize SRAM load; L7-fragmentation on the sender side (PC/Server) is used.
- **Architecture:** Core 0 handles LwIP RX interrupts and asynchronously pushes incoming 8-byte `SpikeEvent` objects into a `std::atomic` Lock-Free Ring Buffer. Core 1 executes the hot loop GLIF/GSOP without pausing for network I/O.
- **AEP Integration:** Due to the cluster transitioning to Asynchronous Epoch Projection (AEP), ESP32 is no longer required to send empty "heartbeat" packets (`is_last = 1` for an empty batch) to unblock PC nodes. The MCU sends spike packets only when they physically occur. This critically saves CPU cycles and battery life by eliminating idle network traffic.

### 4.2. Direct Hardware I/O
Instead of virtual `Input_Bitmask` matrices over the network, Genesis-Lite can bind sensory axons directly to interrupts or DMA buffers of sensors (I2C gyroscopes, SPI cameras, PWM servos).
