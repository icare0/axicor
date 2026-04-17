#=============================================================
#       ВРЕМЕННО НЕ РАБОТАЕТ / ТРЕБУЕТ РЕФАКТОРИНГА
#=============================================================

#!/usr/bin/env python3
import os
import sys
import time
import gymnasium as gym
import numpy as np

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "genesis-client")))

from genesis.utils import fnv1a_32
from genesis.client import GenesisMultiClient
from genesis.contract import GenesisIoContract
from genesis.control import GenesisControl
from genesis.tuner import GenesisAutoTuner, Phase
from genesis.memory import GenesisMemory

# ============================================================
# HFT & TUNER CONSTANTS
# ============================================================
EPISODES = 20_000_000
BATCH_SIZE = 20
PHISICS_SIMULATION_STEP = 0.002
ENCODER_SIGMA = 0.2

PHASE_PARAMS = {
    Phase.EXPLORATION: {
        'dopamine_reward': 10,
        'dopamine_pulse': -5,
        'dopamine_punish': -255,
        'shock_base': 5,
        'shock_max_batches': 20,
    },
    Phase.DISTILLATION: {
        'dopamine_reward': 20,
        'dopamine_pulse': -10,
        'dopamine_punish': -255,
        'shock_base': 5,
        'shock_max_batches': 25,
    },
    Phase.CRYSTALLIZED: {
        'dopamine_reward': 20,
        'dopamine_pulse': -10,
        'dopamine_punish': -15,
        'shock_base': 1,
        'shock_max_batches': 3,
    }
}

def run_ant():
    global BATCH_SIZE
    
    # 1. Контракты Авто-подключения
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../Genesis-Models/AntConnectome/baked"))
    
    contract_sensory = GenesisIoContract(os.path.join(base_dir, "SensoryCortex"), "SensoryCortex")
    contract_motor = GenesisIoContract(os.path.join(base_dir, "MotorCortex"), "MotorCortex")

    cfg_in = contract_sensory.get_client_config(BATCH_SIZE)
    cfg_out = contract_motor.get_client_config(BATCH_SIZE)

    # Клиент: Шлем в Sensory, Читаем только Motor
    client = GenesisMultiClient(
        addr=("127.0.0.1", 8081),
        matrices=cfg_in["matrices"],
        rx_layout=cfg_out["rx_layout"],
        timeout=0.5
    )

    try:
        client.sock.bind(("0.0.0.0", 8092))
    except OSError as e:
        print(f"❌ FATAL: Port 8092 is busy! Kill zombie agents. Error: {e}")
        sys.exit(1)

    # 2. Фабрика DOD Энкодеров/Декодеров
    # У Ant-v4 27 переменных (добиваем 1 нулем до 28 для выравнивания)
    encoder = contract_sensory.create_population_encoder("ant_sensors", vars_count=28, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)
    decoder = contract_motor.create_pwm_decoder("motor_out", batch_size=BATCH_SIZE)

    control = GenesisControl(os.path.join(base_dir, "MotorCortex", "manifest.toml"))
    
    tuner = GenesisAutoTuner(
        control,
        target_score=3000.0, # Муравей может набирать тысячи очков
        explore_prune=750, explore_night=30_000, explore_sprouts=128,
        distill_prune=1500, distill_night=150_000, distill_sprouts=16,
        crystallized_prune=5000, crystallized_night=50_000, crystallized_sprouts=16
    )

    # Аналитика (следим только за Моторной Корой)
    print("⏳ Ожидание инициализации MotorCortex Shared Memory...")
    memory = None
    for i in range(20):
        try:
            memory = GenesisMemory(contract_motor.zone_hash, read_only=False)
            print("✅ Telemetry Plane (MotorCortex) подключен!")
            break
        except Exception:
            time.sleep(1)

    env = gym.make("Ant-v4", exclude_current_positions_from_observation=False).unwrapped
    
    # 28 переменных нормализации (27 реальных + 1 паддинг)
    bounds = np.array([[-30.0, 30.0]] * 28, dtype=np.float16)
    range_diff = bounds[:, 1] - bounds[:, 0]

    episodes, score = 0, 0
    
    # Преаллокация для действий
    action_buffer = np.zeros(8, dtype=np.float32)
    obs_padded = np.zeros(28, dtype=np.float16)

    print(f"🚀 Starting Genesis DOD Ant Loop (Lockstep BATCH_SIZE={BATCH_SIZE})...")
    current_params = PHASE_PARAMS[Phase.EXPLORATION]

    state, _ = env.reset()

    while episodes < EPISODES:
        # [Zero-Garbage Pipeline]
        # Паддинг 27 obs -> 28
        obs_padded[:27] = state[:27]
        norm_state = np.clip((obs_padded - bounds[:, 0]) / range_diff, 0.0, 1.0)

        # Инъекция дофамина
        dopamine_signal = current_params['dopamine_pulse']
        if score > 0 and score % 20 == 0:
            dopamine_signal = current_params['dopamine_reward']

        # [DOD FIX] Zero-Allocation Pipeline
        encoder.encode_into(norm_state, client.payload_views)
        
        # Блокировка на барьере
        rx = client.step(dopamine_signal)
        
        # Извлекаем спайки моторов
        total_motor = decoder.decode_from(rx)
        
        # 128 моторных нейронов / 8 суставов = 16 нейронов на сустав
        # Усредняем активацию популяции (0.0 .. 1.0) и переводим в (-1.0 .. 1.0)
        for i in range(8):
            group_act = np.mean(total_motor[i*16 : (i+1)*16])
            action_buffer[i] = (group_act * 2.0) - 1.0

        state, reward, terminated, truncated, _ = env.step(action_buffer)
        score += reward

        if terminated or truncated:
            # Шок от падения/смерти
            shock_batches = current_params['shock_base']
            for _ in range(shock_batches):
                client.step(current_params['dopamine_punish'])

            stats = memory.get_network_stats() if memory else {"active_synapses": 0, "avg_weight": 0.0}
            
            current_phase = tuner.step(score)
            current_params = PHASE_PARAMS[current_phase]

            print(f"Ep {episodes:04d} | Score: {score:4.0f} | Phase: {current_phase.name:15s} | Synapses (MC): {stats['active_synapses']} | Avg W: {stats['avg_weight']:.1f}")

            state, _ = env.reset()
            score = 0
            episodes += 1

if __name__ == '__main__':
    run_ant()
