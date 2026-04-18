#========================================================
#       SDK AND R-STDP INTEGRATION IN PROGRESS
#========================================================

#!/usr/bin/env python3
import os
import sys
import time
import numpy as np
import re
import importlib.util
from pathlib import Path

# Virtual environment activation check
if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("[ERROR] ERROR: Virtual environment not active!")
    sys.exit(1)

# Add SDK path ( axicor-client/ ) if script is run directly from the example
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "axicor-client")))

from axicor.utils import fnv1a_32
from axicor.client import AxicorMultiClient
from axicor.contract import AxicorIoContract
from axicor.control import AxicorControl
from axicor.memory import AxicorMemory

#============================================================
#       CLIENT & ENVIRONMENT SETTINGS
#============================================================
BATCH_SIZE = 20                 # HFT-cycle: 1 packet = 20 ticks (Must match tick_duration_us in build_brain.py)
ENCODER_SIGMA = 0.2             # Encoder sigma (feature spread)

def sanitize_flygym_xmls():
    """
    [DOD] Self-Healing mechanism. 
    Surgically removes attributes that crash the dm_control parser in newer Python versions.
    """
    spec = importlib.util.find_spec("flygym")
    if not spec or not spec.submodule_search_locations:
        return
    
    base_path = Path(spec.submodule_search_locations[0]) / "data"
    if not base_path.exists():
        return

    # Attributes causing AttributeError in newer MuJoCo versions
    problematic_attrs = ["convexhull", "mpr_iterations", "collision"]

    for xml_file in base_path.rglob("*.xml"):
        try:
            content = xml_file.read_text(encoding="utf-8")
            modified = False
            
            for attr in problematic_attrs:
                if f'{attr}=' in content:
                    # Burn attribute using regex
                    content = re.sub(fr'\s*{attr}="[^"]*"', '', content)
                    modified = True
            
            if modified:
                xml_file.write_text(content, encoding="utf-8")
                print(f" [Self-Healing] Patched XML ({'+'.join(problematic_attrs)}): {xml_file.name}")
        except Exception as e:
            print(f"[WARN] [Self-Healing] Failed to patch {xml_file.name}: {e}")

def run_fly():
    # ============================================================
    # 1. Multi-Port Binding (Zero-Copy Contracts)
    # ============================================================
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../Axicor-Models/FLY_exp/baked"))

    # Load 4 contracts
    c_cx = AxicorIoContract(os.path.join(base_dir, "CX"), "CX")
    c_an = AxicorIoContract(os.path.join(base_dir, "AN"), "AN")
    c_vp = AxicorIoContract(os.path.join(base_dir, "VP"), "VP")
    c_desc = AxicorIoContract(os.path.join(base_dir, "DESCENDING"), "DESCENDING")

    # Assemble a single matrix array for the UDP multiplexer
    # Assembly order is critical: it determines client.payload_views indices
    matrices = (
        c_cx.get_client_config(BATCH_SIZE)["matrices"] +       # view[0]
        c_an.get_client_config(BATCH_SIZE)["matrices"] +       # view[1]
        c_vp.get_client_config(BATCH_SIZE)["matrices"] +       # view[2]
        c_desc.get_client_config(BATCH_SIZE)["matrices"]       # view[3]
    )
    rx_layout = c_desc.get_client_config(BATCH_SIZE)["rx_layout"]

    client = AxicorMultiClient(
        addr=("127.0.0.1", 8081),
        matrices=matrices,
        rx_layout=rx_layout,
        timeout=0.5
    )

    try:
        client.sock.bind(("0.0.0.0", 8092))
    except OSError as e:
        print(f"[ERROR] FATAL: Port 8092 is busy! Kill zombie agents. Error: {e}")
        sys.exit(1)

    # ============================================================
    # 2. DOD Encoder and Decoder Factory
    # ============================================================
    enc_nav = c_cx.create_population_encoder("navigation", vars_count=15, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)
    enc_halt = c_an.create_population_encoder("haltere", vars_count=10, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)
    enc_prop = c_vp.create_population_encoder("proprioception", vars_count=42, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)
    enc_refl = c_desc.create_population_encoder("reflexes", vars_count=6, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)

    dec_mot = c_desc.create_pwm_decoder("motors", batch_size=BATCH_SIZE)

    # ============================================================
    # 3. RL REACTOR (EXPLORATION PHASE)
    # Atomic manifest rewriting for aggressive learning
    # ============================================================
    print(" Initializing RL Reactor (Phase: EXPLORATION)...")
    ctrl_desc = AxicorControl(os.path.join(base_dir, "DESCENDING", "manifest.toml"))
    # Frequent sleep (every 30k ticks) for rapid consolidation
    ctrl_desc.set_night_interval(30_000)
    # Low pruning threshold (allow neurons to make mistakes and keep weak connections)
    ctrl_desc.set_prune_threshold(750)
    # Aggressive structural growth (many new axons each night)
    ctrl_desc.set_max_sprouts(128)

    # ============================================================
    # 4. Environment and Memory Preallocation
    # ============================================================
    print(" Launching asset preprocessor...")
    sanitize_flygym_xmls()

    from flygym.mujoco import NeuroMechFly
    import mujoco.viewer

    print(" Initializing NeuroMechFly (FlyGym)...")
    env = NeuroMechFly()
    state, _ = env.reset()
    viewer = mujoco.viewer.launch_passive(env.physics.model.ptr, env.physics.data.ptr)

    # ============================================================
    # EXPLICIT I/O BLOCK (MULTI-PORT FACADES)
    # ============================================================
    # 1. Navigation (CX: 16 slots)
    buf_nav = np.zeros(16, dtype=np.float16)
    bounds_nav = np.zeros((16, 2), dtype=np.float16)
    bounds_nav[0:12] = [-50.0, 50.0]  # fly_pos, vel
    bounds_nav[12:15] = [-3.15, 3.15] # ori
    rd_nav = bounds_nav[:, 1] - bounds_nav[:, 0]
    rd_nav[rd_nav == 0] = 1.0

    # 2. Haltere (AN: 16 slots)
    buf_halt = np.zeros(16, dtype=np.float16)
    bounds_halt = np.zeros((16, 2), dtype=np.float16)
    bounds_halt[0:10] = [-10.0, 10.0]
    rd_halt = bounds_halt[:, 1] - bounds_halt[:, 0]
    rd_halt[rd_halt == 0] = 1.0

    # 3. Proprioception (VP: 64 slots)
    buf_prop = np.zeros(64, dtype=np.float16)
    bounds_prop = np.zeros((64, 2), dtype=np.float16)
    bounds_prop[0:42] = [-3.15, 3.15] # Joint angles
    rd_prop = bounds_prop[:, 1] - bounds_prop[:, 0]
    rd_prop[rd_prop == 0] = 1.0

    # 4. Reflexes (DESCENDING: 16 slots)
    buf_refl = np.zeros(16, dtype=np.float16)
    bounds_refl = np.zeros((16, 2), dtype=np.float16)
    bounds_refl[0:6] = [0.0, 100.0] # Contact forces
    rd_refl = bounds_refl[:, 1] - bounds_refl[:, 0]
    rd_refl[rd_refl == 0] = 1.0

    # Output facade
    avatar_out = c_desc.create_output_facade("motors", dec_mot._out_buffer)
    action_buffer = np.zeros(42, dtype=np.float32)

    episodes = 0
    print(f" Starting Axicor DOD FLY Loop (Lockstep BATCH_SIZE={BATCH_SIZE})...")

    # ============================================================
    # 4. HFT Hot Loop (Explicit Routing)
    # ============================================================
    while True:
        # ===================================================================
        # SENSORS (Zero-Cost Bulk Copy & SIMD Compute)
        # ===================================================================
        # 1. Navigation
        buf_nav[0:12] = state["fly"].flatten()
        buf_nav[12:15] = state["fly_orientation"]

        # 2. Haltere (Placeholder, 0.0)
        
        # 3. Proprioception (Joint angles only - row 0 of 3x42 matrix)
        buf_prop[0:42] = state["joints"][0, :]

        # 4. Reflexes (Compress 30 contact points into 6 legs without loops)
        # state["contact_forces"] shape: (30, 3) - 5 sensors per 6 legs
        # Vectorized: compute norm, reshape, sum across sensors
        forces = np.linalg.norm(state["contact_forces"].reshape(6, 5, 3), axis=2).sum(axis=1)
        buf_refl[0:6] = forces

        # Normalization
        norm_nav = np.clip((buf_nav - bounds_nav[:, 0]) / rd_nav, 0.0, 1.0)
        norm_halt = np.clip((buf_halt - bounds_halt[:, 0]) / rd_halt, 0.0, 1.0)
        norm_prop = np.clip((buf_prop - bounds_prop[:, 0]) / rd_prop, 0.0, 1.0)
        norm_refl = np.clip((buf_refl - bounds_refl[:, 0]) / rd_refl, 0.0, 1.0)

        # ===================================================================
        # TRANSPORT TO VRAM AND DOPAMINE
        # ===================================================================
        # [DOD FIX] Pass strict O(1) slice of active variables (no padding) 
        # to enable Zero-Cost Broadcasting in NumPy.
        enc_nav.encode_into(norm_nav[:15], client.payload_views[0])
        enc_halt.encode_into(norm_halt[:10], client.payload_views[1])
        enc_prop.encode_into(norm_prop[:42], client.payload_views[2])
        enc_refl.encode_into(norm_refl[:6],  client.payload_views[3])

        # [DOD] Extract linear velocity along X-axis (index 3 of flattened 4x3 matrix)
        # state["fly"] contains pos(3), vel(3), ang_pos(3), ang_vel(3)
        # vel_x - is index 3 (if counting from 0)
        vel_x = state["fly"].flatten()[3]

        # Calculate reward within i16 bounds (-32768 .. 32767)
        if vel_x > 0.02:
            # Moving forward: strong dopamine surge
            dopamine = int(min(vel_x * 1000.0, 32767))
        else:
            # Idle or moving backward: background connection erosion (LTD)
            dopamine = -15

        rx = client.step(dopamine)  # C-ABI header packing and transport

        # ===================================================================
        # MOTORS
        # ===================================================================
        dec_mot.decode_from(rx)
        
        MOTOR_GAIN = 0.2
        # Bulk O(1) copy of 42 decoded signals
        action_buffer[:] = (avatar_out.raw_buffer[:42] - 0.5) * 2.0 * MOTOR_GAIN
        action = {"joints": action_buffer}

        # ===================================================================
        #                           PHYSICS
        # ===================================================================
        try:
            step_result = env.step(action)
            if len(step_result) == 5:
                state, reward, terminated, truncated, _ = step_result
            else:
                state, reward, terminated, info = step_result
                truncated = False
        except Exception as e:
            print(f" Simulation Exploded: {e}")
            state, _ = env.reset()
            continue

        if viewer and viewer.is_running():
            viewer.sync()
            time.sleep(0.002)

        if terminated or truncated:
            state, _ = env.reset()
            episodes += 1
            print(f"Ep {episodes:04d} | Reset")

if __name__ == '__main__':
    run_fly()
