# axicor-compute

The hardware acceleration layer for the Axicor engine, providing highly optimized kernels for neuromorphic simulation.

## Technical Focus

- **Dual-Backend Support:** Native support for both NVIDIA CUDA and AMD HIP (ROCm). The abstraction layer allows the same Rust code to target both ecosystems without performance loss.
- **Warp/Wavefront Alignment:** Kernels are designed around hardware-specific execution widths (32 for NVIDIA, 64 for AMD) to maximize throughput and minimize Divergence.
- **Branchless GLIF Kernels:** All neuron membrane integration and spike detection logic is implemented without conditional branches, ensuring consistent performance regardless of brain activity level.
- **Pinned Memory Management:** Advanced DMA management using Pinned (Page-Locked) memory for ultra-fast transfers between CPU and GPU.

## Features

- `cuda`: (Default) Enable the NVIDIA CUDA backend.
- `amd`: Enable the AMD HIP (ROCm) backend.
- `mock-gpu`: A CPU-based mock implementation for CI/CD and development without physical GPUs.

## License
Dual-licensed under MIT or Apache 2.0.
