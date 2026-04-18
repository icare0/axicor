<h1 style="color: red;">   </h1>

#  Genesis HFT: FLY_exp Example

##    (Zero-Magic Pipeline)

** 0.   **
```bash
source .venv/bin/activate
```

** 1.     (One-Click Build)**
  TOML-  Python SDK    Rust- (`genesis-baker`),    VRAM-.
```bash
python3 examples/FLY_exp/build_brain.py
```

** 2.  HFT-  GPU (Dual-Backend)**
   VRAM- (Zero-Copy)        .

#  CPU
```bash
cargo run --release -p genesis-node -- Axicor-Models/FLY_exp.axic --cpu --log
```

#  NVIDIA (CUDA)
```bash
cargo run --release -p genesis-node -- Axicor-Models/FLY_exp.axic --cpu-profile balanced --log
```

#  AMD (ROCm / HIP)
```bash
cargo run --release -p genesis-node --features amd -- Axicor-Models/FLY_exp.axic --cpu-profile balanced --log
```

** 3.   (RL Agent)**
    Python-.               (-255)        .

```bash
python3 examples/FLY_exp/agent.py
```

   .  ,         .
