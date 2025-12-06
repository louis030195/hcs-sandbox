#Requires -RunAsAdministrator
# End-to-end test: acquire VMs, run MCP calls, release
# Usage: .\test-e2e.ps1 -PoolName test-pool -Count 2

param(
    [string]$PoolName = "test-pool",
    [int]$Count = 2,
    [string]$ApiUrl = "http://localhost:8080"
)

$ErrorActionPreference = "Stop"

Write-Host "=== E2E Test: Parallel VM Execution ===" -ForegroundColor Cyan
Write-Host "Pool: $PoolName, VMs: $Count"
Write-Host ""

# Acquire VMs in parallel
Write-Host "[1] Acquiring $Count VMs..." -ForegroundColor Yellow
$vms = @()
$jobs = @()

for ($i = 0; $i -lt $Count; $i++) {
    $jobs += Start-Job -ScriptBlock {
        param($url, $pool)
        $body = @{ pool_name = $pool } | ConvertTo-Json
        $result = Invoke-RestMethod -Uri "$url/api/v1/acquire" -Method POST -Body $body -ContentType "application/json"
        return $result
    } -ArgumentList $ApiUrl, $PoolName
}

$results = $jobs | Wait-Job | Receive-Job
$jobs | Remove-Job

foreach ($vm in $results) {
    Write-Host "  Acquired: $($vm.vm_name) @ $($vm.ip_address) (${$vm.resume_time_ms}ms)" -ForegroundColor Green
    Write-Host "    MCP: $($vm.mcp_endpoint)"
    $vms += $vm
}

# Test MCP health on each
Write-Host ""
Write-Host "[2] Testing MCP health on each VM..." -ForegroundColor Yellow
foreach ($vm in $vms) {
    try {
        $health = Invoke-RestMethod -Uri "http://$($vm.ip_address):8080/health" -TimeoutSec 5
        Write-Host "  $($vm.vm_name): $($health.status) (v$($health.version))" -ForegroundColor Green
    } catch {
        Write-Host "  $($vm.vm_name): FAILED - $_" -ForegroundColor Red
    }
}

# Run a simple MCP tool call on each (list available tools)
Write-Host ""
Write-Host "[3] Calling MCP tools/list on each VM..." -ForegroundColor Yellow
foreach ($vm in $vms) {
    try {
        $mcpRequest = @{
            jsonrpc = "2.0"
            id = 1
            method = "tools/list"
        } | ConvertTo-Json
        
        $response = Invoke-RestMethod -Uri $vm.mcp_endpoint -Method POST -Body $mcpRequest -ContentType "application/json" -TimeoutSec 10
        $toolCount = $response.result.tools.Count
        Write-Host "  $($vm.vm_name): $toolCount tools available" -ForegroundColor Green
    } catch {
        Write-Host "  $($vm.vm_name): MCP call failed - $_" -ForegroundColor Red
    }
}

# Release VMs
Write-Host ""
Write-Host "[4] Releasing VMs..." -ForegroundColor Yellow
foreach ($vm in $vms) {
    try {
        $body = @{ reset = $false } | ConvertTo-Json
        Invoke-RestMethod -Uri "$ApiUrl/api/v1/vms/$($vm.vm_name)/release" -Method POST -Body $body -ContentType "application/json" | Out-Null
        Write-Host "  Released: $($vm.vm_name)" -ForegroundColor Green
    } catch {
        Write-Host "  Failed to release $($vm.vm_name): $_" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== E2E Test Complete ===" -ForegroundColor Cyan
