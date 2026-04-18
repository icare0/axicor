#!/usr/bin/env python3
import os
import sys
import numpy as np

# Virtual environment activation check
if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("[ERROR] ERROR: Virtual environment not active!")
    sys.exit(1)

try:
    import optuna
    import gymnasium as gym
except ImportError:
    print("[ERROR] ERROR: optuna or gymnasium not installed. Run: pip install optuna gymnasium")
    sys.exit(1)

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "axicor-client")))
from axicor.client import AxicorMultiClient
from axicor.encoders import PopulationEncoder
from axicor.decoders import PwmDecoder
from axicor.control import AxicorControl
from axicor.utils import fnv1a_32
from axicor.contract import AxicorIoContract


# ==========================================
# 1. Global Initialization (Zero-Downtime)
# ==========================================
# [DOD FIX] Synchronized with build_brain.py (10 ticks = 80 bytes C-ABI)
BATCH_SIZE = 10

# [DOD FIX] Path synchronization with the current model
baked_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../Axicor-Models/cartpole_exp/baked/MotorCortex"))
manifest_path = os.path.join(baked_dir, "manifest.toml")

if not os.path.exists(manifest_path):
    print(f"[ERROR] FATAL: Control Plane manifest NOT FOUND at {manifest_path}")
    sys.exit(1)

contract = AxicorIoContract(baked_dir, "MotorCortex")
zone_hash = contract.zone_hash

print(" Connecting to Axicor Node (Data & Memory Planes)...")
# [DOD FIX] Use contract for client configuration (including RX Layout)
client_cfg = contract.get_client_config(BATCH_SIZE)
client = AxicorMultiClient(addr=("127.0.0.1", 8081), **client_cfg)

try:
    client.sock.bind(("0.0.0.0", 8092))
except OSError as e:
    print(f"[ERROR] FATAL: Port 8092 is busy! Kill zombie agents before running. Error: {e}")
    sys.exit(1)

# [DOD FIX] Automatic encoder/decoder creation from contract
encoder = contract.create_population_encoder("sensors", vars_count=4, batch_size=BATCH_SIZE, sigma=0.2)
# [DOD FIX] Connect both physical hemispheres for fair Optuna evaluation
dec_left = contract.create_pwm_decoder("motor_left", batch_size=BATCH_SIZE)
dec_right = contract.create_pwm_decoder("motor_right", batch_size=BATCH_SIZE)

out_l_sz = contract.outputs["motor_left"]["width"] * contract.outputs["motor_left"]["height"] * BATCH_SIZE
out_r_sz = contract.outputs["motor_right"]["width"] * contract.outputs["motor_right"]["height"] * BATCH_SIZE

control = AxicorControl(manifest_path)
memory = AxicorMemory(zone_hash, read_only=False)

bounds = np.array([[-2.4, 2.4], [-3.0, 3.0], [-0.41, 0.41], [-2.0, 2.0]], dtype=np.float16)
range_diff = bounds[:, 1] - bounds[:, 0]

# ==========================================
# 2. Optuna Objective
# ==========================================
def objective(trial):
    # 1. Sample R-STDP hyperparameters (Full spectrum)
    dopamine_pulse = trial.suggest_int("dopamine_pulse", -255, -1)
    dopamine_reward = trial.suggest_int("dopamine_reward", 1, 255)
    prune_threshold = trial.suggest_int("prune_threshold", 2, 30)
    d1_affinity = trial.suggest_int("d1_affinity", 64, 255)
    d2_affinity = trial.suggest_int("d2_affinity", 64, 255)

    # Membrane Physics (GLIF)
    # leak_rate: how fast the neuron forgets incoming potential
    leak_rate = trial.suggest_int("leak_rate", 100, 3000)
    # homeostasis_penalty: how strongly the neuron "tires" after a spike
    homeostasis_penalty = trial.suggest_int("homeostasis_penalty", 500, 10000)
    # homeostasis_decay: how fast the neuron "recovers"
    homeostasis_decay = trial.suggest_int("homeostasis_decay", 1, 100)

    # 2. Hot-Patching Control Plane
    control.set_prune_threshold(prune_threshold)
    control.set_night_interval(2000) # Fixed sleep frequency for rapid mutations
    
    # Apply receptors to types (0: Excitatory, 1: Inhibitory)
    control.set_dopamine_receptors(0, d1_affinity, d2_affinity)
    control.set_dopamine_receptors(1, d1_affinity, d2_affinity)
    
    # Apply membrane physics (Hot-Patching VRAM LUT)
    control.set_membrane_physics(0, leak_rate, homeostasis_penalty, homeostasis_decay)
    control.set_membrane_physics(1, int(leak_rate * 1.5), int(homeostasis_penalty * 0.8), homeostasis_decay)

    # 3. Tabula Rasa (Surgical VRAM erasure)
    memory.clear_weights()
    # [DOD FIX] Hard reset of electrical state and homeostasis!
    memory.voltage.fill(0)
    memory.flags.fill(0)
    memory.threshold_offset.fill(0)
    memory.timers.fill(0)

    # [DOD FIX] Keep original environment physics (tau = 0.02s = 20 ms)
    env = gym.make("CartPole-v1").unwrapped
    state, _ = env.reset()
    norm_state = np.zeros(4, dtype=np.float16)
    
    # [DOD FIX] Fail-Fast Survival Metrics (30 seconds hold = Absolute Success)
    global_steps = 0
    max_score = 0
    MAX_STEPS = 1500
    score = 0
    terminated, truncated = False, False

    # Run the network until it dies from epilepsy or hits the time limit
    while global_steps < MAX_STEPS:

        if terminated or truncated:
            max_score = max(max_score, score)
            
            # Formally broadcast error frame to VRAM
            encoder.encode_into(norm_state, client.payload_views[0])
            
            # Pain shock for 20 batches (20 ms biological time)
            for _ in range(20):
                client.step(-255)

            stats = memory.get_network_stats()
            avg_w = stats["avg_weight"]
            synapses = stats["active_synapses"]

            # [DOD FIX] Graceful Fatality: Network burned out. Record survival time.
            if synapses > 0 and (avg_w > 25000 or avg_w < 10):
                break

            state, _ = env.reset()
            score = 0
            terminated, truncated = False, False
            continue

        # Zero-Cost Normalization (In-Place)
        np.subtract(state, bounds[:, 0], out=norm_state, casting='unsafe')
        np.divide(norm_state, range_diff, out=norm_state, casting='unsafe')
        np.clip(norm_state, 0.0, 1.0, out=norm_state, casting='unsafe')

        # [DOD FIX] Continuous Error Gradient (Zero Branches)
        pole_angle = abs(state[2])
        pole_velocity = abs(state[3])

        # 1. Error normalization (0.0 = ideal, 1.0 = crash)
        angle_error = min(1.0, pole_angle / 0.2094)
        vel_error = min(1.0, pole_velocity / 2.0)

        # 2. Weighted error (maintain focus on angle, but dampen sway)
        error = min(1.0, angle_error * 0.8 + vel_error * 0.2)

        # 3. Dopamine linear algebra (without if/else)
        dop_sig = int(dopamine_reward * (1.0 - error) + dopamine_pulse * error)

        # [DOD FIX] Accelerate brain to physics speed (0.02s = 20 ms). 
        # BATCH_SIZE = 10 ticks (1 ms). We need 20 batches.
        force_left = 0.0
        force_right = 0.0
        
        encoder.encode_into(norm_state, client.payload_views[0])
        
        for _ in range(20):
            rx = client.step(dop_sig)
            rx_view = memoryview(rx)
            
            # [DOD FIX] Strict L7 demultiplexing across two hemispheres
            motor_l = dec_left.decode_from(rx_view[0 : out_l_sz])
            motor_r = dec_right.decode_from(rx_view[out_l_sz : out_l_sz + out_r_sz])
            
            force_left += np.sum(motor_l)
            force_right += np.sum(motor_r)
        
        action = 0 if force_left > force_right else 1

        state, reward, terminated, truncated, _ = env.step(action)
        score += 1
        global_steps += 1

    env.close()
    
    # Save side analytics for dashboard
    max_score = max(max_score, score)
    trial.set_user_attr("max_score", max_score)
    
    # [DOD FIX] Composite Metric: Survival + Skill (with hard priority for skill in immortals)
    return float(global_steps + (max_score * 1000))

if __name__ == '__main__':
    # Reduce noise from Optuna logs
    optuna.logging.set_verbosity(optuna.logging.WARNING) # [DOD FIX] Mute default garbage

    # [DOD FIX] Zero-Cost Telemetry Hook
    def hft_telemetry_callback(study, trial):
        max_score = trial.user_attrs.get("max_score", 0)
        print(f" Trial {trial.number:03d} | Survived: {trial.value:5.0f} steps | MAX SCORE: {max_score:5.0f} | "
              f"Pulse: {trial.params.get('dopamine_pulse')}, Reward: {trial.params.get('dopamine_reward')}")

    study = optuna.create_study(direction="maximize", pruner=optuna.pruners.MedianPruner())
    print(" Starting Zero-Downtime Bayesian Search...")
    
    try:
        # Run 200 trials
        study.optimize(objective, n_trials=200, callbacks=[hft_telemetry_callback])
        print("\n Best Hyperparameters:")
        for key, value in study.best_trial.params.items():
            print(f"  {key}: {value}")
        print(f" Best Score: {study.best_value:.1f}")
    except KeyboardInterrupt:
        print("\nSearch interrupted by user.")
