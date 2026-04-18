# 10. Hardware Backends (Roadmap & Integration)

>   [Axicor](../../README.md)   NVIDIA CUDA.       :   GPU AMD    (ESP32)   .

---

## 1. : Compute Backend Abstraction

**:**   ( GLIF,  GSOP)       API (CUDA/HIP/OpenCL).    `Orchestrator` (Rust)  `Compute Backend` (Native code).

### 1.1. Backend Trait (Rust Side)
       :

```rust
pub trait AxicorBackend {
    fn load_shard(&mut self, state: &ShardState) -> Result<()>;
    fn step(&mut self, inputs: &InputBatch) -> Result<OutputBatch>;
    fn sync_night_phase(&mut self) -> Result<ShardState>; // Download & maintenance
}
```

---

## 2. Tier 1: High-Performance Compute (NVIDIA CUDA)

**: [MVP - ]**

  CUDA- (`axicor-compute/src/cuda/`)    :

*   ** GPU:** Pascal (GTX 10-)  .    (, GTX 1060 Mobile).
*   ** CUDA Toolkit:**  `>= 12.4`.   `nvcc`        (`#pragma unroll`)  32- `alignas`         `__shfl_sync`  `__popc`   HFT-.
*   ** :** Ubuntu 22.04 LTS / 24.04 LTS. 
*   **:**     (, Proxmox 9.1  Debian 13)   -    GCC   .         (PCIe Passthrough)   Ubuntu.

---

## 3. Tier 1: High-Performance Compute (AMD ROCm/HIP)

**: [MVP - ]**

### 3.1. : Dual-Backend C-ABI

    `hipify-perl` (  CUDAHIP)      - **Dual-Backend C-ABI **.

`axicor-compute`            :

```
axicor-compute/src/
+-- cuda/
|   +-- bindings.cu    NVIDIA CUDA (nvcc, Warp 32)
|   +-- physics.cu
+-- amd/
    +-- bindings.hip   AMD ROCm/HIP (hipcc, Wavefront 64)
    +-- physics.hip
```

**  -   **  feature- Cargo. `build.rs`  `nvcc`  `hipcc`:

```sh
# NVIDIA ( )
cargo build -p axicor-node

# AMD
cargo build -p axicor-node --features amd
```

### 3.2.    

  **Data-Oriented    ** ( PyTorch/cuDNN/cuBLAS)       API:

| NVIDIA CUDA | AMD HIP |
|---|---|
| `cudaMalloc` | `hipMalloc` |
| `cudaMemcpy` | `hipMemcpy` |
| `__syncthreads()` | `__syncthreads()` () |
| `blockDim.x = 32` (Warp) | `blockDim.x = 64` (Wavefront) |

   -    : AMD- (GCN/RDNA)  **64- Wavefront**  NVIDIA- 32- Warp.   (Integer GLIF, GSOP, Columnar Dendrite Access)   -        .

---

## 4. Tier 2: Edge Bare Metal (ESP32 & Embedded)

**:**    ()   .

### 4.1. Bare Metal Runtime
  ,      .  [11_edge_bare_metal.md](11_edge_bare_metal.md).
// - **ESP32-S3 (AI Instruction Set):**    Xtensa     GLIF.
// - **:**   `dendrite_slots` (32  128)    SRAM.
// - **Flash-Mapped DNA:**  `mmap` ( )      Flash-    RAM.
// - **I/O:**   GPIO  `InputMatrix` ()  `OutputMatrix` ().

---

## 5. Tier 3: Future Silicon (Neuromorphic & ASICs)

**:**     (   GPU).

### 5.1. Neuromorphic Integration (Loihi, SpiNNaker)
// TODO:   GNM    .
// - **Event-Driven Execution:**     (BSP)    .
// - **On-Chip Learning:**  GSOP    STDP.

### 5.2. ASIC / FPGA (Verilog & VHDL)
// TODO:  RTL-  GLIF.
// - **Pipeline Logic:**     (FSM)  FPGA.
// - **NoC (Network on Chip):**   Ring Buffer   Ghost Axons   .
// - **HBM Integration:**          .

---

## Connected Documents

| Document | Connection |
|---|---|
| [07_gpu_runtime.md](./07_gpu_runtime.md) |     CUDA |
| [01_foundations.md](./01_foundations.md) |   ,     |
| [06_distributed.md](./06_distributed.md) |      (GPU + ESP32) |
