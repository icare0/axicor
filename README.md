<div align="center">
  <img src="./LOGO.svg" height="512" alt="Axicor Logo">
  <h1>Axicor Engine</h1>
  <p><strong>Neuromorphic Computing Engine - SNN AI architecture inspired by biology.</strong></p>

  <p>
    <a href="https://github.com/H4V1K-dev/Axicor/actions/workflows/ci.yml"><img src="https://github.com/H4V1K-dev/Axicor/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <a href="https://crates.io/crates/axicor-core"><img src="https://img.shields.io/crates/v/axicor-core.svg" alt="Crates.io"></a>
    <a href="https://docs.rs/axicor-core"><img src="https://img.shields.io/docsrs/axicor-core" alt="docs.rs"></a>
    <a href="https://pypi.org/project/axicor-client/"><img src="https://img.shields.io/pypi/v/axicor-client.svg" alt="PyPI"></a>
    <a href="#license"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg" alt="License"></a>
    <a href="https://github.com/H4V1K-dev/Axicor/commits"><img src="https://img.shields.io/github/last-commit/H4V1K-dev/Axicor" alt="Last Commit"></a>
  </p>

  <p>
    <a href="https://releases.rs/docs/1.75.0/"><img src="https://img.shields.io/badge/MSRV-1.75-informational" alt="MSRV"></a>
    <a href="https://crates.io/crates/axicor-core"><img src="https://img.shields.io/crates/d/axicor-core" alt="Crates Downloads"></a>
    <a href="https://pypi.org/project/axicor-client/"><img src="https://img.shields.io/pypi/dm/axicor-client" alt="PyPI Downloads"></a>
    <a href="https://deps.rs/repo/github/H4V1K-dev/Axicor"><img src="https://deps.rs/repo/github/H4V1K-dev/Axicor/status.svg" alt="dependency status"></a>
    <a href="https://axicor.dev"><img src="https://img.shields.io/badge/homepage-axicor.dev-blue" alt="Homepage"></a>
  </p>

  <p>
    <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-stable-orange?logo=rust" alt="Rust"></a>
    <a href="https://www.python.org"><img src="https://img.shields.io/badge/Python-3.10%2B-blue?logo=python" alt="Python"></a>
    <a href="https://developer.nvidia.com/cuda-toolkit"><img src="https://img.shields.io/badge/CUDA-12.0%2B-76B900?logo=nvidia" alt="CUDA"></a>
    <a href="https://rocm.docs.amd.com"><img src="https://img.shields.io/badge/ROCm-HIP-ED1C24?logo=amd" alt="ROCm"></a>
    <a href="https://t.me/+zptNJAJhDe41ZTEy"><img src="https://img.shields.io/badge/Telegram-Contributors-26A5E4?logo=telegram" alt="Telegram"></a>
    <a href="https://ko-fi.com/axicor"><img src="https://img.shields.io/badge/Support-Ko--fi-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi"></a>
  </p>
</div>

---

A living brain for embodied AI. Learns in seconds. Runs everywhere - from ESP32 microcontrollers to GPU clusters. Axicor is a Spiking Neural Network engine designed for biological realism and absolute determinism. No backpropagation. No static computation graphs. Neurons fire spikes, axons physically grow through 3D space, weak synapses are pruned, strong ones fortified - all in real-time, without halting the simulation.

## Why Axicor

**[Watch the live demo ->](https://www.tiktok.com/@alex_0xgenesis_agi/video/7617581624011541781)** - an ant agent in `Gymnasium-AntV4`, no pre-training, walking within 30 seconds of spawn.

- **Behavior out of the box.** An ant agent spawned in `Gymnasium-AntV4` stands upright within seconds and begins exploratory locomotion within 30 seconds of instantiation - with **zero pre-training**. No gradient descent, no replay buffer, no training epochs. Behavior emerges from biological priors encoded directly in the connectome topology.
- **Bit-exact determinism across hardware.** Integer-only physics (GLIF, GSOP) guarantees identical simulation results on an RTX 4090, AMD RDNA, or an ESP32. No floating-point drift, no cross-platform divergence.
- **Real-time structural plasticity.** Axons physically grow and prune during the Night Phase while the Day Phase hot-loop keeps ticking. No "training mode" vs "inference mode" divide - the brain rewires itself while it lives.
- **Modest hardware friendly.** Reference demos run on a GTX 1080 Ti. No cloud-scale compute required.

## Core Architecture

Axicor is built on three Data-Oriented Design pillars:

1. **Integer Physics.** All membrane integration (GLIF - Generalized Leaky Integrate-and-Fire) and plasticity (GSOP - Generalized Synaptic Offset Plasticity) math is performed using 100% branchless integer arithmetic. Zero floats. This guarantees bit-exact determinism across entirely different silicon.
2. **Day / Night Cycle.** The GPU/MCU hot loop (Day Phase) is completely isolated from structural plasticity. It only computes spike propagation. The CPU (Night Phase) handles routing, axon sprouting, and memory defragmentation.
3. **Headerless Structure of Arrays (SoA).** Complete rejection of OOP. Neurons do not exist as objects - data is laid out in flat, warp-aligned SoA memory dumps (`.state`, `.axons`) loaded directly into VRAM via zero-copy DMA (`mmap`). Cache-miss rate is practically zero.

## Ecosystem

- **[axicor-core](axicor-core/)** - FFI memory contracts, SoA layouts, `bytemuck`-verified C-ABI structures, IPC abstractions.
- **[axicor-compute](axicor-compute/)** - dual-backend compute kernels (NVIDIA CUDA / AMD HIP) with CPU fallback (`mock-gpu` feature).
- **[axicor-baker](axicor-baker/)** - offline topology compiler. Reads TOML "brain DNA", grows axons via cone tracing, emits binary `.state` / `.axons` dumps.
- **[axicor-node](axicor-node/)** - BSP-lockstep orchestrator, UDP fast-path server, Night Phase daemon.
- **[axicor-client](axicor-client/)** - Python SDK for RL integration. Zero-garbage UDP client, aligned `memoryview` encoders/decoders.
- **axicor-lab** - desktop IDE (Bevy + egui) with connectome viewer, blueprint editor, matrix inspector. Experimental.

## Quickstart

Axicor requires **Rust stable (1.75+)** and **Python 3.10+**. A CUDA-capable GPU or AMD ROCm device is recommended but not required - CPU fallback is shipped in the `mock-gpu` feature.

### Installation

```bash
git clone https://github.com/H4V1K-dev/Axicor.git
cd Axicor

# Bootstrap (Linux / macOS)
./scripts/setup.sh --mock

# Windows PowerShell
./scripts/setup.ps1 -Mock
```

Manual setup:
```bash
python -m venv .venv
source .venv/bin/activate             # Windows: .venv\Scripts\activate
pip install -e axicor-client/
cargo build --release
```

### 1. Bake the connectome (CPU)

Compile TOML "brain DNA" into binary C-ABI memory dumps:
```bash
cargo run --release -p axicor-baker -- \
  --brain Axicor-Models/AntConnectome --yes
```

### 2. Start the reactor (GPU or CPU)

Boot the engine. It maps baked binaries directly into memory and starts the BSP barrier:
```bash
# GPU (default, requires nvcc or hipcc at build time)
cargo run --release -p axicor-node -- Axicor-Models/AntConnectome.axic

# CPU-only fallback
cargo run --release -p axicor-node --features mock-gpu -- \
  Axicor-Models/AntConnectome.axic
```

### 3. Connect from Python

```python
from axicor.client import AxicorMultiClient
from axicor.utils import fnv1a_32

client = AxicorMultiClient(
    addr=("127.0.0.1", 8081),
    matrices=[{
        "zone_hash":    fnv1a_32(b"SensoryCortex"),
        "matrix_hash":  fnv1a_32(b"ant_sensors"),
        "payload_size": 1120,
    }],
    rx_layout=[{
        "matrix_hash": fnv1a_32(b"motor_out"),
        "size":        2560,
    }],
)

for tick in range(1000):
    client.payload_views[0].fill(0x55)    # write sensors
    rx = client.step(reward=1)            # advance one BSP epoch
```

## Engineering Invariants

Axicor makes a handful of deliberate design choices that differ from mainstream deep-learning frameworks. These are contracts, not preferences.

- **Little-Endian Network Law.** All network packets in the Axicor cluster transmit raw little-endian bytes, ignoring RFC Network Byte Order. This avoids `ntohl` / `htonl` CPU overhead on the hot path. All clients and nodes must adhere to this contract.
- **Dale's Law.** Synapse sign (excitatory / inhibitory) is immutable after connectome compilation. Plasticity can only modify magnitude.
- **BSP lockstep.** All shards advance in epoch-aligned barriers. No out-of-order simulation between zones.
- **Deterministic RNG.** All stochastic processes (spontaneous firing, axon growth noise) derive from `wyhash(master_seed, entity_id)`. Two runs with the same seed produce byte-identical results.

## Documentation

Full architectural specification, neuron models, and C-ABI contract reference:

- [Axicor Book](docs/src/) - architectural guide and design specs
- [C-ABI Contracts](docs/src/reference/c-abi-contracts.md) - `#[repr(C)]` layouts with compile-time static assertions
- [Baking Pipeline](docs/specs/09_baking_pipeline.md) - TOML DNA -> binary memory dumps
- [IO Matrix Protocol](docs/specs/08_io_matrix.md) - UDP fast-path encoding
- [Python SDK](docs/Python-SDK/) - encoder, decoder, and surgery APIs

## Community

- **Telegram:** [Contributors group](https://t.me/+zptNJAJhDe41ZTEy) with per-crate topic channels.
- **Issues:** [GitHub Issues](https://github.com/H4V1K-dev/Axicor/issues) for bug reports and feature requests.
- **Homepage:** [axicor.dev](https://axicor.dev)
- **Support development:** [Ko-fi](https://ko-fi.com/axicor)

## License

Licensed under either of:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) . [apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0))
- **MIT License** ([LICENSE-MIT](LICENSE-MIT) . [opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Axicor by you, as defined in the Apache-2.0 license, shall be dual-licensed as above, without any additional terms or conditions. See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution terms.
