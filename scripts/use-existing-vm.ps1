#Requires -RunAsAdministrator
# Use an existing Hyper-V VM as template
# Run as Administrator

param(
    [Parameter(Mandatory=$true)]
    [string]$SourceVmName,

    [string]$TemplateName = "win11-template",
    [string]$TemplateDir = "C:\HyperVKube\Templates"
)

$ErrorActionPreference = "Stop"

Write-Host "=== Create Template from Existing VM ===" -ForegroundColor Cyan

# Get source VM
$vm = Get-VM -Name $SourceVmName -ErrorAction Stop
Write-Host "Found VM: $($vm.Name) (State: $($vm.State))"

# Get VHDX path
$vhd = Get-VMHardDiskDrive -VMName $SourceVmName | Select-Object -First 1
if (-not $vhd) {
    Write-Error "VM has no hard disk attached"
}
$sourceVhdx = $vhd.Path
Write-Host "Source VHDX: $sourceVhdx"

# Create template directory
New-Item -ItemType Directory -Force -Path $TemplateDir | Out-Null

# Stop VM if running
if ($vm.State -eq "Running") {
    Write-Host "Stopping VM..."
    Stop-VM -Name $SourceVmName -Force
}

# Copy VHDX to template location
$targetVhdx = Join-Path $TemplateDir "$TemplateName.vhdx"
Write-Host "Copying VHDX to template location..."
Copy-Item $sourceVhdx $targetVhdx -Force

Write-Host ""
Write-Host "=== Template Created ===" -ForegroundColor Green
Write-Host "Template VHDX: $targetVhdx"
Write-Host ""
Write-Host "Register with hvkube:" -ForegroundColor Cyan
Write-Host "  hvkube template register --name $TemplateName --vhdx `"$targetVhdx`""
