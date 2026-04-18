#  Genesis HFT: Ant-v4 Example

 Embodied AI    Ant-v4,    Spiking Neural Networks (SNN)   3-  (DOD/WTA)      (R-STDP).

##    (Zero-Magic Pipeline)

** 0.   **
```bash
source .venv/bin/activate
```

** 1.     (WTA Architecture)**
  3-  (Sensory, Thoracic, Motor)  60%         Winner-Takes-All .
```bash
python3 examples/ant_exp/build_brain.py
```

** 2.  HFT-  GPU (Dual-Backend)**
    VRAM        .

#  CPU
```bash
cargo run --release -p genesis-node -- Axicor-Models/ant_exp.axic --cpu --log
```

#  NVIDIA (CUDA)
```bash
cargo run --release -p genesis-node -- Axicor-Models/ant_exp.axic --log
```

#  AMD (ROCm / HIP)
```bash
cargo run --release -p genesis-node --features amd -- Axicor-Models/ant_exp.axic --log
```

** 3.   (DOD Hot Loop)**
 Python-.       ,        `TARGET_TIME`  `TARGET_SCORE`.

```bash
python3 examples/ant_exp/ant_agent.py
```

      .
