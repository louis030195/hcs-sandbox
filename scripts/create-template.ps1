#Requires -RunAsAdministrator
# Create a Windows 11 template for hyperv-kube
# Run this script as Administrator

param(
    [string]$TemplateName = "win11-base",
    [string]$TemplateDir = "C:\HyperVKube\Templates",
    [int]$MemoryMB = 4096,
    [int]$CPUs = 2,
    [int]$DiskSizeGB = 40
)

$ErrorActionPreference = "Stop"

Write-Host "=== HyperV-Kube Template Creator ===" -ForegroundColor Cyan

# Check Hyper-V
$hyperv = Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V
if ($hyperv.State -ne "Enabled") {
    Write-Host "Hyper-V is not enabled. Enabling now (requires reboot)..." -ForegroundColor Yellow
    Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All -NoRestart
    Write-Host "Please reboot and run this script again." -ForegroundColor Red
    exit 1
}

# Create directories
New-Item -ItemType Directory -Force -Path $TemplateDir | Out-Null
New-Item -ItemType Directory -Force -Path "C:\HyperVKube\VMs" | Out-Null

$VhdxPath = Join-Path $TemplateDir "$TemplateName.vhdx"
$VmName = "template-$TemplateName"

# Check if template already exists
if (Test-Path $VhdxPath) {
    Write-Host "Template VHDX already exists: $VhdxPath" -ForegroundColor Yellow
    $response = Read-Host "Delete and recreate? (y/N)"
    if ($response -ne "y") {
        Write-Host "Exiting."
        exit 0
    }
    # Remove existing VM if any
    $existingVm = Get-VM -Name $VmName -ErrorAction SilentlyContinue
    if ($existingVm) {
        Stop-VM -Name $VmName -Force -ErrorAction SilentlyContinue
        Remove-VM -Name $VmName -Force
    }
    Remove-Item $VhdxPath -Force
}

Write-Host ""
Write-Host "Choose installation method:" -ForegroundColor Cyan
Write-Host "  1. Download Windows 11 Dev Environment (Free, ~20GB download)"
Write-Host "  2. Use existing Windows ISO"
Write-Host "  3. Create blank VHDX (you install Windows manually)"
Write-Host ""
$choice = Read-Host "Enter choice (1/2/3)"

switch ($choice) {
    "1" {
        # Download Windows 11 Dev Environment
        Write-Host "Downloading Windows 11 Development Environment..." -ForegroundColor Cyan
        Write-Host "This is a free pre-configured VM from Microsoft (~20GB)" -ForegroundColor Gray

        $downloadUrl = "https://aka.ms/windev_VM_hyperv"
        $zipPath = Join-Path $env:TEMP "WinDev.zip"

        Write-Host "Downloading from: $downloadUrl"
        Write-Host "This may take 10-30 minutes depending on your connection..."

        # Download with progress
        $ProgressPreference = 'Continue'
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing

        Write-Host "Extracting..." -ForegroundColor Cyan
        $extractPath = Join-Path $env:TEMP "WinDevExtract"
        Expand-Archive -Path $zipPath -DestinationPath $extractPath -Force

        # Find the VHDX in extracted files
        $sourceVhdx = Get-ChildItem -Path $extractPath -Filter "*.vhdx" -Recurse | Select-Object -First 1
        if (-not $sourceVhdx) {
            # Might be a .vmdk or need conversion
            $sourceVmdk = Get-ChildItem -Path $extractPath -Filter "*.vmdk" -Recurse | Select-Object -First 1
            if ($sourceVmdk) {
                Write-Host "Found VMDK, converting to VHDX..."
                # Would need qemu-img or similar for conversion
                Write-Error "VMDK conversion not yet implemented. Please use option 2 or 3."
            }
            Write-Error "No VHDX found in downloaded package"
        }

        Write-Host "Moving VHDX to template directory..."
        Move-Item $sourceVhdx.FullName $VhdxPath -Force

        # Cleanup
        Remove-Item $zipPath -Force -ErrorAction SilentlyContinue
        Remove-Item $extractPath -Recurse -Force -ErrorAction SilentlyContinue

        Write-Host "Template VHDX created: $VhdxPath" -ForegroundColor Green
    }

    "2" {
        # Use existing ISO
        $isoPath = Read-Host "Enter path to Windows ISO"
        if (-not (Test-Path $isoPath)) {
            Write-Error "ISO not found: $isoPath"
        }

        Write-Host "Creating VHDX from ISO..." -ForegroundColor Cyan

        # Create blank VHDX
        New-VHD -Path $VhdxPath -SizeBytes ($DiskSizeGB * 1GB) -Dynamic

        # Create VM
        New-VM -Name $VmName -MemoryStartupBytes ($MemoryMB * 1MB) -Generation 2 -VHDPath $VhdxPath
        Set-VM -Name $VmName -ProcessorCount $CPUs

        # Attach ISO
        Add-VMDvdDrive -VMName $VmName -Path $isoPath

        # Set boot order (DVD first)
        $dvd = Get-VMDvdDrive -VMName $VmName
        Set-VMFirmware -VMName $VmName -FirstBootDevice $dvd

        # Disable Secure Boot for easier installation
        Set-VMFirmware -VMName $VmName -EnableSecureBoot Off

        Write-Host ""
        Write-Host "VM '$VmName' created with ISO attached." -ForegroundColor Green
        Write-Host "Next steps:" -ForegroundColor Yellow
        Write-Host "  1. Start the VM: Start-VM -Name $VmName"
        Write-Host "  2. Connect to console: vmconnect localhost $VmName"
        Write-Host "  3. Install Windows manually"
        Write-Host "  4. After installation, run: .\finalize-template.ps1 -VmName $VmName"
        exit 0
    }

    "3" {
        # Create blank VHDX
        Write-Host "Creating blank VHDX..." -ForegroundColor Cyan
        New-VHD -Path $VhdxPath -SizeBytes ($DiskSizeGB * 1GB) -Dynamic

        Write-Host "Blank VHDX created: $VhdxPath" -ForegroundColor Green
        Write-Host ""
        Write-Host "To use this, you need to:" -ForegroundColor Yellow
        Write-Host "  1. Create a VM pointing to this VHDX"
        Write-Host "  2. Attach a Windows ISO"
        Write-Host "  3. Install Windows"
        Write-Host "  4. Configure the system"
        exit 0
    }

    default {
        Write-Error "Invalid choice"
    }
}

# If we got here with a VHDX (option 1), create and configure the template VM
if (Test-Path $VhdxPath) {
    Write-Host ""
    Write-Host "Creating template VM..." -ForegroundColor Cyan

    # Create VM
    New-VM -Name $VmName -MemoryStartupBytes ($MemoryMB * 1MB) -Generation 2 -VHDPath $VhdxPath
    Set-VM -Name $VmName -ProcessorCount $CPUs -AutomaticStartAction Nothing -AutomaticStopAction Save
    Set-VMMemory -VMName $VmName -DynamicMemoryEnabled $true

    # Connect to default switch
    $switch = Get-VMSwitch | Where-Object { $_.SwitchType -eq "Internal" -or $_.Name -eq "Default Switch" } | Select-Object -First 1
    if ($switch) {
        Connect-VMNetworkAdapter -VMName $VmName -SwitchName $switch.Name
    }

    # Enable enhanced session
    Set-VM -Name $VmName -EnhancedSessionTransportType HvSocket

    Write-Host ""
    Write-Host "=== Template VM Created ===" -ForegroundColor Green
    Write-Host "VM Name: $VmName"
    Write-Host "VHDX: $VhdxPath"
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "  1. Start and customize the VM:"
    Write-Host "     Start-VM -Name $VmName"
    Write-Host "     vmconnect localhost $VmName"
    Write-Host ""
    Write-Host "  2. Inside the VM, install your software (Chrome, Node, etc.)"
    Write-Host ""
    Write-Host "  3. Configure auto-login (optional):"
    Write-Host "     netplwiz -> uncheck 'Users must enter a username and password'"
    Write-Host ""
    Write-Host "  4. Shutdown the VM, then finalize:"
    Write-Host "     Stop-VM -Name $VmName"
    Write-Host "     Remove-VM -Name $VmName -Force  # Keep the VHDX!"
    Write-Host ""
    Write-Host "  5. Register with hvkube:"
    Write-Host "     hvkube template register --name $TemplateName --vhdx `"$VhdxPath`" --memory $MemoryMB --cpus $CPUs"
}
