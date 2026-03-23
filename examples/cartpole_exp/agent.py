#=======================================================
#   Физика 0.02 - Score 9-11 это PRNG мусор
#   Пример всё еще Експериментальный но уже обучаемый
#=======================================================

#!/usr/bin/env python3
import os
import sys
import time
import numpy as np

# Проверка окружения
if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("❌ ERROR: Virtual environment not active!")
    sys.exit(1)

import gymnasium as gym

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../../genesis-client")))
from genesis.client import GenesisMultiClient
from genesis.contract import GenesisIoContract
from genesis.control import GenesisControl
from genesis.tuner import GenesisAutoTuner, Phase
from genesis.memory import GenesisMemory

# [DOD FIX] Синхронизировано с build_brain.py (10 тиков = 80 байт C-ABI)
BATCH_SIZE = 10

def run_cartpole():
    print("🧠 Инициализация DOD-шлюза CartPole...")
    
    # ============================================================
    # 1. Multi-Port Binding (Zero-Copy Contracts)
    # ============================================================
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../Genesis-Models/cartpole_exp/baked"))
    contract = GenesisIoContract(os.path.join(base_dir, "MotorCortex"), "MotorCortex")
    
    client_cfg = contract.get_client_config(BATCH_SIZE)
    client = GenesisMultiClient(addr=("127.0.0.1", 8081), **client_cfg)
    
    try:
        client.sock.bind(("0.0.0.0", 8092))
    except OSError as e:
        print(f"❌ FATAL: Port 8092 is busy! Kill zombie agents. Error: {e}")
        sys.exit(1)

    # ============================================================
    # 2. RL Реактор (State Machine & Telemetry)
    # ============================================================
    print("🧬 Инициализация Авто-Тюнера и SHM...")
    ctrl = GenesisControl(os.path.join(base_dir, "MotorCortex", "manifest.toml"))
    # [DOD FIX] Агрессивный базовый прунинг
    ctrl.set_night_interval(30_000)
    ctrl.set_prune_threshold(1000)
    ctrl.set_max_sprouts(64)
    
    try:
        memory = GenesisMemory(contract.zone_hash, read_only=False)
    except Exception:
        memory = None
        print("⚠️ SHM not found. Running without memory telemetry.")

    PHASE_PARAMS = {
        Phase.EXPLORATION: {'dopamine_pulse': -5, 'dopamine_punish': -255, 'shock_base': 5, 'shock_vel_mult': 5, 'shock_max_batches': 20},
        Phase.DISTILLATION: {'dopamine_pulse': -10, 'dopamine_punish': -255, 'shock_base': 5, 'shock_vel_mult': 5, 'shock_max_batches': 25},
        Phase.CRYSTALLIZED: {'dopamine_pulse': -10, 'dopamine_punish': -15, 'shock_base': 1, 'shock_vel_mult': 1, 'shock_max_batches': 3}
    }

    tuner = GenesisAutoTuner(
        ctrl,
        target_score=1500.0,
        # Жесткая экономика: выживают только те связи, которые получают мощный LTP
        explore_prune=1000, explore_night=10_000, explore_sprouts=64,
        distill_prune=1500, distill_night=30_000, distill_sprouts=16,
        crystallized_prune=2500, crystallized_night=50_000, crystallized_sprouts=0
    )

    current_phase = Phase.EXPLORATION
    current_params = PHASE_PARAMS[current_phase]

    # ============================================================
    # 3. Фабрика DOD Энкодеров и Декодеров
    # ============================================================
    enc_sensors = contract.create_population_encoder("sensors", vars_count=4, batch_size=BATCH_SIZE, sigma=0.15)
    
    # Разделенные аппаратные порты
    dec_left = contract.create_pwm_decoder("motor_left", batch_size=BATCH_SIZE)
    dec_right = contract.create_pwm_decoder("motor_right", batch_size=BATCH_SIZE)

    # Динамический расчет смещений L7 фрагментации в UDP пакете
    out_l_sz = contract.outputs["motor_left"]["width"] * contract.outputs["motor_left"]["height"] * BATCH_SIZE
    out_r_sz = contract.outputs["motor_right"]["width"] * contract.outputs["motor_right"]["height"] * BATCH_SIZE

    # ============================================================
    # 4. Преаллокация Memory Arenas & Фасады
    # ============================================================
    buf_sensors = np.zeros(4, dtype=np.float16)
    bounds = np.zeros((4, 2), dtype=np.float16)
    bounds[0] = [-2.4, 2.4]   # cart_x
    bounds[1] = [-3.0, 3.0]   # cart_v
    bounds[2] = [-0.41, 0.41] # pole_a
    bounds[3] = [-2.0, 2.0]   # pole_av
    
    rd = bounds[:, 1] - bounds[:, 0]
    rd[rd == 0] = 1.0

    avatar_in = contract.create_input_facade("sensors", buf_sensors)
    avatar_left = contract.create_output_facade("motor_left", dec_left._out_buffer)
    avatar_right = contract.create_output_facade("motor_right", dec_right._out_buffer)

    env = gym.make("CartPole-v1")
    state, _ = env.reset()

    episodes = 0
    score = 0
    dopamine = 10

    print(f"🚀 Starting Lock-Free HFT Loop (BATCH_SIZE={BATCH_SIZE})...")

    while True:
        # ===================================================================
        # СЕНСОРЫ (Zero-Cost Bulk Copy & SIMD Compute)
        # ===================================================================
        # Прямая запись массива в C-память без питоновских циклов
        avatar_in.raw_buffer[:] = state
        
        # Векторизованная нормализация (O(1) по времени для Python)
        norm_state = np.clip((buf_sensors - bounds[:, 0]) / rd, 0.0, 1.0)
        
        # Транспорт в VRAM (1 вызов C-ABI)
        enc_sensors.encode_into(norm_state[:4], client.payload_views)

        # ===================================================================
        # ТРАНСПОРТ И ДОФАМИН (Temporal Sync: 10 батчей = 20 мс физики)
        # ===================================================================
        force_left = 0.0
        force_right = 0.0

        # Разгоняем мозг до скорости физики (0.02s = 20 мс). 
        # BATCH_SIZE = 10 тиков (1 мс). Нам нужно 20 батчей, чтобы прожить 20 мс.
        for _ in range(20):
            rx = client.step(dopamine)
            rx_view = memoryview(rx)
            
            # Жесткий L7-демультиплексинг
            dec_left.decode_from(rx_view[0 : out_l_sz])
            dec_right.decode_from(rx_view[out_l_sz : out_l_sz + out_r_sz])

            # Аккумулируем массу спайков за все 20 мс
            force_left += np.sum(avatar_left.raw_buffer)
            force_right += np.sum(avatar_right.raw_buffer)
        
        action = 0 if force_left > force_right else 1

        # ===================================================================
        # ФИЗИКА СРЕДЫ И R-STDP РЕАКТОР
        # ===================================================================
        state, reward, terminated, truncated, _ = env.step(action)
        score += 1

        if terminated or truncated:
            is_mute = (force_left <= 0.5 and force_right <= 0.5)

            if is_mute:
                total_shock = 0
            else:
                # [DOD] Индекс 2 — это pole_angle, индекс 3 — это pole_angular_velocity в CartPole. 
                # Чем быстрее падает, тем больше боль.
                shock_batches = current_params['shock_base'] + (score >> 5)
                kinetic_penalty = int(abs(state[2]) * current_params['shock_vel_mult'])
                total_shock = min(current_params['shock_max_batches'], shock_batches + kinetic_penalty)

            if total_shock > 0:
                for _ in range(total_shock):
                    client.step(current_params['dopamine_punish'])

            synapses, avg_w = 0, 0.0
            if memory:
                stats = memory.get_network_stats()
                synapses = stats["active_synapses"]
                avg_w = stats["avg_weight"]

            # Переключение фаз O(1)
            current_phase = tuner.step(score)
            current_params = PHASE_PARAMS[current_phase]
            phase_str = current_phase.name

            if synapses == 0:
                print(f"Ep {episodes:04d} | Score: {score:3d} | Phase: {phase_str[:3]} | Waiting for Night Phase DMA...")
            elif is_mute:
                print(f"Ep {episodes:04d} | Score: {score:3d} | Phase: {phase_str[:3]} | Syns: {synapses:5d} | AvgW: {avg_w:4.1f} | L: {force_left:.1f} R: {force_right:.1f} [WARMUP]")
            else:
                print(f"Ep {episodes:04d} | Score: {score:3d} | Phase: {phase_str[:3]} | Syns: {synapses:5d} | AvgW: {avg_w:4.1f} | L: {force_left:.1f} R: {force_right:.1f} [SHOCK: {total_shock}b]")

            state, _ = env.reset()
            score = 0
            episodes += 1
            dopamine = current_params['dopamine_pulse']
        else:
            # Фоновое закрепление выживания
            dopamine = 50

if __name__ == '__main__':
    run_cartpole()