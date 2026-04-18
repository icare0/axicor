# Axicor Engine

[![CI](https://github.com/google/axicor/actions/workflows/ci.yml/badge.svg)](https://github.com/google/axicor/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/axicor-core.svg)](https://crates.io/crates/axicor-core)
[![Documentation](https://docs.rs/axicor-core/badge.svg)](https://docs.rs/axicor-core)
[![PyPI](https://img.shields.io/pypi/v/axicor-client.svg)](https://pypi.org/project/axicor-client/)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL%203.0-blue.svg)](https://opensource.org/licenses/GPL-3.0)

Axicor is a Neuromorphic Computing Engine. SNN AI Architecture based on biology.

A living brain for embodied AI. Learns in seconds. Runs everywhere - from ESP32 to GPU clusters. Axicor is a Spiking Neural Network engine designed for biological realism and absolute determinism. It does not use backpropagation or static computation graphs. Neurons fire spikes, axons physically grow through 3D space, weak synapses are pruned, and strong ones are fortified - all in real-time, without halting the simulation.

## Little-Endian Network Law

All network packets in the Axicor cluster ignore RFC Network Byte Order (Big-Endian). The cluster transmits raw Little-Endian bytes to avoid ntohl/htonl CPU overhead during HFT-scale spike propagation. All clients and nodes must adhere to this contract.

## Core Architecture

Axicor is built on three Data-Oriented Design (DOD) pillars:

1. **Integer Physics:** All membrane integration (GLIF) and plasticity (GSOP) math is performed using 100% branchless integer arithmetic. Zero floats. This guarantees bit-exact determinism across entirely different silicon (RTX 4090, AMD RDNA, or an ESP32 microcontroller).
2. **Day/Night Cycle:** The GPU/MCU Hot Loop (Day Phase) is completely isolated from structural plasticity. It only computes spike propagation. The CPU (Night Phase) handles the heavy lifting of routing, sprouting new axons, and memory defragmentation.
3. **Headerless Structure of Arrays (SoA):** Complete rejection of OOP. Neurons do not exist as objects. Data is laid out in flat, warp-aligned SoA memory dumps (.state, .axons) loaded directly into VRAM via Zero-Copy DMA (mmap). The cache miss rate is practically zero.

## Quickstart

### Step 0: Installation

Axicor requires Rust (stable) and Python 3.10+.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/google/axicor.git
   cd axicor
   ```

2. **Run the bootstrap script:**
   - On Linux/macOS: `./scripts/setup.sh`
   - On Windows (PowerShell): `./scripts/setup.ps1`

3. **Manual Setup (Alternative):**
   ```bash
   # Create and activate virtual environment
   python -m venv .venv
   source .venv/bin/activate  # Or .venv\Scripts\activate on Windows

   # Install Python SDK in editable mode
   pip install -e axicor-client/

   # Build the Rust workspace
   cargo build --release
   ```

### Step 1: Bake the Connectome (CPU)
Compile the TOML DNA into binary C-ABI memory dumps:
```bash
cargo run --release -p axicor-baker -- --brain Axicor-Models/AntAgent
```

### Step 2: Start the Reactor (GPU)
Boot the engine. It maps the binaries directly into memory and starts the BSP barrier:
```bash
cargo run --release -p axicor-node -- --brain AntAgent
```

### Step 3: Inject Dopamine (Python SDK)
Connect your Gymnasium environment via the Zero-Garbage UDP Fast-Path:
```python
from axicor.client import AxicorMultiClient

# The client runs in a strict 10ms lockstep budget
client = AxicorMultiClient(addr=("127.0.0.1", 8081), ...)
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
