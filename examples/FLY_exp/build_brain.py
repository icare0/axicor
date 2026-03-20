#===================================================================================
#                                   НЕЗАВЕРШЕНО
#===================================================================================

#!/usr/bin/env python3
import os
import sys

# Проверка активации виртуального окружения
if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("❌ ERROR: Virtual environment not active!")
    print("Please run: source .venv/bin/activate")
    sys.exit(1)
import subprocess

# Добавляем путь к SDK
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../../genesis-client")))
from genesis.builder import BrainBuilder

def build_FLY_exp_brain():
    print("🧠 Инициализация архитектора коннектома FLY_exp...")
    
    gnm_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../GNM-Library"))
    out_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../Genesis-Models/FLY_exp"))

    builder = BrainBuilder(project_name="FLY_exp", output_dir=out_dir, gnm_lib_path=gnm_path)

    builder.sim_params["sync_batch_ticks"] = 20
    builder.sim_params["tick_duration_us"] = 100
    builder.sim_params["signal_speed_m_s"] = 0.5
    builder.sim_params["segment_length_voxels"] = 5
    builder.sim_params["voxel_size_um"] = 3

#===================================================================================
#                                   ЗОНЫ И СЛОИ
#===================================================================================
    # --- Блюпринты из папки Drosophila ---
    b_ach = builder.gnm_lib("Drosophila_Exc_ACh")
    b_ach_motor = builder.gnm_lib("Drosophila_Exc_ACh_Motor") # 0.5% по Hemibrain
    b_ach_desc  = builder.gnm_lib("Drosophila_Exc_ACh_Desc")  # 5.3% по Hemibrain
    b_glu = builder.gnm_lib("Drosophila_Inh_Glu")
    b_gaba = builder.gnm_lib("Drosophila_Inh_GABA")
    b_da = builder.gnm_lib("Drosophila_Mod_DA")
    b_kc = builder.gnm_lib("Drosophila_Kenyon")

#===================================================================================

    # 1. Центральная зона (Ядро, Хаб, Память)
    CENTRAL = builder.add_zone("CENTRAL", width_vox=51, depth_vox=51, height_vox=10)
    # Слой памяти (Грибовидные тела сверху, 30% высоты)
    CENTRAL.add_layer("MB_Layer", height_pct=0.3, density=0.05)\
        .add_population(b_kc, 0.8)\
        .add_population(b_da, 0.2)
    # Основное месиво проводов (Superior/Inferior, 70% высоты)
    CENTRAL.add_layer("Deep_Layer", height_pct=0.7, density=0.08)\
        .add_population(b_ach_motor, 0.005)\
        .add_population(b_ach_desc, 0.053)\
        .add_population(b_ach, 0.492)\
        .add_population(b_glu, 0.25)\
        .add_population(b_gaba, 0.20)

#-------------------------------------------------------

    # 2. Ascending neurons (Восходящие) | 21×21=441
    AN = builder.add_zone("AN", width_vox=21, depth_vox=21, height_vox=8)
    AN.add_layer("Main", height_pct=1.0, density=0.05)\
        .add_population(b_ach, 0.70)\
        .add_population(b_gaba, 0.15)\
        .add_population(b_glu, 0.10)\
        .add_population(b_da, 0.05)

#-------------------------------------------------------

    # 3. Descending (Motor) neurons | 21×21=441
    DESCENDING = builder.add_zone("DESCENDING", width_vox=21, depth_vox=21, height_vox=8)
    DESCENDING.add_layer("Main", height_pct=1.0, density=0.05)\
        .add_population(b_ach, 0.70)\
        .add_population(b_glu, 0.15)\
        .add_population(b_gaba, 0.10)\
        .add_population(b_da, 0.05)

#-------------------------------------------------------

    # 4. Visual Projection neurons (Зрение) | 51×51=2601
    VP = builder.add_zone("VP", width_vox=51, depth_vox=51, height_vox=16)
    VP.add_layer("Medulla", height_pct=0.6, density=0.05)\
        .add_population(b_ach, 0.85)\
        .add_population(b_glu, 0.10)\
        .add_population(b_gaba, 0.05)
    VP.add_layer("Lobula", height_pct=0.4, density=0.05)\
        .add_population(b_ach, 0.80)\
        .add_population(b_glu, 0.15)\
        .add_population(b_gaba, 0.05)

#-------------------------------------------------------

    # 5. Antenal Lobe Projection neurons | 19×19=361
    ALPN = builder.add_zone("ALPN", width_vox=19, depth_vox=19, height_vox=8)
    ALPN.add_layer("Glomeruli", height_pct=1.0, density=0.05)\
        .add_population(b_ach, 0.70)\
        .add_population(b_gaba, 0.30)

#-------------------------------------------------------

    # 6. Central Complex neurons (Компас) | 21×21=441
    CX = builder.add_zone("CX", width_vox=21, depth_vox=21, height_vox=8)
    CX.add_layer("Fan_Shaped", height_pct=0.5, density=0.05)\
        .add_population(b_ach, 0.65)\
        .add_population(b_glu, 0.25)\
        .add_population(b_gaba, 0.10)
    CX.add_layer("Ellipsoid", height_pct=0.5, density=0.05)\
        .add_population(b_ach, 0.60)\
        .add_population(b_gaba, 0.20)\
        .add_population(b_glu, 0.20)

#-------------------------------------------------------

    # 7. Lateral Horn neurons (Инстинкты) | 17×17=289
    LHLN = builder.add_zone("LHLN", width_vox=17, depth_vox=17, height_vox=10)
    LHLN.add_layer("Main", height_pct=1.0, density=0.05)\
        .add_population(b_glu, 0.65)\
        .add_population(b_gaba, 0.32)\
        .add_population(b_ach, 0.03)

#-------------------------------------------------------

    # 8. Antenal Lobe neurons (Локальные) | 16×16=256
    ALLN = builder.add_zone("ALLN", width_vox=16, depth_vox=16, height_vox=8)
    ALLN.add_layer("Local_Inh", height_pct=1.0, density=0.05)\
        .add_population(b_gaba, 0.35)\
        .add_population(b_glu, 0.35)\
        .add_population(b_ach, 0.15)\
        .add_population(b_da, 0.15)


#===================================================================================
#               ПРОВОДКА (Ghost Axons) МЕЖЗОНАЛЬНЫЕ СВЯЗИ
#===================================================================================

    # CENTRAL → (сверху вниз)
    builder.connect(CENTRAL, AN, out_matrix="to_AN", in_width=16, in_height=16, entry_z="top", growth_steps=63)
    builder.connect(CENTRAL, DESCENDING, out_matrix="to_DESCENDING", in_width=12, in_height=12, entry_z="top", growth_steps=63)
    builder.connect(CENTRAL, VP, out_matrix="to_VP", in_width=10, in_height=10, entry_z="top", growth_steps=135)
    builder.connect(CENTRAL, CX, out_matrix="to_CX", in_width=8, in_height=8, entry_z="top", growth_steps=63)
    builder.connect(CENTRAL, ALPN, out_matrix="to_ALPN", in_width=7, in_height=7, entry_z="top", growth_steps=63)
    builder.connect(CENTRAL, LHLN, out_matrix="to_LHLN", in_width=6, in_height=6, entry_z="top", growth_steps=81)
    builder.connect(CENTRAL, ALLN, out_matrix="to_ALLN", in_width=5, in_height=5, entry_z="top", growth_steps=63)

    # AN → (восходящие, всегда снизу)
    builder.connect(AN, CENTRAL, out_matrix="to_CENTRAL", in_width=17, in_height=17, entry_z="bottom", growth_steps=81)
    builder.connect(AN, DESCENDING, out_matrix="to_DESCENDING", in_width=5, in_height=5, entry_z="bottom", growth_steps=63)
    builder.connect(AN, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="bottom", growth_steps=135)
    builder.connect(AN, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(AN, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(AN, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=81)

    # DESCENDING → (обратные связи, снизу)
    builder.connect(DESCENDING, CENTRAL, out_matrix="to_CENTRAL", in_width=17, in_height=17, entry_z="bottom", growth_steps=81)
    builder.connect(DESCENDING, AN, out_matrix="to_AN", in_width=6, in_height=6, entry_z="bottom", growth_steps=63)
    builder.connect(DESCENDING, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="bottom", growth_steps=135)
    builder.connect(DESCENDING, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(DESCENDING, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(DESCENDING, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=81)
    builder.connect(DESCENDING, ALLN, out_matrix="to_ALLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)

    # VP → (зрение, сверху)
    builder.connect(VP, CENTRAL, out_matrix="to_CENTRAL", in_width=43, in_height=43, entry_z="top", growth_steps=81)
    builder.connect(VP, AN, out_matrix="to_AN", in_width=15, in_height=15, entry_z="top", growth_steps=63)
    builder.connect(VP, DESCENDING, out_matrix="to_DESCENDING", in_width=12, in_height=12, entry_z="top", growth_steps=63)
    builder.connect(VP, CX, out_matrix="to_CX", in_width=8, in_height=8, entry_z="top", growth_steps=63)
    builder.connect(VP, ALPN, out_matrix="to_ALPN", in_width=7, in_height=7, entry_z="top", growth_steps=63)
    builder.connect(VP, ALLN, out_matrix="to_ALLN", in_width=6, in_height=6, entry_z="top", growth_steps=63)
    builder.connect(VP, LHLN, out_matrix="to_LHLN", in_width=6, in_height=6, entry_z="top", growth_steps=81)

    # ALPN → (обоняние, снизу)
    builder.connect(ALPN, CENTRAL, out_matrix="to_CENTRAL", in_width=16, in_height=16, entry_z="bottom", growth_steps=81)
    builder.connect(ALPN, AN, out_matrix="to_AN", in_width=6, in_height=6, entry_z="bottom", growth_steps=63)
    builder.connect(ALPN, DESCENDING, out_matrix="to_DESCENDING", in_width=4, in_height=4, entry_z="top", growth_steps=63)
    builder.connect(ALPN, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="bottom", growth_steps=135)
    builder.connect(ALPN, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(ALPN, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=81)

    # CX → (компас, сверху)
    builder.connect(CX, CENTRAL, out_matrix="to_CENTRAL", in_width=18, in_height=18, entry_z="top", growth_steps=81)
    builder.connect(CX, AN, out_matrix="to_AN", in_width=6, in_height=6, entry_z="top", growth_steps=63)
    builder.connect(CX, DESCENDING, out_matrix="to_DESCENDING", in_width=5, in_height=5, entry_z="top", growth_steps=63)
    builder.connect(CX, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="top", growth_steps=135)
    builder.connect(CX, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="top", growth_steps=81)
    builder.connect(CX, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="top", growth_steps=63)

    # LHLN → (инстинкты)
    builder.connect(LHLN, CENTRAL, out_matrix="to_CENTRAL", in_width=14, in_height=14, entry_z="bottom", growth_steps=81)
    builder.connect(LHLN, AN, out_matrix="to_AN", in_width=5, in_height=5, entry_z="bottom", growth_steps=63)
    builder.connect(LHLN, DESCENDING, out_matrix="to_DESCENDING", in_width=4, in_height=4, entry_z="top", growth_steps=63)
    builder.connect(LHLN, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="top", growth_steps=135)
    builder.connect(LHLN, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(LHLN, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)

    # ALLN → (локальное обоняние, снизу)
    builder.connect(ALLN, CENTRAL, out_matrix="to_CENTRAL", in_width=13, in_height=13, entry_z="bottom", growth_steps=81)
    builder.connect(ALLN, AN, out_matrix="to_AN", in_width=5, in_height=5, entry_z="bottom", growth_steps=63)
    builder.connect(ALLN, DESCENDING, out_matrix="to_DESCENDING", in_width=4, in_height=4, entry_z="top", growth_steps=63)
    builder.connect(ALLN, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="bottom", growth_steps=135)
    builder.connect(ALLN, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(ALLN, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(ALLN, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=81)

    # Компиляция TOML конфигов
    builder.build()

#===================================================================================
#                                   Запекание
#===================================================================================

    print("\n🔥 Запускаем Genesis Baker (CPU Compiler)...")
    brain_toml_path = os.path.join(out_dir, "brain.toml")
    
    result = subprocess.run([
        "cargo", "run", "--release", "-p", "genesis-baker", "--bin", "baker", "--", 
        "--brain", brain_toml_path
    ])

    if result.returncode == 0:
        print("\n✅ Модель успешно запечена и готова к загрузке на GPU.")
    else:
        print("\n❌ Ошибка компиляции коннектома. Проверьте логи Rust-компилятора.")
        sys.exit(1)

#===================================================================================

if __name__ == '__main__':
    build_FLY_exp_brain()

