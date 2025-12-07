# HyperV-Kube

VM orchestrator using Hyper-V Save/Resume for **sub-second** VM startup (~770ms).

## Requirements

- Windows 10/11 Pro with Hyper-V enabled
- Administrator privileges

## Build

```powershell
cargo build --release
```

## Setup Template

1. Download [Windows 11 Dev VM](https://aka.ms/windev_VM_hyperv) or create your own
2. Extract the VHDX
3. Register:

```powershell
hvkube template register --name win11 --vhdx C:\path\to\win11.vhdx
```

## Usage

```powershell
hvkube pool create --name agents --template win11 --count 3
hvkube pool provision agents --count 3
hvkube pool prepare agents  # boots, checkpoints, saves

hvkube vm resume agents-0   # ~770ms
hvkube vm save agents-0
hvkube vm reset agents-0    # restore checkpoint
```

## HTTP API

```powershell
hvkube serve --port 8080
```

```
POST /api/v1/acquire {"pool_name": "agents"}
POST /api/v1/vms/:name/release
POST /api/v1/vms/:name/resume
```

## With Terminator

Install [terminator](https://github.com/mediar-ai/terminator) in VM, then:

```bash
npx @mediar-ai/cli mcp exec --url "http://<IP>:8080/mcp" run_command '{"run": "notepad.exe"}'
```
