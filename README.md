# HCS Sandbox

Low-level Windows sandbox orchestrator using Host Compute Service (HCS) APIs - the same APIs that power Windows Sandbox and Docker on Windows.

## Features

- **No Docker required** - Uses HCS directly with dynamic base OS layers
- **Fast boot** - Sandboxes start in 2-5 seconds vs minutes for VMs
- **UI Automation ready** - HyperV isolation with GPU passthrough
- **High density** - Run 10-20 sandboxes per host
- **Rust API** - Type-safe orchestrator with builder pattern

## Requirements

- Windows 10/11 Pro or Enterprise
- Hyper-V enabled (`Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All`)
- Containers feature enabled (`Enable-WindowsOptionalFeature -Online -FeatureName Containers -All`)
- Run as Administrator

## Quick Start

```powershell
# Clone and build
git clone https://github.com/louis030195/hcs-sandbox
cd hcs-sandbox
cargo build --release

# Run (as Administrator)
.\target\release\hcs-sandbox.exe
```

## Library Usage

```rust
use hcs_sandbox::{Orchestrator, SandboxConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orch = Orchestrator::new()?;

    let config = SandboxConfig::builder()
        .name("my-sandbox")
        .memory_mb(4096)
        .cpu_count(2)
        .gpu_enabled(true)
        .build();

    // Create and start
    let id = orch.create_and_start(config)?;

    // List sandboxes
    for info in orch.list_with_state() {
        println!("{} - {}", info.name, info.state);
    }

    // Cleanup
    orch.destroy(&id)?;
    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│            HOST WINDOWS (Admin)                     │
├─────────────────────────────────────────────────────┤
│  Orchestrator                                       │
│    └── Sandbox[] (up to 20)                         │
│          ├── HCS ComputeSystem (HyperV isolation)   │
│          ├── GPU passthrough                        │
│          └── Desktop session                        │
├─────────────────────────────────────────────────────┤
│  HCS APIs (computecore.dll)                         │
│    ├── HcsCreateComputeSystem                       │
│    ├── HcsStartComputeSystem                        │
│    └── HcsSetupBaseOSLayer (no Docker needed!)      │
└─────────────────────────────────────────────────────┘
```

## Why HCS over Docker?

| Feature | Docker (Process Isolation) | HCS Sandbox (HyperV) |
|---------|---------------------------|----------------------|
| Desktop Session | ❌ No | ✅ Yes |
| UI Automation | ❌ No | ✅ Yes |
| GPU Access | ❌ No | ✅ Yes |
| Boot Time | ~10s | ~3s |
| Base Image | Docker Hub | Host OS (dynamic) |

## License

MIT
