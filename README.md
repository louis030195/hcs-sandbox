# HyperV-Kube

VM orchestrator using Hyper-V Save/Resume for **2-5 second** startup.

## Requirements

- Windows 10/11 Pro or Server with Hyper-V
- Run as Administrator

## Quick Start

```powershell
cargo build --release

hvkube template register --name win11 --vhdx C:\Templates\win11.vhdx
hvkube pool create --name agents --template win11 --count 3
hvkube pool provision agents --count 3
hvkube pool prepare agents

# Resume VM (~3s)
hvkube vm resume agents-0

# HTTP API
hvkube serve --port 8080
```

## API

```
POST /api/v1/acquire {"pool_name": "agents"}
  → {"vm_name": "...", "ip_address": "...", "mcp_endpoint": "http://...:8080/mcp"}

POST /api/v1/vms/:name/release
POST /api/v1/vms/:name/resume
```
