#Requires -RunAsAdministrator
# Run this INSIDE the Azure VM after deployment
# Downloads hvkube binary and Windows 11 template

$ErrorActionPreference = "Stop"
$ProgressPreference = 'Continue'

$TemplateDir = "C:\HyperVKube\Templates"
$BinDir = "C:\HyperVKube"
$TemplateName = "win11-dev"
$VhdxPath = "$TemplateDir\$TemplateName.vhdx"

Write-Host "=== HyperV-Kube VM Setup ===" -ForegroundColor Cyan

# Check Hyper-V
$hyperv = Get-WindowsFeature -Name Hyper-V
if ($hyperv.InstallState -ne "Installed") {
    Write-Error "Hyper-V not installed. Reboot may be pending."
}
Write-Host "[OK] Hyper-V installed" -ForegroundColor Green

# Download hvkube binary (from GitHub releases or build locally)
$hvkubePath = "$BinDir\hvkube.exe"
if (-not (Test-Path $hvkubePath)) {
    Write-Host "Building hvkube from source..."

    # Install Rust if needed
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "Installing Rust..."
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile "$env:TEMP\rustup-init.exe"
        & "$env:TEMP\rustup-init.exe" -y --default-toolchain stable
        $env:PATH += ";$env:USERPROFILE\.cargo\bin"
    }

    # Clone and build
    $repoDir = "$env:TEMP\hyperv-kube"
    if (Test-Path $repoDir) { Remove-Item $repoDir -Recurse -Force }
    git clone --depth 1 -b hyperv-impl https://github.com/louis030195/hcs-sandbox.git $repoDir
    Push-Location $repoDir
    cargo build --release
    Copy-Item "target\release\hvkube.exe" $hvkubePath
    Pop-Location
}
Write-Host "[OK] hvkube.exe ready" -ForegroundColor Green

# Add to PATH
$machinePath = [Environment]::GetEnvironmentVariable("PATH", "Machine")
if ($machinePath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$machinePath;$BinDir", "Machine")
    $env:PATH += ";$BinDir"
}

# Download Windows 11 Dev VM template
if (-not (Test-Path $VhdxPath)) {
    Write-Host "Downloading Windows 11 Dev VM (~20GB)..." -ForegroundColor Yellow
    Write-Host "This takes 10-20 min depending on connection speed"

    $zipPath = "$env:TEMP\WinDev.zip"
    Invoke-WebRequest -Uri "https://aka.ms/windev_VM_hyperv" -OutFile $zipPath -UseBasicParsing

    Write-Host "Extracting..."
    $extractDir = "$env:TEMP\WinDevExtract"
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

    $vhdx = Get-ChildItem -Path $extractDir -Filter "*.vhdx" -Recurse | Select-Object -First 1
    Move-Item $vhdx.FullName $VhdxPath -Force

    Remove-Item $zipPath, $extractDir -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host "[OK] Template: $VhdxPath" -ForegroundColor Green

# Register template
Write-Host "Registering template..."
& $hvkubePath template register --name $TemplateName --vhdx $VhdxPath

Write-Host ""
Write-Host "=== Setup Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Create a pool and start serving:" -ForegroundColor Cyan
Write-Host "  hvkube pool create --name agents --template $TemplateName --count 3"
Write-Host "  hvkube pool provision agents --count 3"
Write-Host "  hvkube pool prepare agents"
Write-Host "  hvkube serve --port 8080"
