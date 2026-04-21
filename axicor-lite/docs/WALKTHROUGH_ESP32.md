<h1 align="center" style="color: red;">EXPERIMENTAL WORK DEMONSTRATION</h1>

# [BRAIN] Axicor Lite: Step-by-Step ESP32 Deployment Guide

This guide describes the process of deploying the Axicor Lite neuromorphic engine on a microcontroller (LilyGO T-Display or similar ESP32/S3 based boards). 

**Goal:** Run a real-time (HFT) simulation of your custom neural network on edge hardware using Data-Oriented Design and Zero-Copy mapping.

---

## Stage 1. Connection and Toolchain

1.  **Hardware:** Insert the USB cable into the controller and connect it to the PC.
2.  **Environment:** Activate the ESP-IDF cross-compiler. Without this, the terminal will not recognize the `idf.py` command.
    ```bash
    . $HOME/esp/esp-idf/export.sh
    ```
3.  **Port:** Ensure the board is visible in the system. Typically this is `/dev/ttyACM0` or `/dev/ttyUSB0`. 
    *If the port is different, replace it in all the commands below.*

---

## Stage 2. Generation and Brain Distillation (WTA-Architecture)

> [!NOTE]  
> **Bring Your Own Model:** The legacy CartPole example is no longer bundled with the Lite version. You will need to build your own model using the Axicor Baker before proceeding to distillation. 

We need to compress the neural network graph to a maximum of 32 dendrites per neuron so it fits within the strict SRAM limits of the microcontroller.

1.  **DNA Assembly (Baker):** Run your desktop graph compiler (e.g. `axicor-baker`) to generate the `.state` and `.axons` files for your model.
2.  **WTA-Distillation:** Run the memory linker. It will select the TOP-32 strongest synapses for each neuron, inject the C-ABI header, and align the binary to a 64 KB boundary for the hardware MMU:
    ```bash
    python3 axicor-lite/tools/distill_esp32.py \
      path/to/your/baked/model/shard.state \
      path/to/your/baked/model/shard.axons
    ```
    *Result:* The files `shard.sram` (dynamic weights) and `shard.flash` (topology) will appear in the project root.

---

## Stage 3. Kernel Compilation and Flashing (Dual-Core RTOS)

Now we upload the firmware itself and partition the Flash memory (creating a RAW `brain_topo` partition).

1.  **Navigate to the project folder:**
    ```bash
    cd axicor-lite
    ```
2.  **Reset config (Important):** If you changed the Watchdog settings or RTOS frequency, delete the old cache:
    ```bash
    rm -f sdkconfig
    rm -rf build/
    ```
3.  **Build and flash:** Set the target (esp32 or esp32s3) and upload the kernel:
    ```bash
    idf.py set-target esp32  # or esp32s3 depending on the board
    idf.py -p /dev/ttyACM0 build flash
    ```
    *Note:* After this step, the board will go into a Boot Loop (Panic) because the topology partition is still empty. This is normal behavior.

---

## Stage 4. Zero-Cost Mmap Injection

We embed the Read-Only topology directly into Flash memory. The code will read it via the hardware data bus without occupying SRAM.

1.  **Return to the root:**
    ```bash
    cd ..
    ```
2.  **Topology write:** Write `shard.flash` into the `brain_topo` partition:
    ```bash
    parttool.py --port /dev/ttyACM0 write_partition --partition-name brain_topo --input shard.flash
    ```

---

## Stage 5. Reactor Startup

Open the hardware monitor to observe neural activity:
```bash
cd axicor-lite
idf.py -p /dev/ttyACM0 monitor
```

### What you should see:
*   **Console:** MMU detects the "TOPO" header, auto-detects 832 neurons.
*   **Core 1:** Starts outputting logs: `[TICK] Tick X00 | Hot loop time: ~12000 us`. (If time < 15ms, the HFT budget is met).
*   **Display:** (Will be added later) A green Lock-Free TUI will render with metrics:
    *   **TPS:** Ticks per second.
    *   **D:** Current system dopamine level.
    *   **L/R:** Motor command telemetry.

---

## Technical Stack (DOD Principles)
*   **SRAM (~120 KB):** Dynamic weights, voltage, signal heads.
*   **Flash (Mmapped):** Static graph topology (Zero-Copy).
*   **RTOS Bypass:** Core 1 is dedicated to physics, Watchdog for IDLE1 is disabled.
*   **Branchless OR:** Hit-detection of 8 axon heads without conditions (SIMD-style).
*   **1000Hz Tick:** FreeRTOS switched to millisecond quantums for HFT precision.
