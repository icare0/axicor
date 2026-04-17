# Axicor Engine

[![Crates.io](https://img.shields.io/crates/v/axicor-core.svg)](https://crates.io/crates/axicor-core)
[![Documentation](https://docs.rs/axicor-core/badge.svg)](https://docs.rs/axicor-core)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%203.0-blue.svg)](https://opensource.org/licenses/GPL-3.0)

**A living brain for embodied AI. Learns in seconds. Runs everywhere — from ESP32 to GPU clusters.**

Axicor (formerly Genesis) is a High-Frequency Trading (HFT) Spiking Neural Network engine designed for biological realism and absolute determinism. It does not use backpropagation or static computation graphs. Neurons fire spikes, axons physically grow through 3D space, weak synapses are pruned, and strong ones are fortified — all in real-time, without halting the simulation.

## Core Architecture

Axicor is built on three Data-Oriented Design (DOD) pillars:

1. **Integer Physics:** All membrane integration (GLIF) and plasticity (GSOP) math is performed using 100% branchless integer arithmetic. Zero floats. This guarantees bit-exact determinism across entirely different silicon (RTX 4090, AMD RDNA, or an ESP32 microcontroller).
2. **Day/Night Cycle:** The GPU/MCU Hot Loop (*Day Phase*) is completely isolated from structural plasticity. It only computes spike propagation. The CPU (*Night Phase*) handles the heavy lifting of routing, sprouting new axons, and memory defragmentation.
3. **Headerless Structure of Arrays (SoA):** Complete rejection of OOP. Neurons do not exist as objects. Data is laid out in flat, warp-aligned SoA memory dumps (`.state`, `.axons`) loaded directly into VRAM via Zero-Copy DMA (`mmap`). The cache miss rate is practically zero.

## Quickstart

**1. Bake the Connectome (CPU)**
Compile the TOML DNA into binary C-ABI memory dumps:
```bash
cargo run --release -p axicor-baker -- --brain Genesis-Models/AntAgent
```

**2. Start the Reactor (GPU)**
Boot the engine. It maps the binaries directly into memory and starts the BSP barrier:
```bash
cargo run --release -p axicor-node -- --brain AntAgent
```

**3. Inject Dopamine (Python SDK)**
Connect your Gymnasium environment via the Zero-Garbage UDP Fast-Path:
```python
from genesis.client import GenesisMultiClient

# The client runs in a strict 10ms lockstep budget
client = GenesisMultiClient(addr=("127.0.0.1", 8081), ...)
```

## Ecosystem

- **axicor-core**: FFI memory contracts, memory alignment, and IPC abstractions.
- **axicor-baker**: The offline topology compiler (Cone Tracing, UV Projection).
- **axicor-compute**: Dual-backend compute kernels (NVIDIA CUDA & AMD HIP).
- **axicor-node**: The BSP orchestrator, UDP Fast-Path server, and Night Phase daemon.
- **axicor-client**: The Python SDK for RL integration (Zero-Garbage Encoders).

## Documentation
Read the full architectural specification, neural models, and C-ABI contracts in the Axicor Book or in the `docs/src` directory.

## License
GPL-3.0-or-later. Open for research, robotics, and embodied AI.