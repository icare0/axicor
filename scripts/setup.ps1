Write-Host "Axicor Alpha - Windows Bootstrap" -ForegroundColor Cyan

# 1. Dependency Check
if (-not (Get-Command "python" -ErrorAction SilentlyContinue)) {
    Write-Host "Python not found. Please install Python 3.10+" -ForegroundColor Red
    exit 1
}
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Host "Rust (cargo) not found. Please install from rustup.rs" -ForegroundColor Red
    exit 1
}

# 2. Virtual Environment
Write-Host "Setting up Python environment..." -ForegroundColor Cyan
if (-not (Test-Path ".venv")) {
    python -m venv .venv
}
& .venv\Scripts\Activate.ps1

Write-Host "Installing Python dependencies..."
python -m pip install -q --upgrade pip
python -m pip install -q numpy==1.26.4 gymnasium==0.29.1 pygame==2.5.2 optuna==3.6.1 toml==0.10.2

# 3. Build Node & Baker in Mock-GPU mode
Write-Host "Building Genesis Node (Mock-GPU)..." -ForegroundColor Cyan
cargo build --release -p genesis-node -p genesis-baker --features genesis-compute/mock-gpu

Write-Host "Installation Verified. Run with .venv\Scripts\Activate.ps1" -ForegroundColor Green
