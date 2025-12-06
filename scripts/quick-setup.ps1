#Requires -RunAsAdministrator
# Quick setup - downloads Windows 11 dev VM, installs terminator, sets up hvkube

$ErrorActionPreference = "Stop"
$ProgressPreference = 'Continue'

$TemplateDir = "C:\HyperVKube\Templates"
$VmDir = "C:\HyperVKube\VMs"
$TemplateName = "win11-terminator"
$VhdxPath = "$TemplateDir\$TemplateName.vhdx"
$TestVmName = "hvkube-test-0"

Write-Host "=== HyperV-Kube Setup ===" -ForegroundColor Cyan

# 1. Check Hyper-V
$hyperv = Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V
if ($hyperv.State -ne "Enabled") {
    Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All -NoRestart
    Write-Host "Hyper-V enabled. REBOOT and run again." -ForegroundColor Red
    exit 1
}
Write-Host "[OK] Hyper-V enabled" -ForegroundColor Green

# 2. Directories
New-Item -ItemType Directory -Force -Path $TemplateDir, $VmDir | Out-Null

# 3. Download Win11 Dev VM if needed
if (-not (Test-Path $VhdxPath)) {
    Write-Host "Downloading Windows 11 Dev VM (~20GB)..." -ForegroundColor Yellow
    $zipPath = "$env:TEMP\WinDev.zip"
    Invoke-WebRequest -Uri "https://aka.ms/windev_VM_hyperv" -OutFile $zipPath -UseBasicParsing
    
    $extractDir = "$env:TEMP\WinDevExtract"
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force
    $vhdx = Get-ChildItem -Path $extractDir -Filter "*.vhdx" -Recurse | Select-Object -First 1
    Move-Item $vhdx.FullName $VhdxPath -Force
    Remove-Item $zipPath, $extractDir -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host "[OK] Template: $VhdxPath" -ForegroundColor Green

# 4. Create test VM
$existing = Get-VM -Name $TestVmName -ErrorAction SilentlyContinue
if ($existing) {
    Stop-VM -Name $TestVmName -Force -TurnOff -ErrorAction SilentlyContinue
    Remove-VM -Name $TestVmName -Force
}

$TestVhdx = "$VmDir\$TestVmName\disk.vhdx"
New-Item -ItemType Directory -Force -Path (Split-Path $TestVhdx) | Out-Null
New-VHD -Path $TestVhdx -ParentPath $VhdxPath -Differencing | Out-Null
New-VM -Name $TestVmName -MemoryStartupBytes 4GB -Generation 2 -VHDPath $TestVhdx | Out-Null
Set-VM -Name $TestVmName -ProcessorCount 2 -AutomaticStopAction Save

$switch = Get-VMSwitch | Where-Object { $_.SwitchType -eq "Internal" -or $_.Name -eq "Default Switch" } | Select-Object -First 1
if ($switch) { Connect-VMNetworkAdapter -VMName $TestVmName -SwitchName $switch.Name }

Write-Host "[OK] VM created: $TestVmName" -ForegroundColor Green

# 5. Boot and install terminator
Write-Host "Booting VM to install terminator..." -ForegroundColor Yellow
Start-VM -Name $TestVmName

# Wait for IP
$timeout = 300
$start = Get-Date
while ($true) {
    Start-Sleep -Seconds 5
    $elapsed = ((Get-Date) - $start).TotalSeconds
    if ($elapsed -gt $timeout) { Write-Error "Timeout waiting for VM" }
    
    $ip = (Get-VMNetworkAdapter -VMName $TestVmName).IPAddresses | Where-Object { $_ -match '^\d+\.\d+\.\d+\.\d+$' } | Select-Object -First 1
    if ($ip) {
        Write-Host "  VM IP: $ip" -ForegroundColor Green
        break
    }
    Write-Host "  Waiting... ($([int]$elapsed)s)"
}

# Wait for Windows to settle
Write-Host "  Waiting for Windows to settle..."
Start-Sleep -Seconds 45

# Install terminator via PowerShell Direct (Win11 dev VM user: "User", no password by default)
Write-Host "Installing terminator MCP agent..." -ForegroundColor Yellow
$cred = Get-Credential -Message "Enter VM credentials (Win11 Dev default: User / empty password)"

Invoke-Command -VMName $TestVmName -Credential $cred -ScriptBlock {
    $ErrorActionPreference = "Stop"
    
    # Create MCP directory
    New-Item -ItemType Directory -Force -Path "C:\MCP" | Out-Null
    
    # Download latest terminator release
    $release = Invoke-RestMethod "https://api.github.com/repos/mediar-ai/terminator/releases/latest"
    $asset = $release.assets | Where-Object { $_.name -like "*win32-x64*" -and $_.name -like "*.zip" } | Select-Object -First 1
    
    if (-not $asset) { throw "No Windows release found" }
    
    $zipPath = "C:\MCP\terminator.zip"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zipPath -UseBasicParsing
    Expand-Archive -Path $zipPath -DestinationPath "C:\MCP" -Force
    Remove-Item $zipPath -Force
    
    # Find the exe
    $exe = Get-ChildItem "C:\MCP" -Filter "*.exe" -Recurse | Select-Object -First 1
    if ($exe) { Move-Item $exe.FullName "C:\MCP\terminator-mcp-agent.exe" -Force }
    
    # Create startup script
    $startupScript = @"
Start-Process -FilePath "C:\MCP\terminator-mcp-agent.exe" -ArgumentList "-t http --host 0.0.0.0 -p 8080" -WindowStyle Hidden
"@
    $startupScript | Out-File "C:\MCP\start-mcp.ps1" -Encoding UTF8
    
    # Add to startup via scheduled task
    $action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-ExecutionPolicy Bypass -File C:\MCP\start-mcp.ps1"
    $trigger = New-ScheduledTaskTrigger -AtLogon
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Highest
    Register-ScheduledTask -TaskName "MCP-Agent" -Action $action -Trigger $trigger -Principal $principal -Force
    
    # Start it now
    Start-Process -FilePath "C:\MCP\terminator-mcp-agent.exe" -ArgumentList "-t http --host 0.0.0.0 -p 8080" -WindowStyle Hidden
    
    Write-Host "Terminator installed and started"
}

# Wait for terminator to be ready
Write-Host "Waiting for terminator health check..." -ForegroundColor Yellow
$mcpReady = $false
for ($i = 0; $i -lt 30; $i++) {
    try {
        $health = Invoke-RestMethod -Uri "http://${ip}:8080/health" -TimeoutSec 2
        if ($health.status -eq "healthy") {
            $mcpReady = $true
            break
        }
    } catch {}
    Start-Sleep -Seconds 2
}

if ($mcpReady) {
    Write-Host "[OK] Terminator healthy at http://${ip}:8080" -ForegroundColor Green
} else {
    Write-Host "[WARN] Terminator not responding, check manually" -ForegroundColor Yellow
}

# 6. Checkpoint and save
Write-Host "Creating checkpoint and saving state..." -ForegroundColor Yellow
Checkpoint-VM -Name $TestVmName -SnapshotName "clean-with-terminator"
Save-VM -Name $TestVmName

Write-Host ""
Write-Host "=== Setup Complete ===" -ForegroundColor Green
Write-Host "VM: $TestVmName (Saved with terminator)"
Write-Host ""
Write-Host "Test:" -ForegroundColor Cyan
Write-Host "  Start-VM -Name $TestVmName"
Write-Host "  # Then: curl http://${ip}:8080/health"
