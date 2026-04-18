#========================================================
#       SDK AND R-STDP INTEGRATION IN PROGRESS
#========================================================


#!/usr/bin/env python3
import os
import sys

# Virtual environment activation check
if not (sys.prefix != sys.base_prefix or 'VIRTUAL_ENV' in os.environ):
    print("[ERROR] ERROR: Virtual environment not active!")
    print("Please run: source .venv/bin/activate")
    sys.exit(1)
import subprocess

# Add SDK path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), "../../axicor-client")))
from axicor.builder import BrainBuilder

def build_FLY_exp_brain():
    print(" Initializing FLY_exp connectome architect...")
    
    gnm_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../GNM-Library"))
    out_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../Axicor-Models/FLY_exp"))

    builder = BrainBuilder(project_name="FLY_exp", output_dir=out_dir, gnm_lib_path=gnm_path)

    # Physics setup
    builder.sim_params["sync_batch_ticks"] = 20
    builder.sim_params["tick_duration_us"] = 100
    builder.sim_params["signal_speed_m_s"] = 0.45
    builder.sim_params["segment_length_voxels"] = 5
    builder.sim_params["voxel_size_um"] = 3

#===================================================================================
#                                   ZONES AND LAYERS
#===================================================================================
# A fly's brain is structured such that all somas are clustered together, resembling a thin layer of roe 
# acting as a cortex, so we implement a dense layered structure
# but add a very sparse bottom layer where somas occupy only 1-2% of the volume
# effectively filling the remaining space with axons and dendrites
# all computations occur within this sparse volume
# while those 1-2% of somas act as relay stations, compensating for the limited
# number of short dendrites

    # --- Blueprints from Drosophila folder ---
    b_ach = builder.gnm_lib("Drosophila_Exc_ACh")
    b_ach_motor = builder.gnm_lib("Drosophila_Exc_ACh_Motor")
    b_ach_desc  = builder.gnm_lib("Drosophila_Exc_ACh_Desc")
    b_glu = builder.gnm_lib("Drosophila_Inh_Glu")
    b_gaba = builder.gnm_lib("Drosophila_Inh_GABA")
    b_da = builder.gnm_lib("Drosophila_Mod_DA")
    b_kc = builder.gnm_lib("Drosophila_Kenyon")
    retranslator_inh = builder.gnm_lib("L23_aspiny_VISp23_5")
    retranslator_exc = builder.gnm_lib("L23_spiny_VISp23_7")

#===================================================================================

    # 1. Central Zone (Core, Hub, Memory)
    CENTRAL = builder.add_zone("CENTRAL", width_vox=71, depth_vox=71, height_vox=40)
    # Memory layer (Mushroom bodies on top, 10% height)
    CENTRAL.add_layer("MB_Layer", height_pct=0.1, density=0.2)\
        .add_population(b_kc, 0.8)\
        .add_population(b_da, 0.2)
    # Main wire mesh (Superior/Inferior, 30% height)
    CENTRAL.add_layer("Deep_Layer", height_pct=0.3, density=0.2)\
        .add_population(b_ach_motor, 0.005)\
        .add_population(b_ach_desc, 0.053)\
        .add_population(b_ach, 0.492)\
        .add_population(b_glu, 0.25)\
        .add_population(b_gaba, 0.20)
    # Relay layer (60% height)
    CENTRAL.add_layer("Retranslator_Layer", height_pct=0.6, density=0.02)\
        .add_population(retranslator_inh, 0.5)\
        .add_population(retranslator_exc, 0.5)
    # Outputs from this zone to other zones
    CENTRAL.add_output("to_AN", width=16, height=16)
    CENTRAL.add_output("to_DESCENDING", width=12, height=12)
    CENTRAL.add_output("to_VP", width=10, height=10)
    CENTRAL.add_output("to_CX", width=8, height=8)
    CENTRAL.add_output("to_ALPN", width=7, height=7)
    CENTRAL.add_output("to_LHLN", width=6, height=6)
    CENTRAL.add_output("to_ALLN", width=5, height=5)

#-------------------------------------------------------

    # 2. Ascending neurons | 2121=441
    AN = builder.add_zone("AN", width_vox=41, depth_vox=41, height_vox=40)
    AN.add_layer("Main", height_pct=0.4, density=0.2)\
        .add_population(b_ach, 0.70)\
        .add_population(b_gaba, 0.15)\
        .add_population(b_glu, 0.10)\
        .add_population(b_da, 0.05)
    # Relay layer (60% height)
    AN.add_layer("Retranslator_Layer", height_pct=0.6, density=0.02)\
        .add_population(retranslator_inh, 0.5)\
        .add_population(retranslator_exc, 0.5)
        # Outputs from this zone to other zones
    AN.add_output("to_CENTRAL", width=17, height=17)
    AN.add_output("to_DESCENDING", width=5, height=5)
    AN.add_output("to_VP", width=4, height=4)
    AN.add_output("to_CX", width=4, height=4)
    AN.add_output("to_ALPN", width=4, height=4)
    AN.add_output("to_LHLN", width=4, height=4)
    
#-------------------------------------------------------

    # 3. Descending (Motor) neurons | 2121=441
    DESCENDING = builder.add_zone("DESCENDING", width_vox=51, depth_vox=51, height_vox=40)
    DESCENDING.add_layer("Main", height_pct=0.4, density=0.2)\
        .add_population(b_ach, 0.70)\
        .add_population(b_glu, 0.15)\
        .add_population(b_gaba, 0.10)\
        .add_population(b_da, 0.05)
    # Relay layer (60% height)
    DESCENDING.add_layer("Retranslator_Layer", height_pct=0.6, density=0.02)\
        .add_population(retranslator_inh, 0.5)\
        .add_population(retranslator_exc, 0.5)
    # Outputs from this zone to other zones
    DESCENDING.add_output("to_CENTRAL", width=17, height=17)
    DESCENDING.add_output("to_AN", width=6, height=6)
    DESCENDING.add_output("to_VP", width=4, height=4)
    DESCENDING.add_output("to_CX", width=4, height=4)
    DESCENDING.add_output("to_ALPN", width=4, height=4)
    DESCENDING.add_output("to_LHLN", width=4, height=4)
    DESCENDING.add_output("to_ALLN", width=4, height=4)

#-------------------------------------------------------

    # 4. Visual Projection neurons (Vision) | 5151=2601
    VP = builder.add_zone("VP", width_vox=81, depth_vox=81, height_vox=40)
    VP.add_layer("Medulla", height_pct=0.2, density=0.2)\
        .add_population(b_ach, 0.85)\
        .add_population(b_glu, 0.10)\
        .add_population(b_gaba, 0.05)
    VP.add_layer("Lobula", height_pct=0.2, density=0.2)\
        .add_population(b_ach, 0.80)\
        .add_population(b_glu, 0.15)\
        .add_population(b_gaba, 0.05)
    # Relay layer (60% height)
    VP.add_layer("Retranslator_Layer", height_pct=0.6, density=0.02)\
        .add_population(retranslator_inh, 0.5)\
        .add_population(retranslator_exc, 0.5)
    # Outputs from this zone to other zones
    VP.add_output("to_CENTRAL", width=43, height=43)
    VP.add_output("to_AN", width=15, height=15)
    VP.add_output("to_DESCENDING", width=12, height=12)
    VP.add_output("to_CX", width=8, height=8)
    VP.add_output("to_ALPN", width=7, height=7)
    VP.add_output("to_ALLN", width=6, height=6)
    VP.add_output("to_LHLN", width=6, height=6)

#-------------------------------------------------------

    # 5. Antenal Lobe Projection neurons | 1919=361
    ALPN = builder.add_zone("ALPN", width_vox=19, depth_vox=19, height_vox=30)
    ALPN.add_layer("Glomeruli", height_pct=0.4, density=0.2)\
        .add_population(b_ach, 0.70)\
        .add_population(b_gaba, 0.30)
    # Relay layer (60% height)
    ALPN.add_layer("Retranslator_Layer", height_pct=0.6, density=0.02)\
        .add_population(retranslator_inh, 0.5)\
        .add_population(retranslator_exc, 0.5)
    # Outputs from this zone to other zones
    ALPN.add_output("to_CENTRAL", width=16, height=16)
    ALPN.add_output("to_AN", width=6, height=6)
    ALPN.add_output("to_DESCENDING", width=4, height=4)
    ALPN.add_output("to_VP", width=4, height=4)
    ALPN.add_output("to_CX", width=4, height=4)
    ALPN.add_output("to_LHLN", width=4, height=4)

#-------------------------------------------------------

    # 6. Central Complex neurons (Compass) | 2121=441
    CX = builder.add_zone("CX", width_vox=41, depth_vox=41, height_vox=40)
    CX.add_layer("Fan_Shaped", height_pct=0.2, density=0.2)\
        .add_population(b_ach, 0.65)\
        .add_population(b_glu, 0.25)\
        .add_population(b_gaba, 0.10)
    CX.add_layer("Ellipsoid", height_pct=0.2, density=0.2)\
        .add_population(b_ach, 0.60)\
        .add_population(b_gaba, 0.20)\
        .add_population(b_glu, 0.20)
    # Relay layer (60% height)
    CX.add_layer("Retranslator_Layer", height_pct=0.6, density=0.02)\
        .add_population(retranslator_inh, 0.5)\
        .add_population(retranslator_exc, 0.5)
        # Outputs from this zone to other zones
    CX.add_output("to_CENTRAL", width=18, height=18)
    CX.add_output("to_AN", width=6, height=6)
    CX.add_output("to_DESCENDING", width=5, height=5)
    CX.add_output("to_VP", width=4, height=4)
    CX.add_output("to_LHLN", width=4, height=4)
    CX.add_output("to_ALPN", width=4, height=4)

#-------------------------------------------------------

    # 7. Lateral Horn neurons (Instincts) | 1717=289
    LHLN = builder.add_zone("LHLN", width_vox=17, depth_vox=17, height_vox=10)
    LHLN.add_layer("Main", height_pct=0.4, density=0.2)\
        .add_population(b_glu, 0.65)\
        .add_population(b_gaba, 0.32)\
        .add_population(b_ach, 0.03)
    # Relay layer (60% height)
    LHLN.add_layer("Retranslator_Layer", height_pct=0.6, density=0.02)\
        .add_population(retranslator_inh, 0.5)\
        .add_population(retranslator_exc, 0.5)
        # Outputs from this zone to other zones
    LHLN.add_output("to_CENTRAL", width=14, height=14)
    LHLN.add_output("to_AN", width=5, height=5)
    LHLN.add_output("to_DESCENDING", width=4, height=4)
    LHLN.add_output("to_VP", width=4, height=4)
    LHLN.add_output("to_CX", width=4, height=4)
    LHLN.add_output("to_ALPN", width=4, height=4)

#-------------------------------------------------------

    # 8. Antenal Lobe neurons (Local) | 1616=256
    ALLN = builder.add_zone("ALLN", width_vox=16, depth_vox=16, height_vox=8)
    ALLN.add_layer("Local_Inh", height_pct=0.4, density=0.2)\
        .add_population(b_gaba, 0.35)\
        .add_population(b_glu, 0.35)\
        .add_population(b_ach, 0.15)\
        .add_population(b_da, 0.15)
    # Relay layer (60% height)
    ALLN.add_layer("Retranslator_Layer", height_pct=0.6, density=0.02)\
        .add_population(retranslator_inh, 0.5)\
        .add_population(retranslator_exc, 0.5)
    # Outputs from this zone to other zones
    ALLN.add_output("to_CENTRAL", width=13, height=13)
    ALLN.add_output("to_AN", width=5, height=5)
    ALLN.add_output("to_DESCENDING", width=4, height=4)
    ALLN.add_output("to_VP", width=4, height=4)
    ALLN.add_output("to_CX", width=4, height=4)
    ALLN.add_output("to_ALPN", width=4, height=4)
    ALLN.add_output("to_LHLN", width=4, height=4)

#=================================================================================
#               WIRING (Ghost Axons) INTER-ZONE CONNECTIONS
#=================================================================================

    # CENTRAL  (from top to bottom)
    builder.connect(CENTRAL, AN, out_matrix="to_AN", in_width=16, in_height=16, entry_z="top", growth_steps=63)
    builder.connect(CENTRAL, DESCENDING, out_matrix="to_DESCENDING", in_width=12, in_height=12, entry_z="top", growth_steps=63)
    builder.connect(CENTRAL, VP, out_matrix="to_VP", in_width=10, in_height=10, entry_z="top", growth_steps=135)
    builder.connect(CENTRAL, CX, out_matrix="to_CX", in_width=8, in_height=8, entry_z="top", growth_steps=63)
    builder.connect(CENTRAL, ALPN, out_matrix="to_ALPN", in_width=7, in_height=7, entry_z="top", growth_steps=63)
    builder.connect(CENTRAL, LHLN, out_matrix="to_LHLN", in_width=6, in_height=6, entry_z="top", growth_steps=81)
    builder.connect(CENTRAL, ALLN, out_matrix="to_ALLN", in_width=5, in_height=5, entry_z="top", growth_steps=63)

    # AN  (ascending, always from bottom)
    builder.connect(AN, CENTRAL, out_matrix="to_CENTRAL", in_width=17, in_height=17, entry_z="bottom", growth_steps=81)
    builder.connect(AN, DESCENDING, out_matrix="to_DESCENDING", in_width=5, in_height=5, entry_z="bottom", growth_steps=63)
    builder.connect(AN, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="bottom", growth_steps=135)
    builder.connect(AN, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(AN, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(AN, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=81)
    

    # DESCENDING  (feedback loops, from bottom)
    builder.connect(DESCENDING, CENTRAL, out_matrix="to_CENTRAL", in_width=17, in_height=17, entry_z="bottom", growth_steps=81)
    builder.connect(DESCENDING, AN, out_matrix="to_AN", in_width=6, in_height=6, entry_z="bottom", growth_steps=63)
    builder.connect(DESCENDING, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="bottom", growth_steps=135)
    builder.connect(DESCENDING, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(DESCENDING, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(DESCENDING, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=81)
    builder.connect(DESCENDING, ALLN, out_matrix="to_ALLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)

    # VP  (vision, from top)
    builder.connect(VP, CENTRAL, out_matrix="to_CENTRAL", in_width=43, in_height=43, entry_z="top", growth_steps=81)
    builder.connect(VP, AN, out_matrix="to_AN", in_width=15, in_height=15, entry_z="top", growth_steps=63)
    builder.connect(VP, DESCENDING, out_matrix="to_DESCENDING", in_width=12, in_height=12, entry_z="top", growth_steps=63)
    builder.connect(VP, CX, out_matrix="to_CX", in_width=8, in_height=8, entry_z="top", growth_steps=63)
    builder.connect(VP, ALPN, out_matrix="to_ALPN", in_width=7, in_height=7, entry_z="top", growth_steps=63)
    builder.connect(VP, ALLN, out_matrix="to_ALLN", in_width=6, in_height=6, entry_z="top", growth_steps=63)
    builder.connect(VP, LHLN, out_matrix="to_LHLN", in_width=6, in_height=6, entry_z="top", growth_steps=81)

    # ALPN  (olfaction, from bottom)
    builder.connect(ALPN, CENTRAL, out_matrix="to_CENTRAL", in_width=16, in_height=16, entry_z="bottom", growth_steps=81)
    builder.connect(ALPN, AN, out_matrix="to_AN", in_width=6, in_height=6, entry_z="bottom", growth_steps=63)
    builder.connect(ALPN, DESCENDING, out_matrix="to_DESCENDING", in_width=4, in_height=4, entry_z="top", growth_steps=63)
    builder.connect(ALPN, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="bottom", growth_steps=135)
    builder.connect(ALPN, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(ALPN, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=81)
    

    # CX  (compass, from top)
    builder.connect(CX, CENTRAL, out_matrix="to_CENTRAL", in_width=18, in_height=18, entry_z="top", growth_steps=81)
    builder.connect(CX, AN, out_matrix="to_AN", in_width=6, in_height=6, entry_z="top", growth_steps=63)
    builder.connect(CX, DESCENDING, out_matrix="to_DESCENDING", in_width=5, in_height=5, entry_z="top", growth_steps=63)
    builder.connect(CX, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="top", growth_steps=135)
    builder.connect(CX, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="top", growth_steps=81)
    builder.connect(CX, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="top", growth_steps=63)
    

    # LHLN  (instincts)
    builder.connect(LHLN, CENTRAL, out_matrix="to_CENTRAL", in_width=14, in_height=14, entry_z="bottom", growth_steps=81)
    builder.connect(LHLN, AN, out_matrix="to_AN", in_width=5, in_height=5, entry_z="bottom", growth_steps=63)
    builder.connect(LHLN, DESCENDING, out_matrix="to_DESCENDING", in_width=4, in_height=4, entry_z="top", growth_steps=63)
    builder.connect(LHLN, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="top", growth_steps=135)
    builder.connect(LHLN, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(LHLN, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)

    # ALLN  (local olfaction, from bottom)
    builder.connect(ALLN, CENTRAL, out_matrix="to_CENTRAL", in_width=13, in_height=13, entry_z="bottom", growth_steps=81)
    builder.connect(ALLN, AN, out_matrix="to_AN", in_width=5, in_height=5, entry_z="bottom", growth_steps=63)
    builder.connect(ALLN, DESCENDING, out_matrix="to_DESCENDING", in_width=4, in_height=4, entry_z="top", growth_steps=63)
    builder.connect(ALLN, VP, out_matrix="to_VP", in_width=4, in_height=4, entry_z="bottom", growth_steps=135)
    builder.connect(ALLN, CX, out_matrix="to_CX", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(ALLN, ALPN, out_matrix="to_ALPN", in_width=4, in_height=4, entry_z="bottom", growth_steps=63)
    builder.connect(ALLN, LHLN, out_matrix="to_LHLN", in_width=4, in_height=4, entry_z="bottom", growth_steps=81)

# =========================================================================
#                    I/O MATRICES (SDK v2 Features)
# =========================================================================
   # Semantic layout generation (Zero-Cost Facades)
    layout_nav = [
        "fly_pos_x", "fly_pos_y", "fly_pos_z",
        "fly_roll", "fly_pitch", "fly_yaw",
        "fly_vel_x", "fly_vel_y", "fly_vel_z",
        "fly_ang_x", "fly_ang_y", "fly_ang_z",
        "ori_x", "ori_y", "ori_z"
    ]
    layout_haltere = [f"haltere_{i}" for i in range(10)]
    layout_proprio = [f"joint_{i}" for i in range(42)]
    layout_contacts = [f"contact_{i}" for i in range(6)]
    layout_motors = [f"motor_{i}" for i in range(42)]

    # 1. Navigation -> CX (15 slots out of 16)
    CX.add_input("navigation", width=4, height=4, entry_z="top", layout=layout_nav)

    # 2. Haltere -> AN (10 slots out of 16)
    AN.add_input("haltere", width=4, height=4, entry_z="bottom", layout=layout_haltere)

    # 3. Proprioception -> VP (42 slots out of 64)
    VP.add_input("proprioception", width=8, height=8, entry_z="top", layout=layout_proprio)

    # 4. Reflexes (Contact Forces) -> DESCENDING (6 slots out of 16)
    # Direct hardware reflex, bypassing the cortex
    DESCENDING.add_input("reflexes", width=4, height=4, entry_z="bottom", layout=layout_contacts)

    # 5. Motors <- DESCENDING (42 slots out of 64)
    # Motor output extracted directly from descending paths
    DESCENDING.add_output("motors", width=8, height=8, target_type="All", layout=layout_motors)

#---------------------- AUTO-CONFIG COMPILATION AND BAKING ----------------------
    # TOML config compilation and VRAM binary dump baking
    builder.build().bake(clean=False)

#===================================================================================

if __name__ == '__main__':
    build_FLY_exp_brain()

