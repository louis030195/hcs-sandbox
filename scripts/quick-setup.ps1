#Requires -RunAsAdministrator
# Quick setup - downloads Windows 11 dev VM and sets up hvkube
# Run: Start-Process powershell -Verb RunAs -ArgumentList "-File C:\Users\louis030195\Documents\hyperv-kube\scripts\quick-setup.ps1"

$ErrorActionPreference = "Stop"
$ProgressPreference = 'Continue'

$TemplateDir = "C:\HyperVKube\Templates"
$VmDir = "C:\HyperVKube\VMs"
$TemplateName = "win11-dev"
$VhdxPath = "$TemplateDir\$TemplateName.vhdx"

Write-Host "=== HyperV-Kube Quick Setup ===" -ForegroundColor Cyan
Write-Host ""

# 1. Check/Enable Hyper-V
Write-Host "[1/5] Checking Hyper-V..." -ForegroundColor Yellow
$hyperv = Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V
if ($hyperv.State -ne "Enabled") {
    Write-Host "Enabling Hyper-V (will require reboot)..."
    Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All -NoRestart
    Write-Host ""
    Write-Host "Hyper-V enabled! Please REBOOT and run this script again." -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}
Write-Host "  Hyper-V is enabled." -ForegroundColor Green

# 2. Create directories
Write-Host "[2/5] Creating directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $TemplateDir | Out-Null
New-Item -ItemType Directory -Force -Path $VmDir | Out-Null
Write-Host "  Created $TemplateDir" -ForegroundColor Green
Write-Host "  Created $VmDir" -ForegroundColor Green

# 3. Download Windows 11 Dev Environment
if (Test-Path $VhdxPath) {
    Write-Host "[3/5] Template already exists: $VhdxPath" -ForegroundColor Green
} else {
    Write-Host "[3/5] Downloading Windows 11 Dev Environment (~20GB)..." -ForegroundColor Yellow
    Write-Host "  This is a free pre-configured VM from Microsoft"
    Write-Host "  URL: https://aka.ms/windev_VM_hyperv"
    Write-Host ""

    $zipPath = "$env:TEMP\WinDev_HyperV.zip"

    # Download
    try {
        Invoke-WebRequest -Uri "https://aka.ms/windev_VM_hyperv" -OutFile $zipPath -UseBasicParsing
    } catch {
        Write-Host "  Download failed. Trying alternative..." -ForegroundColor Yellow
        # Alternative: direct link (may change)
        Invoke-WebRequest -Uri "https://go.microsoft.com/fwlink/?linkid=2243733" -OutFile $zipPath -UseBasicParsing
    }

    Write-Host "  Extracting..." -ForegroundColor Yellow
    $extractDir = "$env:TEMP\WinDevExtract"
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

    # Find VHDX
    $vhdx = Get-ChildItem -Path $extractDir -Filter "*.vhdx" -Recurse | Select-Object -First 1
    if (-not $vhdx) {
        Write-Error "No VHDX found in download. Contents: $(Get-ChildItem $extractDir -Recurse | Select-Object -ExpandProperty Name)"
    }

    # Move to templates
    Move-Item $vhdx.FullName $VhdxPath -Force
    Write-Host "  Template saved: $VhdxPath" -ForegroundColor Green

    # Cleanup
    Remove-Item $zipPath -Force -ErrorAction SilentlyContinue
    Remove-Item $extractDir -Recurse -Force -ErrorAction SilentlyContinue
}

# 4. Create test VM from template
Write-Host "[4/5] Creating test VM from template..." -ForegroundColor Yellow

$TestVmName = "hvkube-test-0"
$TestVhdx = "$VmDir\$TestVmName\disk.vhdx"

# Remove existing test VM
$existing = Get-VM -Name $TestVmName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "  Removing existing test VM..."
    Stop-VM -Name $TestVmName -Force -ErrorAction SilentlyContinue
    Remove-VM -Name $TestVmName -Force
    Remove-Item (Split-Path $TestVhdx) -Recurse -Force -ErrorAction SilentlyContinue
}

# Create directory
New-Item -ItemType Directory -Force -Path (Split-Path $TestVhdx) | Out-Null

# Create differencing disk
Write-Host "  Creating differencing disk (COW clone)..."
New-VHD -Path $TestVhdx -ParentPath $VhdxPath -Differencing | Out-Null

# Create VM
Write-Host "  Creating VM..."
New-VM -Name $TestVmName -MemoryStartupBytes 4GB -Generation 2 -VHDPath $TestVhdx | Out-Null
Set-VM -Name $TestVmName -ProcessorCount 2 -AutomaticStartAction Nothing -AutomaticStopAction Save
Set-VMMemory -VMName $TestVmName -DynamicMemoryEnabled $true

# Connect to switch
$switch = Get-VMSwitch | Where-Object { $_.Name -eq "Default Switch" } | Select-Object -First 1
if (-not $switch) {
    $switch = Get-VMSwitch | Select-Object -First 1
}
if ($switch) {
    Connect-VMNetworkAdapter -VMName $TestVmName -SwitchName $switch.Name
}

Write-Host "  VM created: $TestVmName" -ForegroundColor Green

# 5. First boot and save
Write-Host "[5/5] Booting VM and saving state..." -ForegroundColor Yellow
Write-Host "  Starting VM (first boot takes 30-60 seconds)..."

Start-VM -Name $TestVmName

# Wait for VM to be ready
Write-Host "  Waiting for VM to boot..."
$timeout = 180  # 3 minutes
$start = Get-Date
while ($true) {
    Start-Sleep -Seconds 5
    $elapsed = ((Get-Date) - $start).TotalSeconds

    if ($elapsed -gt $timeout) {
        Write-Host "  Timeout waiting for VM. Check manually." -ForegroundColor Yellow
        break
    }

    # Check for IP
    $ip = (Get-VMNetworkAdapter -VMName $TestVmName).IPAddresses | Where-Object { $_ -match '^\d+\.\d+\.\d+\.\d+$' } | Select-Object -First 1
    if ($ip) {
        Write-Host "  VM has IP: $ip" -ForegroundColor Green

        # Wait a bit more for Windows to settle
        Write-Host "  Waiting for Windows to settle (30s)..."
        Start-Sleep -Seconds 30
        break
    }

    Write-Host "  Still waiting... ($([int]$elapsed)s)"
}

# Create checkpoint
Write-Host "  Creating checkpoint..."
Checkpoint-VM -Name $TestVmName -SnapshotName "clean"

# Save state
Write-Host "  Saving VM state..."
Save-VM -Name $TestVmName

Write-Host ""
Write-Host "=== Setup Complete! ===" -ForegroundColor Green
Write-Host ""
Write-Host "Template: $VhdxPath"
Write-Host "Test VM:  $TestVmName (state: Saved)"
Write-Host ""
Write-Host "Test fast resume:" -ForegroundColor Cyan
Write-Host "  Start-VM -Name $TestVmName"
Write-Host "  # Should resume in 2-5 seconds!"
Write-Host ""
Write-Host "Register with hvkube:" -ForegroundColor Cyan
Write-Host "  cd C:\Users\louis030195\Documents\hyperv-kube"
Write-Host "  cargo build --release"
Write-Host "  .\target\release\hvkube.exe template register --name $TemplateName --vhdx `"$VhdxPath`""
Write-Host "  .\target\release\hvkube.exe pool create --name test-pool --template $TemplateName"
Write-Host ""

Read-Host "Press Enter to exit"
