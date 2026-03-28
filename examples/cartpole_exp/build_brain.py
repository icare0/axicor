#!/usr/bin/env python3
import os
import sys

if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("❌ ERROR: Virtual environment not active!")
    print("Please run: source .venv/bin/activate")
    sys.exit(1)

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../../genesis-client")))
from genesis.builder import BrainBuilder

def build_cartpole_brain():
    print("🧠 Инициализация DOD-архитектора CartPole...")
    base_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    gnm_path = os.path.join(base_path, "GNM-Library")
    out_dir = os.path.join(base_path, "Genesis-Models/cartpole_exp")

    builder = BrainBuilder(project_name="cartpole_exp", output_dir=out_dir, gnm_lib_path=gnm_path)

    # =========================================================================
    # ФИЗИКА И ТАЙМИНГИ (The 10ms Rule)
    # =========================================================================
    builder.sim_params["sync_batch_ticks"] = 10  # 1 ms latency control
    builder.sim_params["tick_duration_us"] = 100
    builder.sim_params["signal_speed_m_s"] = 0.5
    builder.sim_params["segment_length_voxels"] = 2
    builder.sim_params["voxel_size_um"] = 25.0

    # =========================================================================
    # БЛЮПРИНТЫ (Без ручных мутаций)
    # =========================================================================
    exc_type = builder.gnm_lib("L5_spiny_MTG_13")
    inh_type = builder.gnm_lib("L5_aspiny_MTG_10")

    # DOD FIX: Изолируем моторный тип, чтобы GXO-маппер не зацепил сенсорный слой L1
    motor_type = builder.gnm_lib("L5_spiny_MTG_13")
    motor_type.name = "Motor_Pyramidal"
    for d in motor_type.data_list:
        d["name"] = "Motor_Pyramidal"

    # =========================================================================
    # ГЕОМЕТРИЯ (Единый Sensorimotor Cortex)
    # =========================================================================
    cortex = builder.add_zone("MotorCortex", width_vox=32, depth_vox=32, height_vox=32)

    cortex.add_layer("L1_Input", height_pct=0.2, density=0.05) \
          .add_population(exc_type, fraction=0.8) \
          .add_population(inh_type, fraction=0.2)

    cortex.add_layer("L4_Hidden", height_pct=0.6, density=0.1) \
          .add_population(exc_type, fraction=0.7) \
          .add_population(inh_type, fraction=0.3)

    cortex.add_layer("L5_Output", height_pct=0.2, density=0.05) \
          .add_population(motor_type, fraction=0.8) \
          .add_population(inh_type, fraction=0.2)

    # =========================================================================
    # I/O МАТРИЦЫ (Zero-Cost Hardware Routing)
    # =========================================================================
    layout_sensors = ["cart_x", "cart_v", "pole_a", "pole_av"]

    # Входной порт
    cortex.add_input("sensors", width=16, height=16, entry_z="top", \
        layout=layout_sensors)

    # Выходные порты: Физически разрезаем моторную кору на два независимых полушария.
    # Левый порт читает только левую половину (U от 0.0 до 0.5)
    cortex.add_output("motor_left", width=16, height=16, target_type="Motor_Pyramidal", uv_rect=[0.0, 0.0, 0.5, 1.0])
    cortex.add_output("motor_right", width=16, height=16, target_type="Motor_Pyramidal", uv_rect=[0.5, 0.0, 0.5, 1.0])

    # Компиляция TOML конфигов и запекание бинарных дампов VRAM
    builder.build().bake(clean=True)

if __name__ == '__main__':
    build_cartpole_brain()