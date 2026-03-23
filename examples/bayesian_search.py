#!/usr/bin/env python3
import os
import sys
import numpy as np

# Проверка активации виртуального окружения
if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("❌ ERROR: Virtual environment not active!")
    sys.exit(1)

try:
    import optuna
    import gymnasium as gym
except ImportError:
    print("❌ ERROR: optuna or gymnasium not installed. Run: pip install optuna gymnasium")
    sys.exit(1)

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "genesis-client")))
from genesis.client import GenesisMultiClient
from genesis.encoders import PopulationEncoder
from genesis.decoders import PwmDecoder
from genesis.control import GenesisControl
from genesis.memory import GenesisMemory
from genesis.brain import fnv1a_32
from genesis.contract import GenesisIoContract

# ==========================================
# 1. Глобальная инициализация (Zero-Downtime)
# ==========================================
# [DOD FIX] Синхронизировано с build_brain.py (10 тиков = 80 байт C-ABI)
BATCH_SIZE = 10

# [DOD FIX] Синхронизация путей с актуальной моделью
baked_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../Genesis-Models/cartpole_exp/baked/MotorCortex"))
manifest_path = os.path.join(baked_dir, "manifest.toml")

if not os.path.exists(manifest_path):
    print(f"❌ FATAL: Control Plane manifest NOT FOUND at {manifest_path}")
    sys.exit(1)

contract = GenesisIoContract(baked_dir, "MotorCortex")
zone_hash = contract.zone_hash

print("🔌 Connecting to Genesis Node (Data & Memory Planes)...")
# [DOD FIX] Используем контракт для конфигурации клиента (включая RX Layout)
client_cfg = contract.get_client_config(BATCH_SIZE)
client = GenesisMultiClient(addr=("127.0.0.1", 8081), **client_cfg)

try:
    client.sock.bind(("0.0.0.0", 8092))
except OSError as e:
    print(f"❌ FATAL: Port 8092 is busy! Kill zombie agents before running. Error: {e}")
    sys.exit(1)

# [DOD FIX] Автоматическое создание энкодеров/декодеров из контракта
encoder = contract.create_population_encoder("sensors", vars_count=4, batch_size=BATCH_SIZE, sigma=0.2)
# [DOD FIX] Подключаем оба физических полушария для честной оценки Optuna
dec_left = contract.create_pwm_decoder("motor_left", batch_size=BATCH_SIZE)
dec_right = contract.create_pwm_decoder("motor_right", batch_size=BATCH_SIZE)

out_l_sz = contract.outputs["motor_left"]["width"] * contract.outputs["motor_left"]["height"] * BATCH_SIZE
out_r_sz = contract.outputs["motor_right"]["width"] * contract.outputs["motor_right"]["height"] * BATCH_SIZE

control = GenesisControl(manifest_path)
memory = GenesisMemory(zone_hash, read_only=False)

bounds = np.array([[-2.4, 2.4], [-3.0, 3.0], [-0.41, 0.41], [-2.0, 2.0]], dtype=np.float16)
range_diff = bounds[:, 1] - bounds[:, 0]

# ==========================================
# 2. Optuna Objective
# ==========================================
def objective(trial):
    # 1. Сэмплируем гиперпараметры R-STDP (Полный спектр)
    dopamine_pulse = trial.suggest_int("dopamine_pulse", -255, -1)
    dopamine_reward = trial.suggest_int("dopamine_reward", 1, 255)
    prune_threshold = trial.suggest_int("prune_threshold", 2, 30)
    d1_affinity = trial.suggest_int("d1_affinity", 64, 255)
    d2_affinity = trial.suggest_int("d2_affinity", 64, 255)

    # Физика мембран (GLIF)
    # leak_rate: как быстро нейрон забывает входящий потенциал
    leak_rate = trial.suggest_int("leak_rate", 100, 3000)
    # homeostasis_penalty: насколько сильно нейрон "устает" после спайка
    homeostasis_penalty = trial.suggest_int("homeostasis_penalty", 500, 10000)
    # homeostasis_decay: как быстро нейрон "отдыхает"
    homeostasis_decay = trial.suggest_int("homeostasis_decay", 1, 100)

    # 2. Hot-Patching Control Plane
    control.set_prune_threshold(prune_threshold)
    control.set_night_interval(2000) # Жесткая фиксация частоты сна для быстрых мутаций
    
    # Применяем рецепторы к типам (0: Excitatory, 1: Inhibitory)
    control.set_dopamine_receptors(0, d1_affinity, d2_affinity)
    control.set_dopamine_receptors(1, d1_affinity, d2_affinity)
    
    # Применяем физику мембран (Hot-Patching VRAM LUT)
    control.set_membrane_physics(0, leak_rate, homeostasis_penalty, homeostasis_decay)
    control.set_membrane_physics(1, int(leak_rate * 1.5), int(homeostasis_penalty * 0.8), homeostasis_decay)

    # 3. Tabula Rasa (Хирургическое стирание VRAM)
    memory.clear_weights()
    # [DOD FIX] Жесткое обнуление электрического состояния и гомеостаза!
    memory.voltage.fill(0)
    memory.flags.fill(0)
    memory.threshold_offset.fill(0)
    memory.timers.fill(0)

    # [DOD FIX] Оставляем оригинальную физику среды (tau = 0.02s = 20 мс)
    env = gym.make("CartPole-v1").unwrapped
    state, _ = env.reset()
    norm_state = np.zeros(4, dtype=np.float16)
    
    # [DOD FIX] Fail-Fast Survival Metrics (30 секунд удержания = Абсолютный успех)
    global_steps = 0
    max_score = 0
    MAX_STEPS = 1500
    score = 0
    terminated, truncated = False, False

    # Гоняем сеть, пока она не умрет от эпилепсии или не упрется в лимит времени
    while global_steps < MAX_STEPS:

        if terminated or truncated:
            max_score = max(max_score, score)
            
            # Жестко транслируем кадр ошибки в VRAM
            encoder.encode_into(norm_state, client.payload_views[0])
            
            # Болевой шок на 20 батчей (20 мс биологического времени)
            for _ in range(20):
                client.step(-255)

            stats = memory.get_network_stats()
            avg_w = stats["avg_weight"]
            synapses = stats["active_synapses"]

            # [DOD FIX] Graceful Fatality: Сеть сгорела. Фиксируем время жизни.
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

        # 1. Нормализация ошибки (0.0 = идеал, 1.0 = крах)
        angle_error = min(1.0, pole_angle / 0.2094)
        vel_error = min(1.0, pole_velocity / 2.0)

        # 2. Взвешенная ошибка (удерживаем фокус на угле, но гасим раскачку)
        error = min(1.0, angle_error * 0.8 + vel_error * 0.2)

        # 3. Линейная алгебра дофамина (без if/else)
        dop_sig = int(dopamine_reward * (1.0 - error) + dopamine_pulse * error)

        # [DOD FIX] Разгоняем мозг до скорости физики (0.02s = 20 мс). 
        # BATCH_SIZE = 10 тиков (1 мс). Нам нужно 20 батчей.
        force_left = 0.0
        force_right = 0.0
        
        encoder.encode_into(norm_state, client.payload_views[0])
        
        for _ in range(20):
            rx = client.step(dop_sig)
            rx_view = memoryview(rx)
            
            # [DOD FIX] Жесткий L7-демультиплексинг по двум полушариям
            motor_l = dec_left.decode_from(rx_view[0 : out_l_sz])
            motor_r = dec_right.decode_from(rx_view[out_l_sz : out_l_sz + out_r_sz])
            
            force_left += np.sum(motor_l)
            force_right += np.sum(motor_r)
        
        action = 0 if force_left > force_right else 1

        state, reward, terminated, truncated, _ = env.step(action)
        score += 1
        global_steps += 1

    env.close()
    
    # Сохраняем побочную аналитику для дашборда
    max_score = max(max_score, score)
    trial.set_user_attr("max_score", max_score)
    
    # [DOD FIX] Composite Metric: Выживаемость + Навык (с жестким приоритетом навыка для бессмертных)
    return float(global_steps + (max_score * 1000))

if __name__ == '__main__':
    # Снижаем шум от логов Optuna
    optuna.logging.set_verbosity(optuna.logging.WARNING) # [DOD FIX] Глушим дефолтный мусор

    # [DOD FIX] Zero-Cost Telemetry Hook
    def hft_telemetry_callback(study, trial):
        max_score = trial.user_attrs.get("max_score", 0)
        print(f"🏁 Trial {trial.number:03d} | Survived: {trial.value:5.0f} steps | MAX SCORE: {max_score:5.0f} | "
              f"Pulse: {trial.params.get('dopamine_pulse')}, Reward: {trial.params.get('dopamine_reward')}")

    study = optuna.create_study(direction="maximize", pruner=optuna.pruners.MedianPruner())
    print("🚀 Starting Zero-Downtime Bayesian Search...")
    
    try:
        # Гоняем 200 триалов
        study.optimize(objective, n_trials=200, callbacks=[hft_telemetry_callback])
        print("\n🏆 Best Hyperparameters:")
        for key, value in study.best_trial.params.items():
            print(f"  {key}: {value}")
        print(f"🏅 Best Score: {study.best_value:.1f}")
    except KeyboardInterrupt:
        print("\nSearch interrupted by user.")