# axicor-core

The foundation crate for the Axicor engine, defining the binary contracts and shared logic for the neuromorphic reactor.

## Technical Focus

- **C-ABI Contracts:** Strict repr(C) structures for cross-platform and cross-language compatibility. All memory layouts are verified at compile-time with static assertions.
- **Structure of Arrays (SoA):** Highly optimized memory layout designed for GPU coalesced memory access and SIMD efficiency.
- **Integer Physics:** Implementation of GLIF (Generalized Leaky Integrate-and-Fire) and GSOP (Generalized Synaptic Offset Plasticity) using 100% branchless integer arithmetic.
- **Zero-Cost Casting:** Extensive use of the `bytemuck` crate for safe, zero-copy casting of raw binary blobs into structured data.

## Key Modules

- `ipc`: Shared memory and network packet definitions.
- `layout`: SoA definitions for shard states and axon buffers.
- `physics`: Core neuromorphic math.
- `vfs`: The Axicor Virtual File System for managing baked brain archives (.axic).

## License
GPL-3.0-or-later
