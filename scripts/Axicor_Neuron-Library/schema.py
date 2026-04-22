#!/usr/bin/env python3
import toml
from pathlib import Path
from typing import Dict, Any

# [DOD] C-ABI Freeze (64 bytes invariant). 
# Эта схема жестко привязана к axicor-core/src/layout.rs
def generate_scaffold(name: str, is_inhibitory: bool) -> Dict[str, Any]:
    # [DOD FIX] Biological Morphology Divergence
    if is_inhibitory:
        growth_bias = 0.5   # Horizontal dense cloud
        fov = 120.0         # Wide search angle
    else:
        growth_bias = 0.95  # Vertical projector
        fov = 34.4          # Laser focus

    return {
        "neuron_type": [{
            "name": name,
            
            # --- Membrane (32-bit) ---
            "threshold": 0,               # Will be calibrated (uV)
            "rest_potential": 0,          # Will be calibrated (uV)
            "leak_shift": 4,              # Exponential shift
            "homeostasis_penalty": 0,
            "spontaneous_firing_period_ticks": 0,
            
            # --- Synaptic & Timings (16/8-bit) ---
            "initial_synapse_weight": 12000,
            "gsop_potentiation": 400,
            "gsop_depression": 450,
            "homeostasis_decay": 2,
            "refractory_period": 10,      # Ticks
            "synapse_refractory_period": 15,
            "signal_propagation_length": 20,
            "is_inhibitory": is_inhibitory,
            
            # --- Inertia & AHP ---
            # [DOD FIX] Enable GSOP STDP Math! 
            # Ranks 0..7 resistance. Weak = easy to learn, Strong = monumental.
            "inertia_curve": [1, 2, 3, 4, 5, 6, 7, 8],
            "ahp_amplitude": 0,
            "adaptive_leak_min_shift": 0,
            "adaptive_leak_gain": 0,
            "adaptive_mode": 0,
            
            # --- Receptors ---
            "d1_affinity": 64 if is_inhibitory else 192,
            "d2_affinity": 255 if is_inhibitory else 128,

            # --- Missing Baker Fields (Morphology & Memory) ---
            "steering_fov_deg": fov,
            "steering_radius_um": 66.7,
            "growth_vertical_bias": growth_bias,
            "dendrite_radius_um": 420.0,
            "type_affinity": 0.8,
            "sprouting_weight_distance": 0.5,
            "sprouting_weight_power": 0.4,
            "sprouting_weight_explore": 0.167,
            "sprouting_weight_type": 0.2,
            "steering_weight_inertia": 0.5,
            "steering_weight_sensor": 0.4,
            "steering_weight_jitter": 0.1,
            "prune_threshold": 1000,
            "slot_decay_ltm": 75,
            "slot_decay_wm": 233,
            "ltm_slot_count": 86,
            "heartbeat_m": 0
        }]
    }

def init_catalog_dir():
    base_dir = Path(__file__).parent.parent.parent / "Axicor_Neuron-Lib"
    base_dir.mkdir(parents=True, exist_ok=True)
    return base_dir

if __name__ == "__main__":
    cat_dir = init_catalog_dir()
    print(f"[*] Catalog scaffold initialized at {cat_dir}")
