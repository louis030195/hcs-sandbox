# hyperv-kube

Hyper-V VM orchestrator with **sub-second** startup (~770ms) via Save/Resume.

## Quick Start

```powershell
cargo build --release

hvkube template register --name win11 --vhdx C:\path\to\win11.vhdx
hvkube pool create --name agents --template win11 --count 3
hvkube pool provision agents --count 3
hvkube pool prepare agents

hvkube vm resume agents-0   # ~770ms
hvkube serve --port 8080
```

## Deploy to Azure

```bash
./deploy/deploy.sh hyperv-kube-rg eastus 'YourP@ssw0rd!'
```

Then RDP in, copy `hvkube.exe`, and download [Windows 11 Dev VM](https://aka.ms/windev_VM_hyperv).

## API

```
POST /api/v1/acquire {"pool_name": "agents"}
POST /api/v1/vms/:name/release
POST /api/v1/vms/:name/resume
GET  /health
```
