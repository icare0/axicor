#!/usr/bin/env python3
import os
import sys
import time
import numpy as np
import gymnasium as gym

# Добавляем путь к SDK
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "axicor-client")))

from axicor.client import AxicorMultiClient
from axicor.encoders import PopulationEncoder
from axicor.decoders import PwmDecoder
from axicor.contract import AxicorIoContract
from axicor.brain import fnv1a_32
from axicor.control import AxicorControl


# CONFIG
EPISODES = 20_000_000
BATCH_SIZE = 20
TARGET_SCORE = 50_000           # 50k score (Condition de victoire)
TARGET_TIME = 10_000            # 10k steps (Temps max par essai)

# --- RL HYPERPARAMETERS ---
MOTOR_GAIN = 0.015               # Muscles plus forts (avant 0.008)
REWARD_SCALE = 8.0               # Exagère la récompense de vitesse (avant 5.0)
DOPAMINE_BASE = -12              # Rend l'immobilité DOULOUREUSE (avant -5)
DOPAMINE_PUNISHMENT = -120       # MUST BE NEGATIVE for LTD (Long-Term Depression)

def run_ant():
    env = gym.make("Ant-v4", render_mode="human", max_episode_steps=TARGET_TIME if TARGET_TIME > 0 else 1000)
    state, _ = env.reset()

    # --- 1. Load Zero-Cost Contracts ---
    print("Loading C-ABI Contracts for 6 zones...")
    model_path = "Axicor-Models/AntConnectome.axic"

    c_sensory = AxicorIoContract(model_path, "SensoryCortex")
    c_motor = AxicorIoContract(model_path, "MotorCortex")

    # --- CONTROL PLANE SDK ---
    ctrl_sensory = AxicorControl(model_path, "SensoryCortex")
    ctrl_motor = AxicorControl(model_path, "MotorCortex")
    
    # Configure initial plasticity and pruning to avoid Epileptic Seizures
    ctrl_motor.set_prune_threshold(30)
    ctrl_sensory.set_prune_threshold(30)
    ctrl_motor.set_max_sprouts(1)
    ctrl_sensory.set_max_sprouts(1)

    # Build combined layout
    cfg_sens = c_sensory.get_client_config(BATCH_SIZE)
    cfg_motor = c_motor.get_client_config(BATCH_SIZE)

    matrices = cfg_sens["matrices"] 
    rx_layout = cfg_motor["rx_layout"]

    # HFT Transport
    client = AxicorMultiClient(
        addr=("127.0.0.1", 8081),
        matrices=matrices,
        rx_layout=rx_layout
    )

    try:
        client.sock.bind(("0.0.0.0", 8092))
    except OSError:
        pass

    # --- 2. Zero-Garbage Encoders/Decoders ---
    encoder = c_sensory.create_population_encoder("ant_sensors", vars_count=27, batch_size=BATCH_SIZE)
    decoder = c_motor.create_pwm_decoder("motor_out", BATCH_SIZE)

    # --- MEMORY PREALLOCATION (Hot Loop Safety) ---
    action = np.zeros(8, dtype=np.float32)
    norm_state = np.zeros(27, dtype=np.float16)

    bounds = np.tile([-5.0, 5.0], (27, 1)).astype(np.float16)
    range_diff = bounds[:, 1] - bounds[:, 0]

    episodes, score, steps = 0, 0.0, 0
    terminated, truncated = False, False
    last_reward_signal = 0

    best_score = -9999.0
    is_crystallized = False

    print(f"🚀 Thalamo-Cortical-Spinal Hot Loop Started (Batch={BATCH_SIZE})...")

    try:
        while episodes < EPISODES:
            time_reached = (TARGET_TIME > 0 and steps >= TARGET_TIME)
            score_reached = (TARGET_SCORE >= 1000 and score >= TARGET_SCORE)

            if terminated or truncated or time_reached or score_reached:
                if time_reached: reason = "Time Reached"
                elif score_reached: reason = "Score Reached"
                else: reason = "Terminated"

                print(f"Ep {episodes:04d} | Score: {score:6.1f} | Steps: {steps:5d} | [{reason}]")

                # --- 1. Best Score Checkpoint ---
                if score > best_score and steps > 100: # Ensure it's a meaningful score
                    best_score = score
                    print(f"🔥 NOUVEAU RECORD : {best_score:.1f} ! Forçage de la sauvegarde (VRAM -> Disque)...")
                    # Temporarily reduce checkpoint interval to trigger a save
                    ctrl_motor.set_checkpoints_interval(10)
                    ctrl_sensory.set_checkpoints_interval(10)
                    for _ in range(10): client.step(0) # Let the node process the change
                    # Restore standard interval
                    ctrl_motor.set_checkpoints_interval(1_000_000)
                    ctrl_sensory.set_checkpoints_interval(1_000_000)

                # --- 2. Death Signal ---
                if not is_crystallized:
                    # Death Signal: 5 batches of moderate LTD
                    for _ in range(5):
                        client.step(DOPAMINE_PUNISHMENT)

                # --- 3. Sleep Phase (Pruning) ---
                if episodes > 0 and episodes % 25 == 0 and not is_crystallized:
                    print("🌙 Phase de sommeil PROFOND : élagage massif des connexions faibles...")
                    ctrl_motor.set_night_interval(1)
                    ctrl_sensory.set_night_interval(1)
                    for _ in range(50): client.step(0) # Let it sleep for a few ticks
                    ctrl_motor.set_night_interval(0)
                    ctrl_sensory.set_night_interval(0)

                episodes += 1
                state, _ = env.reset()
                score = 0.0
                steps = 0
                last_reward_signal = 0
                terminated, truncated = False, False
                continue

            # 1. Zero-Allocation Normalization
            np.subtract(state, bounds[:, 0], out=norm_state, dtype=np.float16)
            np.divide(norm_state, range_diff, out=norm_state, dtype=np.float16)
            np.clip(norm_state, 0.0, 1.0, out=norm_state)

            # 2. Encode to Network Buffer
            encoder.encode_into(norm_state, client.payload_views)

            # 3. Axicor Step (Network I/O)
            # Pass the reward from the PREVIOUS step to the network
            rx_raw = client.step(last_reward_signal)
            rx_view = memoryview(rx_raw)

            # 4. Zero-Copy Decoding
            motor_out = decoder.decode_from(rx_view)

            # 5. Fast Response Resolution (Slicing & Normalizing)
            # FL
            action[0] = (np.sum(motor_out[0:8]) - np.sum(motor_out[8:16])) * MOTOR_GAIN
            action[1] = (np.sum(motor_out[16:24]) - np.sum(motor_out[24:32])) * MOTOR_GAIN
            # FR
            action[2] = (np.sum(motor_out[32:40]) - np.sum(motor_out[40:48])) * MOTOR_GAIN
            action[3] = (np.sum(motor_out[48:56]) - np.sum(motor_out[56:64])) * MOTOR_GAIN
            # BL
            action[4] = (np.sum(motor_out[64:72]) - np.sum(motor_out[72:80])) * MOTOR_GAIN
            action[5] = (np.sum(motor_out[80:88]) - np.sum(motor_out[88:96])) * MOTOR_GAIN
            # BR
            action[6] = (np.sum(motor_out[96:104]) - np.sum(motor_out[104:112])) * MOTOR_GAIN
            action[7] = (np.sum(motor_out[112:120]) - np.sum(motor_out[120:128])) * MOTOR_GAIN
            
            np.clip(action, -1.0, 1.0, out=action)

            # 6. Physical Step
            state, reward, terminated, truncated, _ = env.step(action)
            
            # 7. Convert Physical Reward to SNN Dopamine
            # Higher gain for positive reinforcement, baseline for stability
            last_reward_signal = int(np.clip(reward * REWARD_SCALE + DOPAMINE_BASE, -128, 127))
            
            score += reward
            steps += 1

    except KeyboardInterrupt:
        print("\n🛑 Interruption manuelle demandée (Ctrl+C) !")
        print("💾 Sauvegarde de l'état du cerveau en VRAM vers le disque en cours...")
        try:
            ctrl_motor.set_checkpoints_interval(10)
            ctrl_sensory.set_checkpoints_interval(10)
            for _ in range(10): client.step(0)
            ctrl_motor.set_checkpoints_interval(1_000_000)
            ctrl_sensory.set_checkpoints_interval(1_000_000)
            print("✅ Cerveau sauvegardé avec succès dans le dossier .axic.mem !")
        except Exception as e:
            pass
    finally:
        env.close()

if __name__ == '__main__':
    run_ant()