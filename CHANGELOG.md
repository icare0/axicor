# Changelog

All notable public-facing changes to Axicor are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) · Versioning: [SemVer](https://semver.org/).

For per-commit internal development history, see [HISTORY.md](HISTORY.md).

---

## [Unreleased]

## [0.1.0] - 2026-04-18

First public release of the Axicor Neuromorphic Computing Engine.

### Added

- **axicor-core** — C-ABI memory contracts, SoA layouts, `bytemuck`-verified `#[repr(C)]` structures, IPC abstractions.
- **axicor-compute** — dual-backend compute kernels (NVIDIA CUDA + AMD HIP) with CPU fallback via `mock-gpu` feature. Full Integer Physics (GLIF, GSOP) in branchless integer arithmetic.
- **axicor-baker** — offline topology compiler: TOML brain DNA → binary `.state` / `.axons` memory dumps via cone-traced axon growth.
- **axicor-node** — BSP-lockstep orchestrator, UDP fast-path server, Night Phase daemon with structural plasticity.
- **axicor-client** (PyPI) — zero-garbage Python SDK for RL integration with aligned `memoryview` encoders / decoders.
- `AntConnectome` reference brain for `Gymnasium-AntV4` locomotion demonstrating zero-pre-training exploratory behavior.
- Deterministic `wyhash` RNG with `master_seed` — byte-identical simulation across RTX 4090, AMD RDNA, and ESP32.
- BSP lockstep protocol with epoch barriers, Little-Endian Network Law, Burst-8 axon layout.
- MSRV: Rust 1.75+.

### License

Dual-licensed under **MIT OR Apache-2.0** (Rust ecosystem standard).
