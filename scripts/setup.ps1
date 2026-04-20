# Axicor Alpha - Windows Bootstrap
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "   Axicor HFT Engine - Windows Bootstrap" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan

# ---------------------------------------------------------
# 1. Dependency Validation & Auto-Install
# ---------------------------------------------------------

function Prompt-Install($ToolName, $WingetId) {
    $response = Read-Host "`n[?] $ToolName is missing or incompatible. Install via winget? (Y/N)"
    if ($response -match "^[yY]$") {
        Write-Host "Installing $ToolName..." -ForegroundColor Cyan
        winget install --id $WingetId -e --accept-package-agreements --accept-source-agreements
        return $true
    }
    return $false
}

$RestartRequired = $false

# --- 1.1 Python Check (Strict 3.10.x or 3.11.x required for ML wheels) ---
$PythonCmd = "python"
$PythonValid = $false

# 1. Try Python Launcher (py.exe) first (Safest method on Windows)
if (Get-Command "py" -ErrorAction SilentlyContinue) {
    $py_ver_str = (py -3.10 --version 2>&1)
    if ($LASTEXITCODE -eq 0 -and $py_ver_str -match "Python 3\.10") {
        $PythonCmd = "py -3.10"
        $PythonValid = $true
        Write-Host "[OK] Found Python 3.10 via Windows Launcher (py.exe)" -ForegroundColor Green
    } else {
        $py_ver_str = (py -3.11 --version 2>&1)
        if ($LASTEXITCODE -eq 0 -and $py_ver_str -match "Python 3\.11") {
            $PythonCmd = "py -3.11"
            $PythonValid = $true
            Write-Host "[OK] Found Python 3.11 via Windows Launcher (py.exe)" -ForegroundColor Green
        }
    }
}

# 2. Fallback to standard python if launcher failed or not found
if (-not $PythonValid -and (Get-Command "python" -ErrorAction SilentlyContinue)) {
    $py_ver_str = (python --version 2>&1)
    if ($py_ver_str -match "Python (3\.1[01])\.\d+") {
        $PythonCmd = "python"
        $PythonValid = $true
        Write-Host "[OK] Found Python $($matches[1]) via standard PATH" -ForegroundColor Green
    } else {
        Write-Host "[!] Found $py_ver_str in PATH, but Axicor requires Python 3.10 or 3.11." -ForegroundColor Yellow
        Write-Host "    (If you recently uninstalled a newer Python version, Windows might still be caching it in PATH)." -ForegroundColor Yellow
    }
}

if (-not $PythonValid) {
    if (Prompt-Install "Python 3.10" "Python.Python.3.10") {
        $RestartRequired = $true
    } else {
        Write-Host "[ERROR] Cannot proceed without Python 3.10 or 3.11." -ForegroundColor Red
        exit 1
    }
}

# --- 1.2 Rust Check ---
if (Get-Command "cargo" -ErrorAction SilentlyContinue) {
    Write-Host "[OK] Found $(cargo --version)" -ForegroundColor Green
} else {
    if (Prompt-Install "Rust (rustup)" "Rustlang.Rustup") {
        $RestartRequired = $true
    } else {
        Write-Host "[ERROR] Cannot proceed without Rust." -ForegroundColor Red
        exit 1
    }
}

# --- 1.3 Visual Studio Build Tools Check ---
$VsToolsFound = $false
$vswhere_path = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"

if (Test-Path $vswhere_path) {
    # Check for MSVC compiler toolset
    $msvc = & $vswhere_path -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($msvc) {
        $VsToolsFound = $true
        Write-Host "[OK] Found Visual Studio C++ Build Tools" -ForegroundColor Green
    }
}

if (-not $VsToolsFound) {
    Write-Host "`n[!] Visual Studio C++ Build Tools are missing!" -ForegroundColor Red
    Write-Host "Axicor requires 'Desktop development with C++' to compile." -ForegroundColor Yellow
    $response = Read-Host "[?] Open the official download page in your browser? (Y/N)"
    if ($response -match "^[yY]$") {
        Start-Process "https://visualstudio.microsoft.com/thank-you-downloading-visual-studio/?sku=Community&channel=Stable&version=VS18&source=VSLandingPage&cid=2500&passive=false"
        Write-Host "Please install the tools, check 'Desktop development with C++', and re-run this script." -ForegroundColor Cyan
    }
    exit 1
}

# --- 1.4 Git Check ---
if (-not (Get-Command "git" -ErrorAction SilentlyContinue)) {
    if (Prompt-Install "Git" "Git.Git") {
        $RestartRequired = $true
    }
}

if ($RestartRequired) {
    Write-Host "`n[!] Dependencies were installed. You MUST restart your terminal (PowerShell) for PATH changes to take effect." -ForegroundColor Yellow
    Write-Host "After restarting, run this script again:" -ForegroundColor Cyan
    Write-Host "powershell -ExecutionPolicy Bypass -File .\scripts\setup.ps1" -ForegroundColor White
    exit 0
}

# ---------------------------------------------------------
# 2. Virtual Environment & Dependencies
# ---------------------------------------------------------
Write-Host "`n[1/2] Setting up Python environment..." -ForegroundColor Cyan

$VenvValid = $false
if (Test-Path ".venv\Scripts\python.exe") {
    $venv_py_test = (& ".venv\Scripts\python.exe" --version 2>&1)
    if ($LASTEXITCODE -eq 0 -and $venv_py_test -match "Python (3\.1[01])\.\d+") {
        $VenvValid = $true
    }
}

if (-not $VenvValid) {
    if (Test-Path ".venv") {
        Write-Host "[!] Existing .venv is broken or uses an incorrect Python version. Recreating..." -ForegroundColor Yellow
        Remove-Item -Recurse -Force ".venv"
    }
    if ($PythonCmd -match "^py -3\.(10|11)$") {
        Invoke-Expression "$PythonCmd -m venv .venv"
    } else {
        python -m venv .venv
    }
    Write-Host "[OK] Created .venv using $PythonCmd" -ForegroundColor Green
} else {
    Write-Host "[OK] Valid .venv already exists" -ForegroundColor Green
}

$env:VIRTUAL_ENV = "$PWD\.venv"
$env:PATH = "$PWD\.venv\Scripts;$env:PATH"

Write-Host "Installing Python ML dependencies..." -ForegroundColor Cyan
python -m pip install -q --upgrade pip
python -m pip install -q numpy==1.26.4 "gymnasium[mujoco]==0.29.1" mujoco==2.3.7 pygame==2.5.2 optuna==3.6.1 toml==0.10.2
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Failed to install Python dependencies. Check the logs above." -ForegroundColor Red
    exit 1
}
Write-Host "[OK] Python dependencies installed" -ForegroundColor Green

# ---------------------------------------------------------
# 3. Build Axicor
# ---------------------------------------------------------
$GpuMode = "mock-gpu"
$NextStepsBrain = "python examples\ant_exp\build_brain.py --cpu"
$NextStepsNode = "cargo run --release -p axicor-node --features axicor-compute/mock-gpu -- Axicor-Models\AntConnectome.axic --cpu --log"

if (Get-Command "nvcc" -ErrorAction SilentlyContinue) {
    $GpuMode = "CUDA (NVIDIA)"
    $NextStepsBrain = "python examples\ant_exp\build_brain.py"
    $NextStepsNode = "cargo run --release -p axicor-node -- Axicor-Models\AntConnectome.axic --log"
} elseif (Get-Command "hipcc" -ErrorAction SilentlyContinue) {
    $GpuMode = "HIP (AMD)"
    $NextStepsBrain = "python examples\ant_exp\build_brain.py"
    $NextStepsNode = "cargo run --release -p axicor-node --features amd -- Axicor-Models\AntConnectome.axic --log"
} else {
    Write-Host "`n[!] No GPU compiler (nvcc / hipcc) detected in PATH. Falling back to CPU Mock-GPU mode." -ForegroundColor Yellow
}

Write-Host "`n[2/2] Building Axicor Node ($GpuMode)..." -ForegroundColor Cyan

if ($GpuMode -eq "CUDA (NVIDIA)") {
    cargo build --release -p axicor-node -p axicor-baker
} elseif ($GpuMode -eq "HIP (AMD)") {
    cargo build --release -p axicor-node -p axicor-baker --features amd
} else {
    cargo build --release -p axicor-node -p axicor-baker --features axicor-compute/mock-gpu
}

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n=============================================" -ForegroundColor Green
    Write-Host "  [SUCCESS] Axicor is ready! ($GpuMode)" -ForegroundColor Green
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host "`nNext steps to test the engine:"
    Write-Host "1. Build brain:  .venv\Scripts\Activate.ps1; $NextStepsBrain"
    Write-Host "2. Start node:   $NextStepsNode"
    Write-Host "3. Start agent:  .venv\Scripts\Activate.ps1; python examples\ant_exp\ant_agent.py"
} else {
    Write-Host "`n[ERROR] Build failed." -ForegroundColor Red
}
