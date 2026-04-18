# 11. Edge Bare Metal (Axicor-Lite)

>   Axicor.  HFT-   (Tier 2: ESP32-S3 / RISC-V).
> :    (Embodied AI)   < 2     520  SRAM.

---

## 1.    

    GPU   VRAM,  ESP32   .  HFT-    ,     :

1. **Dendrite Limit:** `MAX_DENDRITE_SLOTS`   128  **32**.      (Columnar Layout   4   ).
2. **Integer Physics:**  100% Branchless  .  FPU-   .
3. **Alignment:**      **32 ** (`alignas(32)`)     D-Cache (-  Xtensa/RISC-V).

---

## 2.   (Hybrid SRAM / Flash)

520  SRAM        .     ,   Memory-Mapped Flash (XIP - eXecute In Place).

### 2.1. Dual-Memory Artifacts (.sram  .flash)

ESP32-S3      (520 KB SRAM)     (16 MB SPI Flash).    `.state`  . SoA-     :

**1. `shard.flash` (Read-Only Topology)**
  Flash-    `spi_flash_mmap` (D-Bus).       .
* MMU:*    ,  64 KB (  MMU ESP32),    .
 ():
*   `soma_to_axon` [4   N]
*   `dendrite_targets` [4   32  N] (  128  32 )

**2. `shard.sram` (Hot State)**
  `DRAM` (  ).    .
 ():
*   `padded_n` [4 ] + `total_axons` [4 ] (, 8 )
*   `soma_voltage` [4   N]
*   `soma_flags` [1   N]
*   `threshold_offset` [4   N]
*   `refractory_timer` [1   N]
*   `dendrite_weights` [2   32  N]
*   `dendrite_timers` [1   32  N]
*   `axon_heads` [32   A] (BurstHeads8,   .axons)

### 2.2. Flash-Mapped DNA (Read-Only)
,     Day Phase,    Flash- (  QSPI)  `mmap` / `PROGMEM`:
- `dendrite_targets` (PackedTarget, X|Y|Z|Type)
- `soma_to_axon` ()
*    D-Cache   .*

### 2.3. Hot State SRAM (Read/Write)
  520  SRAM       :
- `voltage`, `flags`, `refractory_timer`, `threshold_offset` (Hot Soma)
- `axon_heads` (  `BurstHeads8` - 32 )
- `dendrite_weights` (    GSOP-)

---

## 3.    (FreeRTOS)

ESP32-S3   .  FreeRTOS     HFT-    .

### 3.1. Core 1 (App Core): Day Phase
 1      (Hot Loop). 
-  : `Propagate`, `UpdateNeurons` (GLIF), `ApplyGSOP`.
- ** .** 
- **Watchdog Protection:**   N  (,  10  )    `vTaskDelay(1 / portTICK_PERIOD_MS)`,   1     Task WDT.     .

### 3.2. Core 0 (Pro Core): I/O & Night Phase
 0   "" :
-  Wi-Fi / ESP-NOW.
-    I2C/SPI  DMA   float -> spikes (Sensory Encoding).
- **Night Phase:**  , Pruning  Sprouting.       Lock-Free ,   .

### 3.3.    

```text
[   ]                               [  ESP32-S3 ]
      |                                                |
                                                      
   (I2C)     --(float)-->  [ Core 0 ] Population Encoder (float -> 8-bit SpikeEvent)
(, )                     |
                                      
                             [ SRAM: Lock-Free Ring Buffer ] (alignas 32, std::atomic)
                                      |
                                      
                              [ Core 1 ] Hot Loop (Day Phase)
                              1. Inject:    (h0 = 0)
                              2. Propagate:   (hX += v_seg)
                              3. GLIF:  , ,  
                              4. GSOP:   
                              5. Readout: ++   
                                      |
                                      
                           [ SRAM: MotorOut Struct ] (std::atomic)
                                      |
                                      
        <--(PWM)---   [ Core 0 ] Population Decoder (spikes -> Duty Cycle)
  (, LED)                       |
      |                               |
      +-------------------------------+
          (  / )
```

---

## 4. Micro-Networking  

### 4.1. LwIP UDP Profile (Micro-Networking)

      **LwIP UDP Profile**.

** :**
- **MTU:**   **1400 **.     **173 **   UDP- (`(1400 - 16) / 8`).
- **Fragmentation:**    IP      SRAM;  L7-    (PC/Server).
- **Architecture:** Core 0  LwIP RX      8- `SpikeEvent`  `std::atomic` Lock-Free Ring Buffer. Core 1    GLIF/GSOP     I/O.
- **AEP Integration:**        (AEP), ESP32      "heartbeat"  (`is_last = 1`   )   PC-. MCU      ,    .        ,    .

### 4.2.  Hardware I/O
   `Input_Bitmask`  , Axicor-Lite         DMA-  (I2C , SPI , PWM ).
