# HyperV-Kube: Lightweight VM Orchestrator for UI Automation Agents

## Overview

Replace HCS complexity with direct Hyper-V VM management using **Save/Resume** for 2-5 second "boot" times. Simpler, more reliable, battle-tested.

## Core Insight

```
HCS Approach (complex):
  Create base layer → Create writable layer → Create compute system → Start
  Time: Variable, requires layer setup, can fail mysteriously

Hyper-V Save/Resume Approach (simple):
  Pre-create VM once → Save state → Resume when needed
  Time: 2-5 seconds, reliable, well-documented
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         hyperv-kube                                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐  │
│  │  CLI (hvkube)   │  │   HTTP API      │  │   Rust Library          │  │
│  │                 │  │   (optional)    │  │   (core)                │  │
│  └────────┬────────┘  └────────┬────────┘  └────────────┬────────────┘  │
│           │                    │                        │               │
│           └────────────────────┼────────────────────────┘               │
│                                │                                        │
│                                ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                      Orchestrator                                │   │
│  │                                                                  │   │
│  │  - VM Pool management (warm VMs ready to resume)                 │   │
│  │  - Template management (golden images)                           │   │
│  │  - State tracking (SQLite)                                       │   │
│  │  - Agent scheduling                                              │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                │                                        │
│                                ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                      HyperV Backend                              │   │
│  │                                                                  │   │
│  │  PowerShell/WMI calls:                                           │   │
│  │  - New-VM, Start-VM, Stop-VM                                     │   │
│  │  - Save-VM, Resume-VM (the magic!)                               │   │
│  │  - Checkpoint-VM, Restore-VMCheckpoint                           │   │
│  │  - Get-VM, Get-VMNetworkAdapter                                  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                │                                        │
│                                ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │                      Display Backend                             │   │
│  │                                                                  │   │
│  │  - vmconnect.exe (Hyper-V console)                               │   │
│  │  - RDP (if guest has RDP enabled)                                │   │
│  │  - Screenshot via Hyper-V integration services                   │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Hyper-V                                         │
│                                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   agent-1   │  │   agent-2   │  │   agent-3   │  │  template   │    │
│  │   (Saved)   │  │  (Running)  │  │   (Saved)   │  │   (Off)     │    │
│  │             │  │             │  │             │  │             │    │
│  │  .vmrs file │  │  Active     │  │  .vmrs file │  │  Golden     │    │
│  │  (state)    │  │             │  │  (state)    │  │  Image      │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## VM Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        VM Lifecycle                                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  1. TEMPLATE CREATION (one-time, manual or scripted)                    │
│     ───────────────────────────────────────────────────────────         │
│     - Install Windows 11/Server                                         │
│     - Install required software (Chrome, Node, Python, etc.)            │
│     - Configure auto-login                                              │
│     - Install MCP agent / automation tools                              │
│     - Sysprep (optional, for unique SIDs)                               │
│     - Shutdown → This is the "golden image"                             │
│                                                                         │
│  2. VM PROVISIONING (from template)                                     │
│     ───────────────────────────────────────────────────────────         │
│     a) Clone VHDX (differencing disk for COW)                           │
│        New-VHD -Path "agent-1.vhdx" -ParentPath "template.vhdx"         │
│                                                                         │
│     b) Create VM pointing to differencing disk                          │
│        New-VM -Name "agent-1" -VHDPath "agent-1.vhdx"                   │
│                                                                         │
│     c) First boot (30-60 sec) - let Windows settle                      │
│        Start-VM -Name "agent-1"                                         │
│        # Wait for desktop ready                                         │
│                                                                         │
│     d) SAVE STATE (creates .vmrs file)                                  │
│        Save-VM -Name "agent-1"                                          │
│        # VM is now in "Saved" state, ready for instant resume           │
│                                                                         │
│  3. AGENT EXECUTION (fast path - 2-5 seconds)                           │
│     ───────────────────────────────────────────────────────────         │
│     a) Resume saved VM                                                  │
│        Start-VM -Name "agent-1"  # Resumes from saved state             │
│        # Desktop appears in 2-5 seconds!                                │
│                                                                         │
│     b) Run UI automation task                                           │
│        # Agent does its thing...                                        │
│                                                                         │
│     c) After task: either                                               │
│        - Save-VM (preserve state for next task)                         │
│        - Restore-VMCheckpoint (reset to clean state)                    │
│        - Stop-VM + delete (ephemeral)                                   │
│                                                                         │
│  4. CLEANUP                                                             │
│     ───────────────────────────────────────────────────────────         │
│     - Remove-VM -Name "agent-1" -Force                                  │
│     - Delete VHDX and .vmrs files                                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

## State Transitions

```
                    ┌──────────────────┐
                    │    Template      │
                    │   (Off, VHDX)    │
                    └────────┬─────────┘
                             │ Clone VHDX + New-VM
                             ▼
                    ┌──────────────────┐
                    │      Off         │
                    │   (never run)    │
                    └────────┬─────────┘
                             │ Start-VM (first boot, 30-60s)
                             ▼
                    ┌──────────────────┐
         ┌─────────│    Running       │──────────┐
         │         │   (first boot)   │          │
         │         └────────┬─────────┘          │
         │                  │ Save-VM            │ Stop-VM
         │                  ▼                    ▼
         │         ┌──────────────────┐  ┌──────────────────┐
         │         │     Saved        │  │      Off         │
         │         │  (ready pool)    │  │   (cold start)   │
         │         └────────┬─────────┘  └──────────────────┘
         │                  │ Start-VM (resume, 2-5s!)
         │                  ▼
         │         ┌──────────────────┐
         └────────▶│    Running       │◀─────────┐
                   │  (task active)   │          │
                   └────────┬─────────┘          │
                            │                    │
              ┌─────────────┼─────────────┐      │
              │             │             │      │
              ▼             ▼             ▼      │
        ┌──────────┐  ┌──────────┐  ┌──────────┐│
        │  Save    │  │ Restore  │  │  Stop    ││
        │  (keep   │  │ Checkpoint│  │  (end)  ││
        │  state)  │  │ (reset)  │  │          ││
        └────┬─────┘  └────┬─────┘  └──────────┘│
             │             │                    │
             │             └────────────────────┘
             ▼
        Back to Saved
        (ready for next task)
```

## Data Model

```rust
// models.rs

/// VM template (golden image)
pub struct Template {
    pub id: String,
    pub name: String,
    pub vhdx_path: PathBuf,       // Base VHDX
    pub memory_mb: u64,
    pub cpu_count: u32,
    pub gpu_enabled: bool,
    pub installed_software: Vec<String>,  // Chrome, Node, etc.
    pub created_at: DateTime<Utc>,
}

/// A VM instance (cloned from template)
pub struct VM {
    pub id: String,
    pub name: String,                      // Hyper-V VM name
    pub template_id: String,
    pub state: VMState,
    pub vhdx_path: PathBuf,                // Differencing disk
    pub vmrs_path: Option<PathBuf>,        // Saved state file
    pub ip_address: Option<IpAddr>,
    pub rdp_port: u16,                     // Usually 3389
    pub created_at: DateTime<Utc>,
    pub last_resumed_at: Option<DateTime<Utc>>,
    pub current_agent_id: Option<String>,
}

pub enum VMState {
    Off,           // Never started or stopped
    Running,       // Currently executing
    Saved,         // State saved to disk, ready for fast resume
    Paused,        // Paused in memory (not persisted)
    Error(String), // Something went wrong
}

/// A pool of VMs from same template
pub struct VMPool {
    pub id: String,
    pub name: String,
    pub template_id: String,
    pub desired_count: usize,      // Total VMs to maintain
    pub warm_count: usize,         // VMs to keep in Saved state
    pub max_per_host: usize,
    pub vms: Vec<String>,          // VM IDs in this pool
}

/// An automation task/agent
pub struct Agent {
    pub id: String,
    pub name: String,
    pub vm_id: Option<String>,     // Assigned VM
    pub status: AgentStatus,
    pub task: Task,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<AgentResult>,
}

pub enum AgentStatus {
    Pending,       // Waiting for VM
    Scheduled,     // VM assigned, waiting to start
    Running,       // Executing on VM
    Completed,     // Done successfully
    Failed(String),// Failed with error
    Cancelled,
}

pub struct Task {
    pub workflow: String,          // Workflow name/ID
    pub input: serde_json::Value,  // Task parameters
    pub timeout_seconds: u64,
    pub requires_gpu: bool,
}

pub struct AgentResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub screenshots: Vec<PathBuf>,
    pub duration_seconds: u64,
}
```

## Hyper-V Backend (PowerShell Wrapper)

```rust
// hyperv/mod.rs

use std::process::Command;

pub struct HyperV;

impl HyperV {
    /// List all VMs
    pub fn list_vms() -> Result<Vec<VMInfo>> {
        let output = powershell(r#"
            Get-VM | Select-Object Name, State, MemoryAssigned, Uptime | ConvertTo-Json
        "#)?;
        Ok(serde_json::from_str(&output)?)
    }

    /// Get VM by name
    pub fn get_vm(name: &str) -> Result<Option<VMInfo>> {
        let output = powershell(&format!(r#"
            Get-VM -Name '{}' -ErrorAction SilentlyContinue |
            Select-Object Name, State, MemoryAssigned, Id | ConvertTo-Json
        "#, name))?;
        // Parse...
    }

    /// Create VM from differencing disk
    pub fn create_vm(name: &str, vhdx_path: &str, memory_mb: u64, cpu_count: u32) -> Result<()> {
        powershell(&format!(r#"
            New-VM -Name '{}' -MemoryStartupBytes {}MB -Generation 2 -VHDPath '{}'
            Set-VM -Name '{}' -ProcessorCount {} -AutomaticStartAction Nothing
            Set-VMMemory -VMName '{}' -DynamicMemoryEnabled $true
        "#, name, memory_mb, vhdx_path, name, cpu_count, name))?;
        Ok(())
    }

    /// Create differencing disk (COW clone)
    pub fn create_differencing_disk(parent: &str, child: &str) -> Result<()> {
        powershell(&format!(r#"
            New-VHD -Path '{}' -ParentPath '{}' -Differencing
        "#, child, parent))?;
        Ok(())
    }

    /// Start VM (resumes if saved, cold boots if off)
    pub fn start_vm(name: &str) -> Result<()> {
        powershell(&format!("Start-VM -Name '{}'", name))?;
        Ok(())
    }

    /// Save VM state (fast resume later)
    pub fn save_vm(name: &str) -> Result<()> {
        powershell(&format!("Save-VM -Name '{}'", name))?;
        Ok(())
    }

    /// Stop VM (graceful shutdown)
    pub fn stop_vm(name: &str) -> Result<()> {
        powershell(&format!("Stop-VM -Name '{}' -Force", name))?;
        Ok(())
    }

    /// Delete VM
    pub fn remove_vm(name: &str) -> Result<()> {
        powershell(&format!("Remove-VM -Name '{}' -Force", name))?;
        Ok(())
    }

    /// Create checkpoint (snapshot)
    pub fn checkpoint_vm(name: &str, checkpoint_name: &str) -> Result<()> {
        powershell(&format!(r#"
            Checkpoint-VM -Name '{}' -SnapshotName '{}'
        "#, name, checkpoint_name))?;
        Ok(())
    }

    /// Restore to checkpoint
    pub fn restore_checkpoint(name: &str, checkpoint_name: &str) -> Result<()> {
        powershell(&format!(r#"
            Restore-VMCheckpoint -VMName '{}' -Name '{}' -Confirm:$false
        "#, name, checkpoint_name))?;
        Ok(())
    }

    /// Get VM IP address
    pub fn get_vm_ip(name: &str) -> Result<Option<String>> {
        let output = powershell(&format!(r#"
            (Get-VMNetworkAdapter -VMName '{}').IPAddresses |
            Where-Object {{ $_ -match '^\d+\.\d+\.\d+\.\d+$' }} |
            Select-Object -First 1
        "#, name))?;
        Ok(if output.trim().is_empty() { None } else { Some(output.trim().to_string()) })
    }

    /// Wait for VM to be ready (has IP, RDP responding)
    pub fn wait_for_ready(name: &str, timeout_secs: u64) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Err(Error::Timeout);
            }
            if let Some(ip) = Self::get_vm_ip(name)? {
                // Try TCP connect to RDP port
                if std::net::TcpStream::connect_timeout(
                    &format!("{}:3389", ip).parse()?,
                    std::time::Duration::from_secs(2)
                ).is_ok() {
                    return Ok(());
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    /// Enable Enhanced Session Mode (for better display)
    pub fn enable_enhanced_session(name: &str) -> Result<()> {
        powershell(&format!(r#"
            Set-VM -Name '{}' -EnhancedSessionTransportType HvSocket
        "#, name))?;
        Ok(())
    }

    /// Configure GPU passthrough (if available)
    pub fn add_gpu(name: &str) -> Result<()> {
        powershell(&format!(r#"
            $vm = Get-VM -Name '{}'
            Add-VMGpuPartitionAdapter -VMName '{}'
            Set-VMGpuPartitionAdapter -VMName '{}' -MinPartitionVRAM 80000000 -MaxPartitionVRAM 100000000 -OptimalPartitionVRAM 100000000
        "#, name, name, name))?;
        Ok(())
    }
}

fn powershell(script: &str) -> Result<String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::PowerShell(stderr.to_string()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

## Display/Screen Access

```rust
// display/mod.rs

pub struct Display;

impl Display {
    /// Open vmconnect (Hyper-V console) - for local viewing
    pub fn open_console(vm_name: &str) -> Result<()> {
        Command::new("vmconnect")
            .args(["localhost", vm_name])
            .spawn()?;
        Ok(())
    }

    /// Take screenshot via Hyper-V integration
    /// (Requires integration services in guest)
    pub fn screenshot(vm_name: &str, output_path: &str) -> Result<()> {
        // Option 1: Use Hyper-V thumbnail (low quality but fast)
        powershell(&format!(r#"
            $vm = Get-VM -Name '{}'
            $vm | Get-VMVideo | Export-VMSnapshot -Path '{}'
        "#, vm_name, output_path))?;
        Ok(())
    }

    /// Connect via RDP and screenshot (higher quality)
    pub fn rdp_screenshot(ip: &str, username: &str, password: &str, output_path: &str) -> Result<()> {
        // Use FreeRDP or similar
        Command::new("wfreerdp")
            .args([
                "/v:", ip,
                "/u:", username,
                "/p:", password,
                "/cert:ignore",
                "/screenshot:", output_path,
                "/timeout:5000",
            ])
            .output()?;
        Ok(())
    }

    /// Send keyboard input via RDP
    pub fn send_keys(ip: &str, keys: &str) -> Result<()> {
        // Via RDP or PowerShell remoting
        todo!()
    }

    /// Send mouse click via RDP
    pub fn send_click(ip: &str, x: i32, y: i32) -> Result<()> {
        todo!()
    }
}
```

## Orchestrator

```rust
// orchestrator.rs

pub struct Orchestrator {
    db: Database,           // SQLite for state
    hyperv: HyperV,
    config: OrchestratorConfig,
}

impl Orchestrator {
    /// Create VMs for a pool, save them, ready for use
    pub async fn provision_pool(&self, pool: &VMPool) -> Result<()> {
        let template = self.db.get_template(&pool.template_id)?;

        for i in 0..pool.desired_count {
            let vm_name = format!("{}-{}", pool.name, i);
            let vhdx_path = format!("C:\\VMs\\{}\\disk.vhdx", vm_name);

            // Create differencing disk
            HyperV::create_differencing_disk(&template.vhdx_path, &vhdx_path)?;

            // Create VM
            HyperV::create_vm(&vm_name, &vhdx_path, template.memory_mb, template.cpu_count)?;

            if template.gpu_enabled {
                HyperV::add_gpu(&vm_name)?;
            }

            // First boot (slow, 30-60s)
            HyperV::start_vm(&vm_name)?;
            HyperV::wait_for_ready(&vm_name, 120)?;

            // Create "clean" checkpoint
            HyperV::checkpoint_vm(&vm_name, "clean")?;

            // Save state (now ready for fast resume)
            HyperV::save_vm(&vm_name)?;

            // Record in DB
            self.db.insert_vm(VM {
                name: vm_name,
                state: VMState::Saved,
                // ...
            })?;
        }

        Ok(())
    }

    /// Get a ready VM from pool (resumes in 2-5s)
    pub async fn acquire_vm(&self, pool_id: &str) -> Result<VM> {
        // Find a saved VM in the pool
        let vm = self.db.find_saved_vm_in_pool(pool_id)?
            .ok_or(Error::NoVMAvailable)?;

        // Resume it
        HyperV::start_vm(&vm.name)?;

        // Wait for ready (should be fast, 2-5s)
        HyperV::wait_for_ready(&vm.name, 30)?;

        // Update state
        self.db.update_vm_state(&vm.id, VMState::Running)?;

        Ok(vm)
    }

    /// Release VM back to pool
    pub async fn release_vm(&self, vm_id: &str, reset: bool) -> Result<()> {
        let vm = self.db.get_vm(vm_id)?;

        if reset {
            // Reset to clean checkpoint
            HyperV::restore_checkpoint(&vm.name, "clean")?;
        }

        // Save state for next use
        HyperV::save_vm(&vm.name)?;

        self.db.update_vm_state(vm_id, VMState::Saved)?;
        self.db.clear_vm_agent(vm_id)?;

        Ok(())
    }

    /// Schedule an agent on a VM
    pub async fn schedule_agent(&self, agent: &Agent) -> Result<()> {
        // Find appropriate pool
        let pool = self.find_pool_for_task(&agent.task)?;

        // Acquire VM
        let vm = self.acquire_vm(&pool.id).await?;

        // Bind agent to VM
        self.db.update_agent_vm(&agent.id, &vm.id)?;
        self.db.update_agent_status(&agent.id, AgentStatus::Running)?;

        // Agent execution happens externally (MCP, etc.)
        // This just handles VM lifecycle

        Ok(())
    }
}
```

## CLI

```rust
// bin/hvkube.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hvkube")]
#[command(about = "Lightweight Hyper-V orchestrator for UI automation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Template management
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
    /// VM pool management
    Pool {
        #[command(subcommand)]
        action: PoolAction,
    },
    /// Individual VM operations
    Vm {
        #[command(subcommand)]
        action: VmAction,
    },
    /// Agent/task management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
}

#[derive(Subcommand)]
enum VmAction {
    /// List VMs
    List {
        #[arg(short, long)]
        pool: Option<String>,
    },
    /// Resume a saved VM
    Resume { name: String },
    /// Save VM state
    Save { name: String },
    /// Reset VM to clean checkpoint
    Reset { name: String },
    /// Open display console
    Console { name: String },
    /// Take screenshot
    Screenshot {
        name: String,
        #[arg(short, long, default_value = "screenshot.png")]
        output: String,
    },
    /// Get VM info
    Info { name: String },
}

// Usage:
// hvkube template create --name win11-chrome --vhdx C:\Templates\win11.vhdx
// hvkube pool create --name browser-pool --template win11-chrome --count 5
// hvkube vm list
// hvkube vm resume agent-1
// hvkube vm console agent-1
// hvkube vm screenshot agent-1 -o screen.png
// hvkube agent submit --pool browser-pool --workflow book-flight --input '{"dest":"NYC"}'
```

## Edge Cases & Error Handling

### 1. Saved State Corruption

```rust
/// If saved state is corrupted, fall back to checkpoint restore
pub async fn resume_with_fallback(&self, vm_id: &str) -> Result<()> {
    let vm = self.db.get_vm(vm_id)?;

    // Try fast resume first
    match HyperV::start_vm(&vm.name) {
        Ok(_) => {
            if HyperV::wait_for_ready(&vm.name, 30).is_ok() {
                return Ok(());
            }
        }
        Err(e) => {
            tracing::warn!(vm = %vm.name, error = %e, "saved state resume failed");
        }
    }

    // Fallback: restore checkpoint and cold boot
    tracing::info!(vm = %vm.name, "falling back to checkpoint restore");
    HyperV::stop_vm(&vm.name)?;  // Force stop if stuck
    HyperV::restore_checkpoint(&vm.name, "clean")?;
    HyperV::start_vm(&vm.name)?;
    HyperV::wait_for_ready(&vm.name, 120)?;  // Longer timeout for cold boot
    HyperV::save_vm(&vm.name)?;  // Re-create saved state

    Ok(())
}
```

### 2. VM IP Changes After Resume

```rust
/// VMs may get different IPs after resume if DHCP lease expired
pub async fn ensure_vm_ip(&self, vm_id: &str) -> Result<String> {
    let vm = self.db.get_vm(vm_id)?;

    // Check current IP
    let ip = HyperV::get_vm_ip(&vm.name)?
        .ok_or(Error::NoIP)?;

    // Update DB if changed
    if Some(&ip) != vm.ip_address.as_ref().map(|i| &i.to_string()) {
        tracing::info!(vm = %vm.name, old = ?vm.ip_address, new = %ip, "IP changed");
        self.db.update_vm_ip(vm_id, &ip)?;
    }

    Ok(ip)
}
```

### 3. Concurrent VM Access

```rust
/// Prevent same VM from being acquired twice
pub async fn acquire_vm(&self, pool_id: &str) -> Result<VM> {
    // Use DB transaction with row locking
    let mut tx = self.db.begin_transaction()?;

    let vm = tx.find_and_lock_saved_vm(pool_id)?
        .ok_or(Error::NoVMAvailable)?;

    tx.update_vm_state(&vm.id, VMState::Running)?;
    tx.commit()?;

    // Now safe to resume
    HyperV::start_vm(&vm.name)?;
    // ...
}
```

### 4. Host Resource Exhaustion

```rust
/// Check host resources before provisioning
pub fn check_host_capacity(&self, needed_vms: usize) -> Result<()> {
    let host_info = HyperV::get_host_info()?;

    let required_memory = needed_vms as u64 * self.config.default_memory_mb;
    let available = host_info.available_memory_mb;

    if required_memory > available * 80 / 100 {  // 80% threshold
        return Err(Error::InsufficientMemory {
            required: required_memory,
            available,
        });
    }

    Ok(())
}
```

### 5. Cleanup on Crash

```rust
/// On startup, reconcile DB state with actual Hyper-V state
pub async fn reconcile(&self) -> Result<()> {
    let db_vms = self.db.list_all_vms()?;
    let hyperv_vms = HyperV::list_vms()?;

    for db_vm in db_vms {
        let actual = hyperv_vms.iter().find(|v| v.name == db_vm.name);

        match actual {
            Some(hv_vm) => {
                // Sync state
                let actual_state = match hv_vm.state.as_str() {
                    "Running" => VMState::Running,
                    "Saved" => VMState::Saved,
                    "Off" => VMState::Off,
                    _ => VMState::Error("unknown".into()),
                };
                if db_vm.state != actual_state {
                    self.db.update_vm_state(&db_vm.id, actual_state)?;
                }
            }
            None => {
                // VM was deleted outside our control
                tracing::warn!(vm = %db_vm.name, "VM not found in Hyper-V, marking as error");
                self.db.update_vm_state(&db_vm.id, VMState::Error("not found".into()))?;
            }
        }
    }

    Ok(())
}
```

### 6. Guest Not Responding After Resume

```rust
/// Sometimes guest needs a moment after resume
pub async fn wait_for_guest_ready(&self, vm_name: &str) -> Result<()> {
    // First wait for VM to be running
    HyperV::wait_for_ready(vm_name, 30)?;

    // Then wait for guest services (integration services heartbeat)
    let start = std::time::Instant::now();
    loop {
        if start.elapsed().as_secs() > 60 {
            return Err(Error::GuestNotResponding);
        }

        let status = powershell(&format!(r#"
            (Get-VMIntegrationService -VMName '{}' -Name 'Heartbeat').PrimaryStatusDescription
        "#, vm_name))?;

        if status.trim() == "OK" {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
```

## Infrastructure Deployment (Azure)

### Terraform for Nested Hyper-V Host

```hcl
# main.tf

# VM that supports nested virtualization
resource "azurerm_windows_virtual_machine" "hyperv_host" {
  name                = "hyperv-kube-host"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location

  # Must be v3+ for nested virtualization
  size                = "Standard_D16s_v3"  # 16 vCPU, 64GB RAM

  admin_username      = var.admin_username
  admin_password      = var.admin_password

  network_interface_ids = [azurerm_network_interface.main.id]

  os_disk {
    caching              = "ReadWrite"
    storage_account_type = "Premium_LRS"
    disk_size_gb         = 256
  }

  source_image_reference {
    publisher = "MicrosoftWindowsServer"
    offer     = "WindowsServer"
    sku       = "2022-datacenter-g2"
    version   = "latest"
  }
}

# Data disk for VM storage
resource "azurerm_managed_disk" "vm_storage" {
  name                = "hyperv-kube-vms"
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name

  storage_account_type = "Premium_LRS"
  disk_size_gb         = 512
  create_option        = "Empty"
}

resource "azurerm_virtual_machine_data_disk_attachment" "vm_storage" {
  managed_disk_id    = azurerm_managed_disk.vm_storage.id
  virtual_machine_id = azurerm_windows_virtual_machine.hyperv_host.id
  lun                = 0
  caching            = "ReadWrite"
}

# Setup script
resource "azurerm_virtual_machine_extension" "setup" {
  name                 = "setup-hyperv"
  virtual_machine_id   = azurerm_windows_virtual_machine.hyperv_host.id
  publisher            = "Microsoft.Compute"
  type                 = "CustomScriptExtension"
  type_handler_version = "1.10"

  settings = jsonencode({
    commandToExecute = <<-EOT
      powershell -Command "
        # Install Hyper-V
        Install-WindowsFeature -Name Hyper-V -IncludeManagementTools -Restart

        # Initialize data disk
        Get-Disk | Where-Object PartitionStyle -eq 'RAW' |
          Initialize-Disk -PartitionStyle GPT -PassThru |
          New-Partition -AssignDriveLetter -UseMaximumSize |
          Format-Volume -FileSystem NTFS -NewFileSystemLabel 'VMs' -Confirm:$false

        # Create VM storage directory
        New-Item -Path 'D:\VMs' -ItemType Directory -Force
        New-Item -Path 'D:\Templates' -ItemType Directory -Force

        # Download hyperv-kube binary
        Invoke-WebRequest -Uri '${var.hyperv_kube_url}' -OutFile 'C:\hyperv-kube\hvkube.exe'

        # Install as service (optional)
        # ...
      "
    EOT
  })
}

# Network security group
resource "azurerm_network_security_group" "main" {
  name                = "hyperv-kube-nsg"
  location            = azurerm_resource_group.main.location
  resource_group_name = azurerm_resource_group.main.name

  # RDP to host
  security_rule {
    name                       = "RDP"
    priority                   = 100
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "3389"
    source_address_prefix      = var.allowed_ip
    destination_address_prefix = "*"
  }

  # API server
  security_rule {
    name                       = "API"
    priority                   = 110
    direction                  = "Inbound"
    access                     = "Allow"
    protocol                   = "Tcp"
    source_port_range          = "*"
    destination_port_range     = "8080"
    source_address_prefix      = var.allowed_ip
    destination_address_prefix = "*"
  }
}
```

### Packer for Golden Image

```hcl
# template.pkr.hcl

source "hyperv-iso" "windows11" {
  iso_url          = "path/to/windows11.iso"
  iso_checksum     = "sha256:..."
  generation       = 2
  memory           = 4096
  cpus             = 2
  disk_size        = 40960
  switch_name      = "Default Switch"

  communicator     = "winrm"
  winrm_username   = "packer"
  winrm_password   = "packer"
  winrm_timeout    = "30m"

  shutdown_command = "shutdown /s /t 10 /f"
}

build {
  sources = ["source.hyperv-iso.windows11"]

  # Install software
  provisioner "powershell" {
    inline = [
      # Chrome
      "choco install googlechrome -y",

      # Node.js
      "choco install nodejs-lts -y",

      # Python
      "choco install python3 -y",

      # Configure auto-login
      "$RegPath = 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon'",
      "Set-ItemProperty -Path $RegPath -Name AutoAdminLogon -Value 1",
      "Set-ItemProperty -Path $RegPath -Name DefaultUsername -Value 'agent'",
      "Set-ItemProperty -Path $RegPath -Name DefaultPassword -Value 'AgentPassword123!'",

      # Disable UAC
      "Set-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System' -Name EnableLUA -Value 0",

      # Enable RDP
      "Set-ItemProperty -Path 'HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server' -Name fDenyTSConnections -Value 0",
      "Enable-NetFirewallRule -DisplayGroup 'Remote Desktop'",
    ]
  }

  # Export as template VHDX
  post-processor "compress" {
    output = "template-win11.vhdx"
  }
}
```

## File Structure

```
hyperv-kube/
├── Cargo.toml
├── PLAN.md                      # This file
├── src/
│   ├── lib.rs                   # Library exports
│   ├── error.rs                 # Error types
│   ├── models/
│   │   ├── mod.rs
│   │   ├── vm.rs                # VM, VMState
│   │   ├── pool.rs              # VMPool
│   │   ├── template.rs          # Template
│   │   └── agent.rs             # Agent, Task
│   ├── hyperv/
│   │   ├── mod.rs
│   │   ├── commands.rs          # PowerShell wrappers
│   │   └── types.rs             # Hyper-V specific types
│   ├── display/
│   │   ├── mod.rs
│   │   ├── screenshot.rs        # Screen capture
│   │   └── input.rs             # Keyboard/mouse injection
│   ├── orchestrator.rs          # Main orchestration logic
│   ├── scheduler.rs             # Agent scheduling
│   └── db.rs                    # SQLite state storage
├── src/bin/
│   └── hvkube.rs                # CLI binary
├── tests/
│   ├── integration/
│   │   ├── vm_lifecycle.rs
│   │   └── pool_operations.rs
│   └── unit/
│       └── models.rs
└── infra/
    ├── terraform/
    │   ├── main.tf
    │   ├── variables.tf
    │   └── outputs.tf
    └── packer/
        └── template.pkr.hcl
```

## Summary: HCS vs Hyper-V Approach

| Aspect | HCS (old) | Hyper-V (new) |
|--------|-----------|---------------|
| Complexity | High (layer APIs, JSON schemas) | Low (PowerShell) |
| Boot time | 2-5s (but setup complex) | 2-5s (Save/Resume) |
| Reliability | Can fail mysteriously | Battle-tested, stable |
| Documentation | Sparse, undocumented | Extensive, well-known |
| Tooling | Custom FFI bindings | PowerShell + WMI |
| Debugging | Difficult | Easy (Hyper-V Manager) |
| State management | Complex (layers, VHDs) | Simple (Save/Resume) |
| Multi-instance | 10-20 (HCS limit) | Limited by resources |

**Verdict**: Use Hyper-V directly. Same fast resume capability, much simpler implementation, better reliability.

## Next Steps

1. [ ] Rename package to `hyperv-kube`
2. [ ] Remove HCS code, add Hyper-V PowerShell wrapper
3. [ ] Implement basic VM lifecycle (create, start, save, resume, stop)
4. [ ] Add SQLite state storage
5. [ ] Implement pool management
6. [ ] Add CLI commands
7. [ ] Test Save/Resume timing
8. [ ] Add display/screenshot support
9. [ ] Integration tests
10. [ ] Azure deployment terraform
