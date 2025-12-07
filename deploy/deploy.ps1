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

# Get current IP
$MyIP = (Invoke-RestMethod -Uri "https://ifconfig.me/ip").Trim()
Write-Host "Your IP: $MyIP"

# Create resource group
Write-Host "Creating resource group: $ResourceGroup in $Location"
az group create --name $ResourceGroup --location $Location --output none

# Deploy ARM template
Write-Host "Deploying VM with Hyper-V (takes ~5 min)..."
$DeployOutput = az deployment group create `
    --resource-group $ResourceGroup `
    --template-file "$PSScriptRoot\azuredeploy.json" `
    --parameters vmName=$VmName adminPassword=$Password allowedIP=$MyIP `
    --query "properties.outputs" `
    --output json | ConvertFrom-Json

$PublicIP = $DeployOutput.publicIP.value
$FQDN = $DeployOutput.fqdn.value

Write-Host ""
Write-Host "=== Deployment Complete ===" -ForegroundColor Green
Write-Host "VM is rebooting to enable Hyper-V..."
Write-Host ""
Write-Host "Public IP: $PublicIP"
Write-Host "FQDN: $FQDN"
Write-Host "RDP: mstsc /v:$PublicIP"
Write-Host "API: http://${PublicIP}:8080 (after setup)"
Write-Host ""
Write-Host "=== Next Steps ===" -ForegroundColor Cyan
Write-Host "1. Wait 2-3 min for reboot"
Write-Host "2. RDP: mstsc /v:$PublicIP (user: hvadmin)"
Write-Host "3. Run setup inside VM (copies hvkube + downloads template)"
