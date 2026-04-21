## What
This PR fixes the CPU/mock-gpu bootstrap path so Axicor can be built, baked, and started on machines without CUDA or ROCm.

## Why
The repository advertises CPU/mock usage, but the actual startup path was broken:
- `mock-gpu` was not exposed by top-level crates
- Python builder always invoked the GPU baking path
- mock backend did not cover the FFI surface required by baking/runtime startup
- CPU runtime could panic on out-of-bounds batch slicing
- setup instructions referenced outdated example paths and commands

## Scope
- expose `mock-gpu` in `axicor-node` and `axicor-baker`
- complete mock FFI coverage required by baking/runtime startup
- harden CPU batch processing against invalid slice bounds
- auto-select backend in Python builder
- fix setup script commands for the current example layout

## Architectural Safety
This PR does not change:
- Integer Physics rules
- SoA / VRAM field order
- `ShardVramPtrs` memory contract
- the 1166-byte invariant
- baking binary formats

The changes are limited to bootstrap, backend selection, mock backend completeness, and runtime safety checks.

## Verification
```bash
cargo build --release -p axicor-node -p axicor-baker --features mock-gpu
source .venv/bin/activate
# Example assumes you have brought your own model (BYOM)
python your_model_dir/build_brain.py
./target/release/axicor-node Axicor-Models/your_model.axic --cpu --log
python your_model_dir/agent.py
```

## Notes
- This PR intentionally stays narrow: one PR, one task.
- Forex / new example work will be proposed separately to keep this reviewable and aligned with `CONTRIBUTING.md`.
