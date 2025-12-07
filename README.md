# HyperV-Kube

VM orchestrator using Hyper-V Save/Resume for **sub-second** startup (~770ms).

## Local Setup

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

One-liner deployment:

```bash
# Bash
./deploy/deploy.sh hyperv-kube-rg eastus 'YourP@ssw0rd123!'

# PowerShell
.\deploy\deploy.ps1 -ResourceGroup hyperv-kube-rg -Location eastus -Password 'YourP@ssw0rd123!'
```

Then RDP into the VM and run:

```powershell
# Downloads hvkube + Windows 11 template (~20GB)
irm https://raw.githubusercontent.com/louis030195/hcs-sandbox/hyperv-impl/deploy/vm-setup.ps1 | iex
```

## API

```
POST /api/v1/acquire {"pool_name": "agents"}
POST /api/v1/vms/:name/release
POST /api/v1/vms/:name/resume
GET  /health
```

## With Terminator

```bash
npx @mediar-ai/cli mcp exec --url "http://<VM_IP>:8080/mcp" run_command '{"run": "notepad.exe"}'
```
