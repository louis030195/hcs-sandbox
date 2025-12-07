# Deploy hyperv-kube to Azure
# Usage: .\deploy.ps1 -ResourceGroup "hyperv-kube-rg" -Location "eastus" -Password "MyP@ssw0rd123!"

param(
    [string]$ResourceGroup = "hyperv-kube-rg",
    [string]$Location = "eastus",
    [Parameter(Mandatory=$true)]
    [string]$Password,
    [string]$VmName = "hyperv-kube"
)

$ErrorActionPreference = "Stop"

$MyIP = (Invoke-RestMethod -Uri "https://ifconfig.me/ip").Trim()
Write-Host "Your IP: $MyIP"

Write-Host "Creating resource group: $ResourceGroup in $Location"
az group create --name $ResourceGroup --location $Location --output none

Write-Host "Deploying VM with Hyper-V (~5 min)..."
$DeployOutput = az deployment group create `
    --resource-group $ResourceGroup `
    --template-file "$PSScriptRoot\azuredeploy.json" `
    --parameters vmName=$VmName adminPassword=$Password allowedIP=$MyIP `
    --query "properties.outputs" `
    --output json | ConvertFrom-Json

$PublicIP = $DeployOutput.publicIP.value

Write-Host ""
Write-Host "=== Done ===" -ForegroundColor Green
Write-Host "Public IP: $PublicIP"
Write-Host "RDP: mstsc /v:$PublicIP (user: hvadmin)"
Write-Host ""
Write-Host "After RDP:" -ForegroundColor Cyan
Write-Host "  1. Copy hvkube.exe to VM"
Write-Host "  2. Download Win11 VHDX: aka.ms/windev_VM_hyperv"
Write-Host "  3. hvkube template register ..."
