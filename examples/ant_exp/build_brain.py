#!/usr/bin/env python3
import os
import sys
import subprocess

if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("[ERROR] ERROR: Virtual environment not active!")
    sys.exit(1)

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "axicor-client")))
from axicor.builder import BrainBuilder

def build_ant_connectome():
    print(" Initializing architect (3-Zone Feedforward WTA)...")
    base_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
    gnm_path = os.path.join(base_path, "GNM-Library")
    out_dir = os.path.join(base_path, "Axicor-Models/AntConnectome")

    builder = BrainBuilder(project_name="AntConnectome", output_dir=out_dir, gnm_lib_path=gnm_path)
    
    # [BATCH_SIZE FIX] 2ms HFT Batch for ultra-fast response
    builder.sim_params["sync_batch_ticks"] = 20
    builder.sim_params["tick_duration_us"] = 100

    # ============================================================
    # HARDWARE PROFILES (Strict C-ABI Constants)
    # ============================================================
    # [DOD FIX] No hardcoded plasticity! Use Mass Domain and library defaults.
    exc_type = builder.gnm_lib("VISp4/141")
    inh_type = builder.gnm_lib("VISp4/114")

    motor_type = builder.gnm_lib("VISp4/141")
    motor_type.name = "Motor_Pyramidal"
    for d in motor_type.data_list:
        d["name"] = "Motor_Pyramidal"
        d["dendrite_radius_um"] = 500.0     # Wide capture for signal convergence

    # ============================================================
    # SHARD 1: SENSORY CORTEX (Input Gateway)
    # ============================================================
    sensory = builder.add_zone("SensoryCortex", width_vox=64, depth_vox=64, height_vox=16)

    # [DOD FIX] Reduce density to 10% to stay within the 128-slot limit
    sensory.add_layer("L4_Input", height_pct=1.0, density=0.10) \
           .add_population(exc_type, fraction=0.9) \
           .add_population(inh_type, fraction=0.1)

    sensory.add_input("ant_sensors", width=28, height=16, entry_z="top")
    sensory.add_output("to_thoracic", width=16, height=16)

    # ============================================================
    # SHARD 2: THORACIC GANGLION (Pattern Generator / Hub)
    # ============================================================
    thoracic = builder.add_zone("ThoracicGanglion", width_vox=64, depth_vox=64, height_vox=32)

    thoracic.add_layer("L_Lower", height_pct=0.5, density=0.08) \
            .add_population(exc_type, fraction=0.7) \
            .add_population(inh_type, fraction=0.3)

    thoracic.add_layer("L_Upper", height_pct=0.5, density=0.08) \
            .add_population(exc_type, fraction=0.6) \
            .add_population(inh_type, fraction=0.4)

    thoracic.add_output("to_motor", width=16, height=16)

    # ============================================================
    # SHARD 3: MOTOR CORTEX (Output Gateway + Winner-Takes-All)
    # ============================================================
    motor = builder.add_zone("MotorCortex", width_vox=64, depth_vox=64, height_vox=32)

    motor.add_layer("L5_Lower", height_pct=0.6, density=0.10) \
         .add_population(motor_type, fraction=0.4) \
         .add_population(inh_type, fraction=0.6)

    motor.add_layer("L5_Upper", height_pct=0.4, density=0.08) \
         .add_population(exc_type, fraction=1.0)

    motor.add_output("motor_out", width=16, height=8, target_type="Motor_Pyramidal")

    # ============================================================
    # WIRING (Ghost Axons Routing)
    # ============================================================
    print("[*] Routing axons...")
    builder.connect(sensory, thoracic, out_matrix="to_thoracic", 
                    in_width=16, in_height=16, entry_z="bottom", growth_steps=1200)

    builder.connect(thoracic, motor, out_matrix="to_motor", 
                    in_width=16, in_height=16, entry_z="bottom", target_type="Motor_Pyramidal", growth_steps=1500)

    builder.build()

    print("\n Launching Axicor Baker (CPU Compiler)...")
    brain_toml_path = os.path.join(out_dir, "brain.toml")

    result = subprocess.run([
        "cargo", "run", "--release", "-p", "axicor-baker", "--bin", "axicor-baker", "--",
        "--brain", brain_toml_path, "--clean", "--yes"
    ], cwd=base_path)

    if result.returncode != 0:
        print("\n[ERROR] Connectome compilation failed.")
        sys.exit(1)

if __name__ == '__main__':
    build_ant_connectome()
