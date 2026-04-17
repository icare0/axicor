# Hardware Backends

> Part of the Axicor architecture. Expanding the Axicor ecosystem beyond NVIDIA CUDA. Strategy for porting the compute core to alternative silicon: from AMD server GPUs to embedded systems (ESP32) and neuromorphic processors.

## 1. Concept: Compute Backend Abstraction

**[INVARIANT]** The simulation core (Integer Physics GLIF, GSOP plasticity) is deterministic and does not depend on a specific API (CUDA/HIP/OpenCL). The runtime is strictly separated into the Orchestrator (Rust) and the Compute Backend (Native C/C++ code).

### 1.1. Backend Trait (Rust Side)
To support different hardware types, a compute abstraction is introduced:

```rust
pub trait AxicorBackend {
    fn load_shard(&mut self, state: &ShardState) -> Result<()>;
    fn step(&mut self, inputs: &InputBatch) -> Result<()>;
    fn sync_night_phase(&mut self) -> Result<()>; // Download & maintenance
}
```

---

## 2. Tier 1: High-Performance Compute (NVIDIA CUDA)
Status: [MVP - Implemented]

Strict hardware invariants apply for compiling the CUDA backend (`axicor-compute/src/cuda/`):

- **GPU Architecture:** Pascal (GTX 10-series) and above. Mobile versions (e.g., GTX 1060 Mobile) are supported.
- **CUDA Toolkit Version:** Strictly >= 12.4. Older `nvcc` compilers possess heuristic analyzer bugs when unrolling loops (`#pragma unroll`) over 32-byte `alignas` structures and fail to correctly allocate registers for `__shfl_sync` and `__popc` in our HFT kernels.
- **Host OS:** Ubuntu 22.04 LTS / 24.04 LTS.
- **Virtualization:** Compilation on hypervisor hosts (e.g., Proxmox 9.1 on Debian 13) is NOT supported due to GCC system linker conflicts with proprietary headers. For virtual machines, exclusively use direct device passthrough (PCIe Passthrough) to a guest Ubuntu OS.

---

## 3. Tier 1: High-Performance Compute (AMD ROCm/HIP)
Status: [MVP - Implemented]

### 3.1. Architecture: Dual-Backend C-ABI
Instead of relying on fragile tooling like `hipify-perl` (automatic CUDA→HIP translation), a cleaner approach was implemented — the Dual-Backend C-ABI abstraction.
`axicor-compute` contains two mirrored directories with native implementations of the exact same interface:

```text
axicor-compute/src/
├── cuda/
│   ├── bindings.cu   ← NVIDIA CUDA (nvcc, Warp 32)
│   └── physics.cu
└── amd/
    ├── bindings.hip  ← AMD ROCm/HIP (hipcc, Wavefront 64)
    └── physics.hip
```

Backend selection occurs at compile time via Cargo feature flags. `build.rs` dynamically invokes `nvcc` or `hipcc`:

```bash
# NVIDIA (default)
cargo build -p axicor-node

# AMD
cargo build -p axicor-node --features amd
```

### 3.2. Why Porting Was Cheap
Due to the inherent Data-Oriented architecture with no computation graphs (zero PyTorch/cuDNN/cuBLAS usage), porting the mathematics was reduced to mechanical API replacements:

| NVIDIA CUDA | AMD HIP |
| :--- | :--- |
| `cudaMalloc` | `hipMalloc` |
| `cudaMemcpy` | `hipMemcpy` |
| `__syncthreads()` | `__syncthreads()` (identical) |
| `blockDim.x = 32` (Warp) | `blockDim.x = 64` (Wavefront) |

The only substantial adaptation was the local thread group size: AMD architectures (GCN/RDNA) natively use a 64-thread Wavefront instead of NVIDIA's 32-thread Warp. All algorithms (Integer Physics GLIF, GSOP, Columnar Dendrite Access) scale linearly — a larger wavefront simply increases VRAM utilization efficiency without code changes in the hot loop.

---

## 4. Tier 2: Edge Bare Metal (ESP32 & Embedded)
Goal: Autonomous embodied agents (robotics) on ultra-cheap hardware.

### 4.1. Bare Metal Runtime
For the implementation of the memory architecture, dual-core parallelization, and network stack, see `edge-bare-metal.md`.

- **ESP32-S3 (AI Instruction Set):** Utilization of Xtensa vector instructions to accelerate Integer Physics GLIF.
- **Constraints:** Reduced `MAX_DENDRITE_SLOTS` (32 instead of 128) to fit within SRAM limits via WTA Distillation.
- **Flash-Mapped DNA:** Utilizing `mmap` (or equivalent) to read axons directly from Flash memory without copying to RAM.
- **I/O:** Direct mapping of GPIO to InputMatrix (sensors) and OutputMatrix (servos).

---

## 5. Tier 3: Future Silicon (Neuromorphic & ASICs)
Goal: Energy efficiency on par with a biological brain (orders of magnitude higher than GPUs).

### 5.1. Neuromorphic Integration (Loihi, SpiNNaker)
[PLANNED] Investigate mapping GNM to asynchronous neuromorphic architectures.

- **Event-Driven Execution:** Abandoning the global tick (BSP Barrier) in favor of asynchronous spikes.
- **On-Chip Learning:** Adapting GSOP for hardware STDP implementations.

### 5.2. ASIC / FPGA (Verilog & VHDL)
[PLANNED] Design RTL description of the GLIF core.

- **Pipeline Logic:** Neuron as a Finite State Machine (FSM) on FPGA.
- **NoC (Network on Chip):** Hardware implementation of the Ring Buffer for forwarding Ghost Axons between chip cores.
- **HBM Integration:** High Bandwidth Memory utilization for storing synaptic weights.
