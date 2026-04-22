#!/usr/bin/env python3
import toml
from pathlib import Path
from typing import Dict, Any

# [DOD] C-ABI Freeze (64 bytes invariant). 
# Эта схема жестко привязана к axicor-core/src/layout.rs
def generate_scaffold(name: str, is_inhibitory: bool) -> Dict[str, Any]:
    return {
        "neuron_type": [{
            "name": name,
            
            # --- Membrane (32-bit) ---
            "threshold": 0,               # Will be calibrated (uV)
            "rest_potential": 0,          # Will be calibrated (uV)
            "leak_shift": 4,              # Exponential shift. Calibrated from tau_m
            "homeostasis_penalty": 0,
            "spontaneous_firing_period_ticks": 0, # Converted to heartbeat_m by Baker
            
            # --- Synaptic & Timings (16/8-bit) ---
            "initial_synapse_weight": 1000,
            "gsop_potentiation": 0,
            "gsop_depression": 0,
            "homeostasis_decay": 0,
            "refractory_period": 10,      # Calibrated (ticks)
            "synapse_refractory_period": 15,
            "signal_propagation_length": 20,
            "is_inhibitory": is_inhibitory,
            
            # --- Inertia & AHP ---
            "inertia_curve": [0, 0, 0, 0, 0, 0, 0, 0], # Compressed to 8 points
            "ahp_amplitude": 0,           # Variant A AHP (uV)
            "adaptive_leak_min_shift": 0,
            "adaptive_leak_gain": 0,
            "adaptive_mode": 0,
            
            # --- Receptors ---
            "d1_affinity": 64 if is_inhibitory else 192,
            "d2_affinity": 256 if is_inhibitory else 128
        }]
    }

def init_catalog_dir():
    base_dir = Path(__file__).parent.parent.parent / "Axicor_Neuron-Lib"
    base_dir.mkdir(parents=True, exist_ok=True)
    return base_dir

if __name__ == "__main__":
    cat_dir = init_catalog_dir()
    print(f"[*] Catalog scaffold initialized at {cat_dir}")
