#========================================================
#       ВЕДУТСЯ РАБОТЫ С SDK И ВНЕДРЕНИЕМ R-STDP
#========================================================

#!/usr/bin/env python3
import os
import sys
import time
import numpy as np
import re
import importlib.util
from pathlib import Path

# Проверка активации виртуального окружения
if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("❌ ERROR: Virtual environment not active!")
    sys.exit(1)

# Добавляем путь к SDK ( genesis-client/ ) если скрипт запущен напрямую из примера
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "genesis-client")))

from genesis.client import GenesisMultiClient
from genesis.contract import GenesisIoContract
from genesis.memory import GenesisMemory

#============================================================
#       CLIENT & ENVIRONMENT SETTINGS
#============================================================
BATCH_SIZE = 20                 # HFT-цикл: 1 пакет = 20 тиков (Должно быть равно tick_duration_us в build_brain.py)
ENCODER_SIGMA = 0.2             # Сигма энкодера (разброс признаков)

#============================================================
#               АВАТАР МУХИ (FLOAT ПЕРЕМЕННЫЕ)
#============================================================
class FlyAvatarState:
    """Группировка всех float-переменных состояния аватара для удобства"""
    def __init__(self):
        # --- Глобальная позиция и ориентация (Голова/Тело) ---
        self.pos_x: float = 0.0
        self.pos_y: float = 0.0
        self.pos_z: float = 0.0
        self.roll: float = 0.0
        self.pitch: float = 0.0
        self.yaw: float = 0.0
        
        # --- Линейные и угловые скорости тела ---
        self.vel_x: float = 0.0
        self.vel_y: float = 0.0
        self.vel_z: float = 0.0
        self.ang_vel_roll: float = 0.0
        self.ang_vel_pitch: float = 0.0
        self.ang_vel_yaw: float = 0.0

        # --- Состояние суставов (УГЛЫ ЛАП: LF, LM, LH, RF, RM, RH) ---
        # Левая передняя (Left Front)
        self.angle_LF_coxa: float = 0.0
        self.angle_LF_femur: float = 0.0
        self.angle_LF_tibia: float = 0.0
        self.angle_LF_tarsus: float = 0.0

        # Левая средняя (Left Middle)
        self.angle_LM_coxa: float = 0.0
        self.angle_LM_femur: float = 0.0
        self.angle_LM_tibia: float = 0.0
        self.angle_LM_tarsus: float = 0.0

        # Левая задняя (Left Hind)
        self.angle_LH_coxa: float = 0.0
        self.angle_LH_femur: float = 0.0
        self.angle_LH_tibia: float = 0.0
        self.angle_LH_tarsus: float = 0.0

        # Правая передняя (Right Front)
        self.angle_RF_coxa: float = 0.0
        self.angle_RF_femur: float = 0.0
        self.angle_RF_tibia: float = 0.0
        self.angle_RF_tarsus: float = 0.0

        # Правая средняя (Right Middle)
        self.angle_RM_coxa: float = 0.0
        self.angle_RM_femur: float = 0.0
        self.angle_RM_tibia: float = 0.0
        self.angle_RM_tarsus: float = 0.0

        # Правая задняя (Right Hind)
        self.angle_RH_coxa: float = 0.0
        self.angle_RH_femur: float = 0.0
        self.angle_RH_tibia: float = 0.0
        self.angle_RH_tarsus: float = 0.0

        # --- Состояние суставов (УГЛЫ КРЫЛЬЕВ) ---
        # Левое (Left) и Правое (Right) крыло (углы маха/вращения)
        self.angle_L_wing_roll: float = 0.0
        self.angle_L_wing_pitch: float = 0.0
        self.angle_L_wing_yaw: float = 0.0
        
        self.angle_R_wing_roll: float = 0.0
        self.angle_R_wing_pitch: float = 0.0
        self.angle_R_wing_yaw: float = 0.0

        # --- Скорости суставов (опционально, объединил) ---
        # (Если понадобятся индивидуальные скорости каждого сустава лап — можно расписать по аналогии)
        self.legs_joint_velocities: float = 0.0  
        self.wings_joint_velocities: float = 0.0 

        # --- Силы и Контакты (End Effectors & Sensors) ---
        # Силы реакции опоры на каждую из 6 лап
        self.contact_force_LF: float = 0.0
        self.contact_force_LM: float = 0.0
        self.contact_force_LH: float = 0.0
        self.contact_force_RF: float = 0.0
        self.contact_force_RM: float = 0.0
        self.contact_force_RH: float = 0.0
        
        # Сенсоры внешней среды
        self.odor_intensity: float = 0.0         # Интенсивность запаха (обоняние)
        self.vision_features: float = 0.0        # Визуальные признаки (зрение 32x32)

def sanitize_flygym_xmls():
    """
    [DOD] Self-Healing механизм. 
    Хирургически вырезает атрибуты, которые крашат парсер dm_control в новых версиях Python.
    """
    spec = importlib.util.find_spec("flygym")
    if not spec or not spec.submodule_search_locations:
        return
    
    base_path = Path(spec.submodule_search_locations[0]) / "data"
    if not base_path.exists():
        return

    # Список атрибутов, вызывающих AttributeError в новых версиях MuJoCo
    problematic_attrs = ["convexhull", "mpr_iterations", "collision"]

    for xml_file in base_path.rglob("*.xml"):
        try:
            content = xml_file.read_text(encoding="utf-8")
            modified = False
            
            for attr in problematic_attrs:
                if f'{attr}=' in content:
                    # Выжигаем атрибут регулярным выражением
                    content = re.sub(fr'\s*{attr}="[^"]*"', '', content)
                    modified = True
            
            if modified:
                xml_file.write_text(content, encoding="utf-8")
                print(f"🔧 [Self-Healing] Patched XML ({'+'.join(problematic_attrs)}): {xml_file.name}")
        except Exception as e:
            print(f"⚠️ [Self-Healing] Failed to patch {xml_file.name}: {e}")

def run_fly():
    # ============================================================
    # 1. Multi-Port Binding (Zero-Copy Contracts)
    # ============================================================
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../Genesis-Models/FLY_exp/baked"))

    # Загружаем 4 контракта
    c_cx = GenesisIoContract(os.path.join(base_dir, "CX"), "CX")
    c_an = GenesisIoContract(os.path.join(base_dir, "AN"), "AN")
    c_vp = GenesisIoContract(os.path.join(base_dir, "VP"), "VP")
    c_desc = GenesisIoContract(os.path.join(base_dir, "DESCENDING"), "DESCENDING")

    # Собираем единый массив матриц для мультиплексора UDP
    # Порядок сборки критичен: он определяет индексы client.payload_views
    matrices = (
        c_cx.get_client_config(BATCH_SIZE)["matrices"] +       # view
        c_an.get_client_config(BATCH_SIZE)["matrices"] +       # view[1]
        c_vp.get_client_config(BATCH_SIZE)["matrices"] +       # view[2]
        c_desc.get_client_config(BATCH_SIZE)["matrices"]       # view[3]
    )
    rx_layout = c_desc.get_client_config(BATCH_SIZE)["rx_layout"]

    client = GenesisMultiClient(
        addr=("127.0.0.1", 8081),
        matrices=matrices,
        rx_layout=rx_layout,
        timeout=0.5
    )

    try:
        client.sock.bind(("0.0.0.0", 8092))
    except OSError as e:
        print(f"❌ FATAL: Port 8092 is busy! Kill zombie agents. Error: {e}")
        sys.exit(1)

    # ============================================================
    # 2. Фабрика DOD Энкодеров и Декодеров
    # ============================================================
    enc_nav = c_cx.create_population_encoder("navigation", vars_count=15, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)
    enc_halt = c_an.create_population_encoder("haltere", vars_count=10, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)
    enc_prop = c_vp.create_population_encoder("proprioception", vars_count=42, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)
    enc_refl = c_desc.create_population_encoder("reflexes", vars_count=6, batch_size=BATCH_SIZE, sigma=ENCODER_SIGMA)

    dec_mot = c_desc.create_pwm_decoder("motors", batch_size=BATCH_SIZE)

    # ============================================================
    # 3. Среда и Преаллокация Памяти
    # ============================================================
    print("🩹 Запуск препроцессора ассетов...")
    sanitize_flygym_xmls()

    from flygym.mujoco import NeuroMechFly
    import mujoco.viewer

    print("🪰 Инициализация NeuroMechFly (FlyGym)...")
    env = NeuroMechFly()
    state, _ = env.reset()
    viewer = mujoco.viewer.launch_passive(env.physics.model.ptr, env.physics.data.ptr)

    # ============================================================
    # БЛОК ЯВНОГО ВВОДА/ВЫВОДА (MULTI-PORT FACADES)
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
    bounds_prop[0:42] = [-3.15, 3.15] # Углы суставов
    rd_prop = bounds_prop[:, 1] - bounds_prop[:, 0]
    rd_prop[rd_prop == 0] = 1.0

    # 4. Reflexes (DESCENDING: 16 slots)
    buf_refl = np.zeros(16, dtype=np.float16)
    bounds_refl = np.zeros((16, 2), dtype=np.float16)
    bounds_refl[0:6] = [0.0, 100.0] # Силы контакта
    rd_refl = bounds_refl[:, 1] - bounds_refl[:, 0]
    rd_refl[rd_refl == 0] = 1.0

    # Выходной фасад
    avatar_out = c_desc.create_output_facade("motors", dec_mot._out_buffer)
    action_buffer = np.zeros(42, dtype=np.float32)

    episodes = 0
    print(f"🚀 Starting Genesis DOD FLY Loop (Lockstep BATCH_SIZE={BATCH_SIZE})...")

    # ============================================================
    # 4. HFT Hot Loop (Explicit Routing)
    # ============================================================
    while True:
        # ===================================================================
        # СЕНСОРЫ (Zero-Cost Bulk Copy & SIMD Compute)
        # ===================================================================
        # 1. Navigation
        buf_nav[0:12] = state["fly"].flatten()
        buf_nav[12:15] = state["fly_orientation"]

        # 2. Haltere (Пока заглушка, 0.0)
        
        # 3. Proprioception (Только углы суставов - строка 0 матрицы 3x42)
        buf_prop[0:42] = state["joints"][0, :]

        # 4. Reflexes (Сжатие 30 точек контакта на 6 лап без циклов)
        # state["contact_forces"] имеет форму (30, 3) - по 5 сенсоров на 6 лап
        # Векторизованно: находим норму вектора, решейпим, суммируем по сенсорам
        forces = np.linalg.norm(state["contact_forces"].reshape(6, 5, 3), axis=2).sum(axis=1)
        buf_refl[0:6] = forces

        # Нормализация
        norm_nav = np.clip((buf_nav - bounds_nav[:, 0]) / rd_nav, 0.0, 1.0)
        norm_halt = np.clip((buf_halt - bounds_halt[:, 0]) / rd_halt, 0.0, 1.0)
        norm_prop = np.clip((buf_prop - bounds_prop[:, 0]) / rd_prop, 0.0, 1.0)
        norm_refl = np.clip((buf_refl - bounds_refl[:, 0]) / rd_refl, 0.0, 1.0)

        # ===================================================================
        # ТРАНСПОРТ В VRAM (4 параллельных канала)
        # ===================================================================
        # [DOD FIX] Передаем энкодеру строгий O(1) срез активных переменных (без паддинга),
        # чтобы NumPy смог сделать Zero-Cost Broadcasting.
        enc_nav.encode_into(norm_nav[:15], client.payload_views[0])
        enc_halt.encode_into(norm_halt[:10], client.payload_views[1])
        enc_prop.encode_into(norm_prop[:42], client.payload_views[2])
        enc_refl.encode_into(norm_refl[:6],  client.payload_views[3])

        rx = client.step(0)  # Барьер и Дофамин

        # ===================================================================
        # МОТОРЫ
        # ===================================================================
        dec_mot.decode_from(rx)
        
        MOTOR_GAIN = 0.2
        # Блочное O(1) копирование 42 декодированных сигналов
        action_buffer[:] = (avatar_out.raw_buffer[:42] - 0.5) * 2.0 * MOTOR_GAIN
        action = {"joints": action_buffer}

        # ===================================================================
        #                           ФИЗИКА
        # ===================================================================
        try:
            step_result = env.step(action)
            if len(step_result) == 5:
                state, reward, terminated, truncated, _ = step_result
            else:
                state, reward, terminated, info = step_result
                truncated = False
        except Exception as e:
            print(f"💥 Simulation Exploded: {e}")
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