#!/usr/bin/env python3
import os
import sys
import subprocess
from pathlib import Path

# Подключаем SDK
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "axicor-client")))
from axicor.builder import BrainBuilder

def build_validation_brain():
    print("[*] Initializing Validation Connectome...")
    base_path = Path(__file__).parent.parent.parent
    gnm_path = base_path / "Axicor_Neuron-Lib"
    out_dir = base_path / "Axicor-Models" / "ValidationConnectome"

    builder = BrainBuilder(project_name="ValidationConnectome", output_dir=str(out_dir), gnm_lib_path=str(gnm_path))
    builder.sim_params["sync_batch_ticks"] = 1000 # Широкое окно для снятия трасс

    # Canonical IDs
    canons = {
        "Integrator": "313860745",
        "Fast-Spiking": "475549334",
        "Relay": "313861411",
        "Pacemaker": "313862134",
        "Martinotti": "313861608"
    }

    # Создаем микро-зону 8x8x1 = 64 вокселя.
    # Чтобы получить ровно 5 нейронов (по 1 на тип), ставим плотность 5/64
    zone = builder.add_zone("SensoryCortex", width_vox=8, depth_vox=8, height_vox=1)
    layer = zone.add_layer("L1", height_pct=1.0, density=5/64)
    
    for name, cid in canons.items():
        cell = builder.gnm_lib(cid)
        layer.add_population(cell, fraction=0.2)

    builder.build()

    print("[*] Compiling ValidationConnectome via axicor-baker...")
    brain_toml_path = out_dir / "brain.toml"
    
    cargo_cmd = ["cargo", "run", "--release", "-p", "axicor-baker", "--bin", "axicor-baker", "--"]
    # Оставляем авто-детект бэкенда на стороне cargo, просто вызываем компилятор
    cargo_cmd.extend(["--brain", str(brain_toml_path), "--clean", "--yes"])
    
    subprocess.run(cargo_cmd, cwd=str(base_path), check=True)

if __name__ == "__main__":
    build_validation_brain()
