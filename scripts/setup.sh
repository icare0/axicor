#!/bin/bash
# Axicor Alpha  Bootstrap Script
# https://github.com/H4V1K-dev/axicor
set -e

# ---------------------------------------------
# Colors
# ---------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

echo ""
echo -e "${BOLD}\033[38;5;196m"
echo "   #####+ ##+  ##+##+ ######+ ######+ ######+ "
echo "  ##+==##++##+##++##|##+====+##+===##+##+==##+"
echo "  #######| +###++ ##|##|     ##|   ##|######++"
echo "  ##+==##| ##+##+ ##|##|     ##|   ##|##+==##+"
echo "  ##|  ##|##++ ##+##|+######++######++##|  ##|"
echo "  +=+  +=++=+  +=++=+ +=====+ +=====+ +=+  +=+"
echo -e "${NC}"
echo -e "  ${BOLD}Embodied AI Engine  Alpha Bootstrap${NC}"
echo ""

# ---------------------------------------------
# 0. Argument Parsing
# ---------------------------------------------
MOCK_ARG=0
for arg in "$@"; do
    if [ "$arg" == "--mock" ] || [ "$arg" == "--cpu" ]; then
        MOCK_ARG=1
    fi
done

# ---------------------------------------------
# 1. Mandatory dependencies
# ---------------------------------------------
echo -e "${CYAN}[1/5] Checking dependencies...${NC}"

MISSING=0

check_cmd() {
    if command -v "$1" >/dev/null 2>&1; then
        echo -e "  ${GREEN}${NC} $1 found"
    else
        echo -e "  ${RED}${NC} $1 not found  $2"
        MISSING=1
    fi
}

check_cmd python3  "install from https://python.org"
check_cmd cargo   "install from https://rustup.rs"
check_cmd git     "install via your package manager"
# [DOD FIX] Host-compiler lock for CUDA
check_cmd gcc-13   "install via apt (sudo apt install gcc-13 g++-13)"

if [ $MISSING -eq 1 ]; then
    echo ""
    echo -e "${RED}[ERROR] Missing required dependencies. Please install them and re-run.${NC}"
    exit 1
fi

# ---------------------------------------------
# 2. GPU Detection
# ---------------------------------------------
echo ""
echo -e "${CYAN}[2/5] Detecting GPU backend...${NC}"

GPU_FEATURES=""
GPU_FOUND=0

if command -v nvcc >/dev/null 2>&1; then
    NVCC_VER=$(nvcc --version | grep "release" | awk '{print $6}' | cut -c2-)
    
    # Extract Major and Minor versions
    NVCC_MAJOR=$(echo $NVCC_VER | cut -d'.' -f1)
    NVCC_MINOR=$(echo $NVCC_VER | cut -d'.' -f2)
    
    # DOD Check: Requires minimum CUDA 12.4 for correct unfolding of 32-byte structures and __shfl_sync
    if [ "$NVCC_MAJOR" -lt 12 ] || ( [ "$NVCC_MAJOR" -eq 12 ] && [ "$NVCC_MINOR" -lt 4 ] ); then
        echo -e "  ${RED}${NC} NVIDIA CUDA version too old (nvcc ${NVCC_VER}). Axicor requires >= 12.4."
        echo -e "  Old nvcc heuristic analyzers crash on branchless AST unrolling."
        exit 1
    fi
    
    echo -e "  ${GREEN}${NC} NVIDIA CUDA found (nvcc ${NVCC_VER})"
    GPU_FEATURES=""
    GPU_FOUND=1
elif command -v hipcc >/dev/null 2>&1; then
    HIPCC_VER=$(hipcc --version 2>&1 | head -1)
    echo -e "  ${GREEN}${NC} AMD ROCm found (${HIPCC_VER})"
    GPU_FEATURES="--features amd"
    GPU_FOUND=1
else
    echo -e "  ${YELLOW}[WARN]${NC}  No GPU toolkit detected (nvcc / hipcc not found)"
fi

if [ $GPU_FOUND -eq 0 ]; then
    if [ $MOCK_ARG -eq 1 ]; then
        GPU_FEATURES="--features mock-gpu"
        echo -e "  ${YELLOW} No GPU detected, but --mock/--cpu was passed. Enabling Mock-GPU mode.${NC}"
    else
        echo ""
        echo -e "  ${YELLOW}Axicor requires CUDA or ROCm for full performance.${NC}"
        echo -e "  A ${BOLD}CPU mock mode${NC} is available for testing without GPU.${NC}"
        echo ""
        echo -e "  ${RED}[ERROR] No GPU toolkit detected and --mock not provided.${NC}"
        echo -e "  To continue without a GPU, use: ${BOLD}bash scripts/setup.sh --mock${NC}"
        echo ""
        echo -e "  CUDA:  https://developer.nvidia.com/cuda-downloads"
        echo -e "  ROCm:  https://rocm.docs.amd.com/en/latest/deploy/linux/index.html"
        exit 1
    fi
fi

# ---------------------------------------------
# 3. Python venv
# ---------------------------------------------
echo ""
echo -e "${CYAN}[3/5] Setting up Python environment...${NC}"

if [ ! -d ".venv" ]; then
    echo -e "  Creating virtual environment..."
    python3 -m venv .venv
    echo -e "  ${GREEN}${NC} .venv created"
else
    echo -e "  ${GREEN}${NC} .venv already exists"
fi

source .venv/bin/activate

echo -e "  Installing Python dependencies..."
pip install -q --upgrade pip
# [DOD FIX] Strict version pinning to prevent C-ABI breakage
pip install -q numpy==1.26.4 gymnasium==0.29.1 pygame==2.5.2 optuna==3.6.1 toml==0.10.2

echo -e "  ${GREEN}${NC} Python dependencies installed"

# ---------------------------------------------
# 4. Rust build
# ---------------------------------------------
echo ""
echo -e "${CYAN}[4/5] Building Axicor Node (release)...${NC}"
echo -e "  Features: ${BOLD}${GPU_FEATURES:-default}${NC}"
echo ""

cargo build --release -p axicor-node -p axicor-baker $GPU_FEATURES

echo -e "  ${GREEN}${NC} Build complete"

# ---------------------------------------------
# 5. Verification
# ---------------------------------------------
echo ""
echo -e "${CYAN}[5/5] Verifying installation...${NC}"

NODE_BIN="./target/release/axicor-node"
BAKER_BIN="./target/release/axicor-baker"

[ -f "$NODE_BIN" ]  && echo -e "  ${GREEN}${NC} axicor-node binary found" \
                    || echo -e "  ${RED}${NC} axicor-node binary missing"

[ -f "$BAKER_BIN" ] && echo -e "  ${GREEN}${NC} axicor-baker binary found" \
                    || echo -e "  ${RED}${NC} axicor-baker binary missing"

# ---------------------------------------------
# Done
# ---------------------------------------------
echo ""
echo -e "${GREEN}${BOLD}${NC}"
echo -e "${GREEN}${BOLD}  [OK] Axicor is ready.${NC}"
echo -e "${GREEN}${BOLD}${NC}"
echo ""
echo -e "  ${BOLD}Next steps:${NC}"
echo ""
echo -e "  ${CYAN}1.${NC} Bake the brain:"
echo -e "     ${BOLD}python3 examples/ant_exp/build_brain.py${NC}"
echo ""
echo -e "  ${CYAN}2.${NC} Start the node:"
echo -e "     ${BOLD}cargo run --release -p axicor-node -- Axicor-Models/AntConnectome.axic --cpu --log${NC}"
echo ""
echo -e "  ${CYAN}3.${NC} Run the agent:"
echo -e "     ${BOLD}python3 examples/ant_exp/ant_agent.py${NC}"
echo ""
echo -e "  Docs: ${CYAN}https://github.com/H4V1K-dev/axicor${NC}"
echo ""
