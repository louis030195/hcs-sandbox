# HyperV-Kube

Lightweight Kubernetes-like VM orchestrator for UI automation agents. Uses Hyper-V Save/Resume for **2-5 second** VM startup.

## Features

- **Blazing fast** - VMs resume from saved state in 2-5 seconds
- **Simple** - Uses Hyper-V directly via PowerShell (no HCS complexity)
- **Pool management** - Pre-warm VMs for instant availability
- **Template-based** - Clone VMs from golden images using differencing disks
- **SQLite state** - Lightweight persistent state tracking

## Requirements

- Windows 10/11 Pro or Windows Server
- Hyper-V enabled: `Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All`
- Run as Administrator

## Quick Start

```powershell
# Build
cargo build --release

# Register a template (your golden VHDX image)
hvkube template register --name win11-chrome --vhdx C:\Templates\win11.vhdx

# Create a pool
hvkube pool create --name browser-pool --template win11-chrome --count 3

# Provision VMs (creates differencing disks)
hvkube pool provision browser-pool --count 3

# Prepare VMs (first boot, checkpoint, save state)
hvkube pool prepare browser-pool

# Now VMs are ready for instant resume!
hvkube vm resume browser-pool-0
# VM ready in ~3 seconds!

# Save back when done
hvkube vm save browser-pool-0
```

## CLI Commands

```
hvkube template register   Register a golden image
hvkube template list       List templates
hvkube pool create         Create a VM pool
hvkube pool provision      Create VMs for pool
hvkube pool prepare        Boot and save all VMs
hvkube pool status         Show pool status
hvkube vm list             List all VMs
hvkube vm resume <name>    Resume saved VM (fast!)
hvkube vm save <name>      Save VM state
hvkube vm reset <name>     Reset to clean checkpoint
hvkube vm console <name>   Open Hyper-V console
hvkube reconcile           Sync DB with Hyper-V
```

## Library Usage

```rust
use hyperv_kube::{Orchestrator, models::{Template, VMPool}};

fn main() -> hyperv_kube::Result<()> {
    let orch = Orchestrator::new()?;

    // Register template
    let template = Template::new("win11", r"C:\Templates\win11.vhdx");
    orch.register_template(template)?;

    // Create pool and provision VMs
    let pool = VMPool::new("agents", "tmpl-xxx").with_count(3);
    orch.create_pool(pool)?;
    orch.provision_pool("pool-xxx", 3)?;

    // Prepare VMs (first boot + save)
    // ... or use CLI: hvkube pool prepare agents

    // Acquire VM (resumes in 2-5 seconds!)
    let vm = orch.acquire_vm("pool-xxx")?;
    println!("VM ready at: {}", vm.ip_address.unwrap());

    // Do UI automation work...

    // Release back to pool
    orch.release_vm(&vm.id, false)?;

    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         hvkube CLI                              │
├─────────────────────────────────────────────────────────────────┤
│  Orchestrator                                                   │
│    ├── Templates (golden images)                                │
│    ├── Pools (groups of VMs)                                    │
│    └── VMs (instances)                                          │
├─────────────────────────────────────────────────────────────────┤
│  SQLite DB (state.db)          Hyper-V (PowerShell)             │
│    ├── templates               Start-VM, Save-VM                │
│    ├── pools                   New-VM, New-VHD                  │
│    ├── vms                     Checkpoint-VM                    │
│    └── agents                  Get-VMNetworkAdapter             │
└─────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│  Hyper-V VMs                                                    │
│    ┌───────────┐  ┌───────────┐  ┌───────────┐                 │
│    │ agent-0   │  │ agent-1   │  │ agent-2   │                 │
│    │ (Saved)   │  │ (Running) │  │ (Saved)   │                 │
│    │ ~3s start │  │           │  │ ~3s start │                 │
│    └───────────┘  └───────────┘  └───────────┘                 │
└─────────────────────────────────────────────────────────────────┘
```

## Why Hyper-V Save/Resume?

| | Full Boot | Save/Resume |
|---|-----------|-------------|
| Time | 30-60s | 2-5s |
| State | Fresh | Preserved |
| Apps | Need to relaunch | Already running |

The trick: Pre-boot VMs once, save state. Resume instantly when needed.

## Creating a Golden Image

1. Create a new VM in Hyper-V Manager
2. Install Windows 11/Server
3. Install required software (Chrome, Node, etc.)
4. Configure auto-login
5. Shutdown
6. Register the VHDX with hvkube

```powershell
hvkube template register --name win11-chrome --vhdx "C:\VMs\golden\disk.vhdx" --memory 4096 --cpus 2
```

## License

MIT
