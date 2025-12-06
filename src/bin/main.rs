//! HCS Sandbox CLI

use clap::{Parser, Subcommand};
use hcs_sandbox::SandboxConfig;
use hcs_sandbox::hcs::compute;
use std::path::Path;

#[derive(Parser)]
#[command(name = "hcs-sandbox")]
#[command(about = "Windows sandbox orchestrator using HCS APIs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new sandbox (raw HCS - requires base OS layer)
    Create {
        /// Sandbox name
        #[arg(short, long)]
        name: String,
        /// Memory in MB (default: 4096)
        #[arg(short, long, default_value = "4096")]
        memory: u64,
        /// CPU count (default: 2)
        #[arg(short, long, default_value = "2")]
        cpus: u32,
        /// Disable GPU passthrough
        #[arg(long)]
        no_gpu: bool,
    },
    /// Start a sandbox
    Start {
        /// Sandbox name or ID
        name: String,
    },
    /// Stop a sandbox
    Stop {
        /// Sandbox name or ID
        name: String,
    },
    /// List all sandboxes
    List,
    /// Destroy a sandbox
    Destroy {
        /// Sandbox name or ID
        name: String,
    },
    /// Show HCS service info
    Info,
    /// Launch Windows Sandbox with custom config (easy mode - works out of the box)
    Run {
        /// Memory in MB (default: 4096)
        #[arg(short, long, default_value = "4096")]
        memory: u64,
        /// Disable GPU (vGPU)
        #[arg(long)]
        no_gpu: bool,
        /// Disable networking
        #[arg(long)]
        no_network: bool,
        /// Map a host folder into sandbox (format: host_path or host_path:sandbox_path)
        #[arg(short, long)]
        folder: Option<String>,
        /// Command to run on startup
        #[arg(short, long)]
        cmd: Option<String>,
        /// Keep .wsb config file after sandbox closes
        #[arg(long)]
        keep_config: bool,
    },
    /// Launch raw HCS container (supports multiple concurrent instances)
    Hcs {
        /// Sandbox name (must be unique)
        #[arg(short, long)]
        name: String,
        /// Memory in MB (default: 4096)
        #[arg(short, long, default_value = "4096")]
        memory: u64,
        /// CPU count (default: 2)
        #[arg(short, long, default_value = "2")]
        cpus: u32,
        /// Disable GPU passthrough
        #[arg(long)]
        no_gpu: bool,
    },
    /// Show available base layers
    Layers,
    /// Get properties of a compute system
    Props {
        /// Compute system ID or name
        id: String,
    },
    /// Clone from existing sandbox storage
    Clone {
        /// Sandbox name (must be unique)
        #[arg(short, long)]
        name: String,
        /// Existing sandbox storage ID (from 'layers' command)
        #[arg(short, long)]
        storage: String,
        /// Copy VHDX to private storage (required for multiple sandboxes)
        #[arg(long)]
        copy: bool,
    },
    /// Create sandbox with fresh VHDX (no copy from existing storage)
    New {
        /// Sandbox name (must be unique)
        #[arg(short, long)]
        name: String,
        /// Memory in MB (default: 4096)
        #[arg(short, long, default_value = "4096")]
        memory: u64,
        /// CPU count (default: 2)
        #[arg(short, long, default_value = "2")]
        cpus: u32,
    },
    /// Test minimal HCS configuration
    Test {
        /// Sandbox name
        #[arg(short, long)]
        name: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Create { name, memory, cpus, no_gpu }) => {
            cmd_create(&name, memory, cpus, !no_gpu)?;
        }
        Some(Commands::Start { name }) => {
            cmd_start(&name)?;
        }
        Some(Commands::Stop { name }) => {
            cmd_stop(&name)?;
        }
        Some(Commands::List) => {
            cmd_list()?;
        }
        Some(Commands::Destroy { name }) => {
            cmd_destroy(&name)?;
        }
        Some(Commands::Info) => {
            cmd_info()?;
        }
        Some(Commands::Run { memory, no_gpu, no_network, folder, cmd, keep_config }) => {
            cmd_run(memory, !no_gpu, !no_network, folder, cmd, keep_config)?;
        }
        Some(Commands::Hcs { name, memory, cpus, no_gpu }) => {
            cmd_hcs(&name, memory, cpus, !no_gpu)?;
        }
        Some(Commands::Layers) => {
            cmd_layers()?;
        }
        Some(Commands::Props { id }) => {
            cmd_props(&id)?;
        }
        Some(Commands::Clone { name, storage, copy }) => {
            cmd_clone(&name, &storage, copy)?;
        }
        Some(Commands::New { name, memory, cpus }) => {
            cmd_new(&name, memory, cpus)?;
        }
        Some(Commands::Test { name }) => {
            cmd_test(&name)?;
        }
        None => {
            cmd_info()?;
        }
    }

    Ok(())
}

fn cmd_create(name: &str, memory: u64, cpus: u32, gpu: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating sandbox '{}'...", name);
    println!("  Memory: {} MB", memory);
    println!("  CPUs: {}", cpus);
    println!("  GPU: {}", if gpu { "enabled" } else { "disabled" });

    let config = SandboxConfig::builder()
        .name(name)
        .memory_mb(memory)
        .cpu_count(cpus)
        .gpu_enabled(gpu)
        .build();

    // Generate HCS config
    let hcs_config = config.to_hcs_config();
    let config_json = serde_json::to_string_pretty(&hcs_config)?;
    
    println!("\nHCS Configuration:");
    println!("{}", config_json);

    // Try to create the compute system
    println!("\nCreating HCS compute system...");
    
    match hcs_sandbox::hcs::ComputeSystem::create(name, &serde_json::to_string(&hcs_config)?) {
        Ok(cs) => {
            println!("Created compute system: {}", cs.id());
            println!("\nNote: Sandbox created but not started.");
            println!("Run: hcs-sandbox start {}", name);
        }
        Err(e) => {
            println!("Failed to create: {}", e);
            println!("\nThis is expected - we need a base OS layer first.");
            println!("The HCS config above shows what would be created.");
        }
    }

    Ok(())
}

fn cmd_start(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting sandbox '{}'...", name);
    
    match hcs_sandbox::hcs::ComputeSystem::open(name) {
        Ok(cs) => {
            cs.start()?;
            println!("Sandbox '{}' started!", name);
        }
        Err(e) => {
            println!("Failed to start: {}", e);
        }
    }

    Ok(())
}

fn cmd_stop(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Stopping sandbox '{}'...", name);
    
    match hcs_sandbox::hcs::ComputeSystem::open(name) {
        Ok(cs) => {
            cs.terminate()?;
            println!("Sandbox '{}' stopped!", name);
        }
        Err(e) => {
            println!("Failed to stop: {}", e);
        }
    }

    Ok(())
}

fn cmd_list() -> Result<(), Box<dyn std::error::Error>> {
    println!("Listing compute systems...\n");
    
    match compute::enumerate_compute_systems(None) {
        Ok(systems) => {
            if systems.is_empty() {
                println!("No compute systems found.");
            } else {
                println!("{:<40} {:<15} {:<10}", "ID", "OWNER", "STATE");
                println!("{}", "-".repeat(65));
                for sys in systems {
                    println!("{:<40} {:<15} {:<10}",
                        &sys.id[..std::cmp::min(38, sys.id.len())],
                        sys.owner.as_deref().unwrap_or("-"),
                        sys.state.as_deref().unwrap_or("-")
                    );
                }
            }
        }
        Err(e) => {
            println!("Failed to list: {}", e);
        }
    }

    Ok(())
}

fn cmd_destroy(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Destroying sandbox '{}'...", name);
    
    match hcs_sandbox::hcs::ComputeSystem::open(name) {
        Ok(cs) => {
            // Try to terminate first
            let _ = cs.terminate();
            drop(cs);
            println!("Sandbox '{}' destroyed!", name);
        }
        Err(e) => {
            println!("Failed to destroy: {}", e);
        }
    }

    Ok(())
}

fn cmd_info() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HCS Sandbox Info ===\n");

    if !is_elevated() {
        println!("WARNING: Not running as Administrator.\n");
    }

    println!("[*] HCS Service:");
    match compute::get_service_properties() {
        Ok(props) => {
            println!("    Status: Available");
            if let Some(versions) = props.supported_schema_versions {
                for v in versions {
                    println!("    Schema: {}.{}", v.major, v.minor);
                }
            }
        }
        Err(e) => println!("    Error: {}", e),
    }

    println!("\n[*] Compute Systems:");
    match compute::enumerate_compute_systems(None) {
        Ok(systems) => {
            if systems.is_empty() {
                println!("    None found");
            } else {
                for sys in systems {
                    println!("    - {} ({})", 
                        sys.id,
                        sys.owner.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }
        Err(e) => println!("    Error: {}", e),
    }

    println!("\n[*] Usage:");
    println!("    hcs-sandbox run [--memory <mb>] [--folder <path>] [--cmd <command>]");
    println!("    hcs-sandbox create --name <name> [--memory <mb>] [--cpus <n>]");
    println!("    hcs-sandbox list");
    println!("    hcs-sandbox start <name>");
    println!("    hcs-sandbox stop <name>");
    println!("    hcs-sandbox destroy <name>");
    println!("");
    println!("    'run' uses Windows Sandbox (easy mode, works out of the box)");
    println!("    'create' uses raw HCS API (requires base OS layer setup)");

    Ok(())
}

fn cmd_run(
    memory: u64,
    gpu: bool,
    network: bool,
    folder: Option<String>,
    cmd: Option<String>,
    keep_config: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Launching Windows Sandbox...");
    println!("  Memory: {} MB", memory);
    println!("  GPU: {}", if gpu { "enabled" } else { "disabled" });
    println!("  Network: {}", if network { "enabled" } else { "disabled" });

    // Build .wsb XML config
    let mut wsb = String::from("<Configuration>\n");

    // vGPU (GPU passthrough)
    wsb.push_str(&format!("  <VGpu>{}</VGpu>\n", if gpu { "Enable" } else { "Disable" }));

    // Networking
    wsb.push_str(&format!("  <Networking>{}</Networking>\n", if network { "Default" } else { "Disable" }));

    // Memory (in MB)
    wsb.push_str(&format!("  <MemoryInMB>{}</MemoryInMB>\n", memory));

    // Mapped folders
    if let Some(ref folder_spec) = folder {
        let (host_path, sandbox_path) = if folder_spec.contains(':') && folder_spec.chars().nth(1) != Some(':') {
            // Format: host_path:sandbox_path (but not C:\path)
            let parts: Vec<&str> = folder_spec.splitn(2, ':').collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else if folder_spec.len() > 2 && folder_spec.chars().nth(1) == Some(':') && folder_spec.contains("::") {
            // Handle Windows paths like C:\foo::C:\Users\...
            let parts: Vec<&str> = folder_spec.splitn(2, "::").collect();
            (parts[0].to_string(), Some(parts[1].to_string()))
        } else {
            (folder_spec.clone(), None)
        };

        wsb.push_str("  <MappedFolders>\n");
        wsb.push_str("    <MappedFolder>\n");
        wsb.push_str(&format!("      <HostFolder>{}</HostFolder>\n", host_path));
        if let Some(sandbox) = sandbox_path {
            wsb.push_str(&format!("      <SandboxFolder>{}</SandboxFolder>\n", sandbox));
        }
        wsb.push_str("      <ReadOnly>false</ReadOnly>\n");
        wsb.push_str("    </MappedFolder>\n");
        wsb.push_str("  </MappedFolders>\n");
        println!("  Mapped: {}", folder_spec);
    }

    // Startup command
    if let Some(ref command) = cmd {
        wsb.push_str("  <LogonCommand>\n");
        wsb.push_str(&format!("    <Command>{}</Command>\n", command));
        wsb.push_str("  </LogonCommand>\n");
        println!("  Startup: {}", command);
    }

    wsb.push_str("</Configuration>\n");

    // Write to a simple path (avoid 8.3 short names in %TEMP%)
    let wsb_path = std::path::PathBuf::from(format!("C:\\hcs-sandbox-{}.wsb", std::process::id()));
    std::fs::write(&wsb_path, &wsb)?;

    println!("\nConfig file: {}", wsb_path.display());
    println!("\n--- WSB Config ---");
    println!("{}", wsb);
    println!("------------------\n");

    // Launch Windows Sandbox
    println!("Starting WindowsSandbox.exe...");

    // Use spawn() instead of status() - WindowsSandbox.exe exits before the VM reads the config
    // We need to keep the file around until the sandbox is fully initialized
    let child = std::process::Command::new("WindowsSandbox.exe")
        .arg(&wsb_path)
        .spawn()?;

    println!("Windows Sandbox launched (PID: {:?})", child.id());

    // Wait a bit for sandbox to read the config before cleanup
    if !keep_config {
        println!("Waiting for sandbox to initialize before cleanup...");
        std::thread::sleep(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_file(&wsb_path);
        println!("Config file cleaned up.");
    } else {
        println!("Config file kept at: {}", wsb_path.display());
    }

    Ok(())
}

fn is_elevated() -> bool {
    std::process::Command::new("net")
        .args(["session"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Find available base layers in the Windows containers directory
fn find_base_layers() -> Vec<(String, String)> {
    let layers_path = Path::new(r"C:\ProgramData\Microsoft\Windows\Containers\Layers");
    let mut layers = Vec::new();

    if let Ok(entries) = std::fs::read_dir(layers_path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let layer_id = entry.file_name().to_string_lossy().to_string();
                let layer_path = entry.path().to_string_lossy().to_string();
                layers.push((layer_id, layer_path));
            }
        }
    }

    layers
}

fn cmd_layers() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Available Base Layers ===\n");

    let layers = find_base_layers();

    if layers.is_empty() {
        println!("No base layers found.");
        println!("\nTo create base layers, enable Windows Sandbox or Containers feature.");
    } else {
        println!("{:<40} PATH", "LAYER ID");
        println!("{}", "-".repeat(80));
        for (id, path) in &layers {
            println!("{:<40} {}", id, path);
        }
        println!("\nFound {} layer(s)", layers.len());
    }

    // Also check for sandbox VHDXs
    let storage_path = Path::new(r"C:\ProgramData\Microsoft\Windows\Containers\ContainerStorages");
    if storage_path.exists() {
        println!("\n=== Existing Sandbox Storage ===\n");
        if let Ok(entries) = std::fs::read_dir(storage_path) {
            for entry in entries.flatten() {
                let sandbox_vhdx = entry.path().join("sandbox.vhdx");
                if sandbox_vhdx.exists() {
                    println!("  {}", entry.path().display());
                }
            }
        }
    }

    Ok(())
}

fn cmd_clone(name: &str, storage_id: &str, copy: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cloning Sandbox: {} ===\n", name);

    if !is_elevated() {
        println!("ERROR: Must run as Administrator.\n");
        return Ok(());
    }

    // Find the storage path
    let storage_base = Path::new(r"C:\ProgramData\Microsoft\Windows\Containers\ContainerStorages");
    let storage_path = storage_base.join(storage_id);

    if !storage_path.exists() {
        println!("ERROR: Storage not found: {}", storage_path.display());
        println!("\nAvailable storages:");
        if let Ok(entries) = std::fs::read_dir(storage_base) {
            for entry in entries.flatten() {
                println!("  {}", entry.file_name().to_string_lossy());
            }
        }
        return Ok(());
    }

    let sandbox_vhdx = storage_path.join("sandbox.vhdx");

    if !sandbox_vhdx.exists() {
        println!("ERROR: sandbox.vhdx not found in storage");
        return Ok(());
    }

    // Determine VHDX path - either use original or make a standalone copy
    let vhdx_path = if copy {
        let our_storage = format!(r"C:\HcsSandboxes\{}", name);
        std::fs::create_dir_all(&our_storage)?;
        let our_vhdx = format!(r"{}\sandbox.vhdx", our_storage);

        if !Path::new(&our_vhdx).exists() {
            println!("Converting VHDX to standalone disk (this may take a while)...");
            // Use Convert-VHD to create a standalone copy (merges parent chain)
            let output = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "Convert-VHD -Path '{}' -DestinationPath '{}' -VHDType Dynamic",
                        sandbox_vhdx.display(), our_vhdx
                    ),
                ])
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("Convert-VHD failed: {}", stderr);
                println!("Falling back to simple copy...");
                std::fs::copy(&sandbox_vhdx, &our_vhdx)?;
            } else {
                println!("Created standalone VHDX: {}", our_vhdx);
            }
        } else {
            println!("Using existing copy: {}", our_vhdx);
        }
        our_vhdx
    } else {
        sandbox_vhdx.to_string_lossy().to_string()
    };

    println!("Using VHDX: {}", vhdx_path);

    // Use EXACT same minimal config that worked in test
    let hcs_config = serde_json::json!({
        "SchemaVersion": { "Major": 2, "Minor": 1 },
        "Owner": "hcs-sandbox",
        "ShouldTerminateOnLastHandleClosed": true,
        "VirtualMachine": {
            "StopOnReset": true,
            "Chipset": {
                "Uefi": {
                    "BootThis": {
                        "DeviceType": "ScsiDrive",
                        "DevicePath": "Scsi(0,0)"
                    }
                }
            },
            "ComputeTopology": {
                "Memory": { "SizeInMB": 2048 },
                "Processor": { "Count": 2 }
            },
            "Devices": {
                "Scsi": {
                    "0": {
                        "Attachments": {
                            "0": {
                                "Path": &vhdx_path,
                                "Type": "VirtualDisk"
                            }
                        }
                    }
                },
                "HvSocket": {}
            },
            "GuestState": {
                "GuestStateFilePath": "",
                "RuntimeStateFilePath": ""
            }
        }
    });

    let config_json = serde_json::to_string_pretty(&hcs_config)?;
    println!("\n--- HCS Configuration ---");
    println!("{}", config_json);
    println!("-------------------------\n");

    // Create and start
    println!("Creating compute system...");
    match hcs_sandbox::hcs::ComputeSystem::create(name, &serde_json::to_string(&hcs_config)?) {
        Ok(cs) => {
            println!("Created: {}", cs.id());
            println!("Starting...");
            match cs.start() {
                Ok(()) => {
                    println!("\n=== SUCCESS ===");
                    println!("Sandbox '{}' is running!", name);
                    println!("\nKeeping process alive to maintain sandbox...");
                    println!("Press Ctrl+C to stop the sandbox.");
                    println!("\nIn another terminal:");
                    println!("  cargo run -- list    # See running sandboxes");

                    // Keep the handle alive by waiting forever
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(60));
                    }
                }
                Err(e) => println!("Start failed: {}", e),
            }
        }
        Err(e) => println!("Create failed: {}", e),
    }

    Ok(())
}

fn cmd_new(name: &str, memory: u64, cpus: u32) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Creating New Sandbox: {} ===\n", name);

    if !is_elevated() {
        println!("ERROR: Must run as Administrator.\n");
        return Ok(());
    }

    // Find base layer
    let layers = find_base_layers();
    if layers.is_empty() {
        println!("ERROR: No base layers found.");
        println!("Run Windows Sandbox at least once to create base layers.");
        return Ok(());
    }

    let (base_layer_id, base_layer_path) = &layers[0];
    println!("Using base layer: {}", base_layer_id);
    println!("  Path: {}", base_layer_path);

    // Create sandbox storage directory
    let our_storage = format!(r"C:\HcsSandboxes\{}", name);
    std::fs::create_dir_all(&our_storage)?;
    let sandbox_vhdx = format!(r"{}\sandbox.vhdx", our_storage);

    println!("Storage: {}", our_storage);

    // Create a differencing VHDX that references the base layer
    if !Path::new(&sandbox_vhdx).exists() {
        println!("\nCreating differencing VHDX...");

        // The base layer has a UtilityVM folder with the actual bootable disk
        let utility_vm_vhdx = format!(r"{}\UtilityVM\SystemTemplate.vhdx", base_layer_path);

        if Path::new(&utility_vm_vhdx).exists() {
            println!("Found UtilityVM template: {}", utility_vm_vhdx);

            // Create differencing disk
            let output = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!(
                        "New-VHD -Path '{}' -ParentPath '{}' -Differencing",
                        sandbox_vhdx, utility_vm_vhdx
                    ),
                ])
                .output()?;

            if !output.status.success() {
                println!("Failed to create differencing VHDX: {}", String::from_utf8_lossy(&output.stderr));

                // Fallback: create a fresh dynamic VHDX
                println!("Falling back to fresh dynamic VHDX...");
                let output = std::process::Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!("New-VHD -Path '{}' -SizeBytes 20GB -Dynamic", sandbox_vhdx),
                    ])
                    .output()?;

                if !output.status.success() {
                    println!("Failed to create VHDX: {}", String::from_utf8_lossy(&output.stderr));
                    return Ok(());
                }
            } else {
                println!("Created differencing VHDX: {}", sandbox_vhdx);
            }
        } else {
            println!("No UtilityVM template found, creating fresh VHDX...");
            let output = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!("New-VHD -Path '{}' -SizeBytes 20GB -Dynamic", sandbox_vhdx),
                ])
                .output()?;

            if !output.status.success() {
                println!("Failed to create VHDX: {}", String::from_utf8_lossy(&output.stderr));
                return Ok(());
            }
            println!("Created fresh VHDX: {}", sandbox_vhdx);
        }
    }

    // Build config
    let config = SandboxConfig::builder()
        .name(name)
        .memory_mb(memory)
        .cpu_count(cpus)
        .gpu_enabled(true)
        .build();

    let hcs_config = config.to_hcs_fresh_config(&our_storage, base_layer_id);
    let config_json = serde_json::to_string_pretty(&hcs_config)?;
    println!("\n--- HCS Configuration ---");
    println!("{}", config_json);
    println!("-------------------------\n");

    // Create and start
    println!("Creating compute system...");
    match hcs_sandbox::hcs::ComputeSystem::create(name, &serde_json::to_string(&hcs_config)?) {
        Ok(cs) => {
            println!("Created: {}", cs.id());
            println!("Starting...");
            match cs.start() {
                Ok(()) => {
                    println!("\n=== SUCCESS ===");
                    println!("Sandbox '{}' is running!", name);
                    println!("\nKeeping process alive to maintain sandbox...");
                    println!("Press Ctrl+C to stop the sandbox.");

                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(60));
                    }
                }
                Err(e) => println!("Start failed: {}", e),
            }
        }
        Err(e) => println!("Create failed: {}", e),
    }

    Ok(())
}

fn cmd_test(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HCS Configuration Tests: {} ===\n", name);

    if !is_elevated() {
        println!("ERROR: Must run as Administrator.\n");
        return Ok(());
    }

    // Find the Windows Sandbox VHDX
    let storage_base = Path::new(r"C:\ProgramData\Microsoft\Windows\Containers\ContainerStorages");
    let mut sandbox_vhdx: Option<String> = None;

    if let Ok(entries) = std::fs::read_dir(storage_base) {
        for entry in entries.flatten() {
            let vhdx = entry.path().join("sandbox.vhdx");
            if vhdx.exists() {
                sandbox_vhdx = Some(vhdx.to_string_lossy().to_string());
                break;
            }
        }
    }

    let vhdx_path = match sandbox_vhdx {
        Some(p) => p,
        None => {
            println!("ERROR: No sandbox VHDX found. Run Windows Sandbox first.");
            return Ok(());
        }
    };

    println!("Using VHDX: {}\n", vhdx_path);

    // Test 1: VM with full config including firmware
    println!("=== Test 1: VM with UEFI boot config ===");
    let config1 = serde_json::json!({
        "SchemaVersion": { "Major": 2, "Minor": 1 },
        "Owner": "hcs-sandbox-test",
        "ShouldTerminateOnLastHandleClosed": true,
        "VirtualMachine": {
            "StopOnReset": true,
            "Chipset": {
                "Uefi": {
                    "BootThis": {
                        "DeviceType": "ScsiDrive",
                        "DevicePath": "Scsi(0,0)"
                    }
                }
            },
            "ComputeTopology": {
                "Memory": { "SizeInMB": 2048 },
                "Processor": { "Count": 2 }
            },
            "Devices": {
                "Scsi": {
                    "0": {
                        "Attachments": {
                            "0": {
                                "Path": &vhdx_path,
                                "Type": "VirtualDisk"
                            }
                        }
                    }
                },
                "HvSocket": {}
            },
            "GuestState": {
                "GuestStateFilePath": "",
                "RuntimeStateFilePath": ""
            }
        }
    });

    let test_name1 = format!("{}-test1", name);
    println!("Config: {}", serde_json::to_string_pretty(&config1)?);

    match hcs_sandbox::hcs::ComputeSystem::create(&test_name1, &serde_json::to_string(&config1)?) {
        Ok(cs) => {
            println!("SUCCESS: Created compute system: {}", cs.id());
            println!("Now trying to start...");
            match cs.start() {
                Ok(()) => println!("SUCCESS: Started!"),
                Err(e) => println!("FAILED to start: {}", e),
            }
        }
        Err(e) => println!("FAILED to create: {}", e),
    }

    // Test 1b: Try launching Windows Sandbox in background and capturing its ID
    println!("\n=== Test 1b: Launch Windows Sandbox and inspect ===");
    println!("Running 'WindowsSandbox.exe' in background...");

    let wsb_content = "<Configuration><VGpu>Enable</VGpu></Configuration>";
    let wsb_path = format!("C:\\hcs-sandbox-test-{}.wsb", std::process::id());
    std::fs::write(&wsb_path, wsb_content)?;

    let child = std::process::Command::new("WindowsSandbox.exe")
        .arg(&wsb_path)
        .spawn();

    match child {
        Ok(_) => {
            println!("Windows Sandbox launched. Waiting 10s for it to start...");
            std::thread::sleep(std::time::Duration::from_secs(10));

            // List compute systems to see what Windows Sandbox created
            println!("Checking compute systems after Windows Sandbox launch:");
            match hcs_sandbox::hcs::compute::enumerate_compute_systems(None) {
                Ok(systems) => {
                    for s in &systems {
                        println!("  {} - owner: {:?}, state: {:?}", s.id, s.owner, s.state);

                        // Try to get properties of the running sandbox
                        if s.state.as_deref() == Some("Running") {
                            if let Ok(cs) = hcs_sandbox::hcs::ComputeSystem::open(&s.id) {
                                if let Ok(props) = cs.get_properties(Some("{}")) {
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&props) {
                                        println!("  Properties: {}", serde_json::to_string_pretty(&json)?);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("Failed: {}", e),
            }
        }
        Err(e) => println!("Could not launch Windows Sandbox: {}", e),
    }

    let _ = std::fs::remove_file(&wsb_path);

    println!("\n=== Test 2: Container config (not VM) ===");
    let layers = find_base_layers();
    if !layers.is_empty() {
        let (layer_id, layer_path) = &layers[0];

        let config2 = serde_json::json!({
            "SchemaVersion": { "Major": 2, "Minor": 1 },
            "Owner": "hcs-sandbox-test",
            "ShouldTerminateOnLastHandleClosed": true,
            "Container": {
                "Storage": {
                    "Layers": [
                        {
                            "Id": layer_id,
                            "Path": layer_path
                        }
                    ]
                }
            }
        });

        let test_name2 = format!("{}-test2", name);
        println!("Config: {}", serde_json::to_string_pretty(&config2)?);

        match hcs_sandbox::hcs::ComputeSystem::create(&test_name2, &serde_json::to_string(&config2)?) {
            Ok(cs) => {
                println!("SUCCESS: Created container: {}", cs.id());
                match cs.start() {
                    Ok(()) => println!("SUCCESS: Started!"),
                    Err(e) => println!("FAILED to start: {}", e),
                }
            }
            Err(e) => println!("FAILED to create: {}", e),
        }
    }

    println!("\n=== Test 3: Check what HCS reports ===");
    match hcs_sandbox::hcs::compute::get_service_properties() {
        Ok(props) => {
            println!("HCS Service Properties:");
            if let Some(versions) = props.supported_schema_versions {
                for v in versions {
                    println!("  Schema version: {}.{}", v.major, v.minor);
                }
            }
        }
        Err(e) => println!("Failed to get properties: {}", e),
    }

    println!("\n=== Test 4: List existing compute systems ===");
    let mut template_id: Option<String> = None;
    match hcs_sandbox::hcs::compute::enumerate_compute_systems(None) {
        Ok(systems) => {
            if systems.is_empty() {
                println!("No compute systems found");
            } else {
                for s in &systems {
                    println!("  {} - owner: {:?}, state: {:?}", s.id, s.owner, s.state);
                    // Find the CmService template
                    if s.owner.as_deref() == Some("CmService") && s.state.as_deref() == Some("SavedAsTemplate") {
                        template_id = Some(s.id.clone());
                    }
                }
            }
        }
        Err(e) => println!("Failed to enumerate: {}", e),
    }

    // Test 5: Try to use the CmService template
    if let Some(ref tid) = template_id {
        println!("\n=== Test 5: Use CmService template as hosting system ===");
        println!("Template ID: {}", tid);

        // Try a Hyper-V isolated container that uses the template
        let config5 = serde_json::json!({
            "SchemaVersion": { "Major": 2, "Minor": 1 },
            "Owner": "hcs-sandbox-test",
            "ShouldTerminateOnLastHandleClosed": true,
            "HostingSystemId": tid,
            "Container": {
                "Storage": {
                    "Layers": [
                        {
                            "Id": &layers[0].0,
                            "Path": &layers[0].1
                        }
                    ]
                }
            }
        });

        let test_name5 = format!("{}-test5", name);
        println!("Config: {}", serde_json::to_string_pretty(&config5)?);

        match hcs_sandbox::hcs::ComputeSystem::create(&test_name5, &serde_json::to_string(&config5)?) {
            Ok(cs) => {
                println!("SUCCESS: Created hosted container: {}", cs.id());
                match cs.start() {
                    Ok(()) => println!("SUCCESS: Started!"),
                    Err(e) => println!("FAILED to start: {}", e),
                }
            }
            Err(e) => println!("FAILED to create: {}", e),
        }

        // Test 6: Try to clone/resume from template
        println!("\n=== Test 6: Try to open and get properties of template ===");
        match hcs_sandbox::hcs::ComputeSystem::open(tid) {
            Ok(cs) => {
                println!("Opened template: {}", cs.id());
                match cs.get_properties(Some("{}")) {
                    Ok(props) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&props) {
                            println!("Properties: {}", serde_json::to_string_pretty(&json)?);
                        } else {
                            println!("Properties: {}", props);
                        }
                    }
                    Err(e) => println!("Failed to get properties: {}", e),
                }
            }
            Err(e) => println!("Failed to open template: {}", e),
        }
    }

    Ok(())
}

fn cmd_props(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Compute System Properties: {} ===\n", id);

    match hcs_sandbox::hcs::ComputeSystem::open(id) {
        Ok(cs) => {
            // Try different query formats
            let queries = [
                ("Schema v2.1", r#"{"SchemaVersion":{"Major":2,"Minor":1}}"#),
                ("Empty object", "{}"),
                ("Null", "null"),
            ];

            for (name, query) in queries {
                println!("--- Trying {} ---", name);
                match cs.get_properties(Some(query)) {
                    Ok(props) => {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&props) {
                            println!("{}\n", serde_json::to_string_pretty(&json)?);
                        } else {
                            println!("{}\n", props);
                        }
                        break; // Success, stop trying
                    }
                    Err(e) => {
                        println!("Failed: {}\n", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("Failed to open compute system: {}", e);
        }
    }

    Ok(())
}

fn cmd_hcs(name: &str, memory: u64, cpus: u32, gpu: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Launching HCS Container: {} ===\n", name);

    if !is_elevated() {
        println!("ERROR: Must run as Administrator for raw HCS access.\n");
        return Ok(());
    }

    // Find base layer
    let layers = find_base_layers();
    if layers.is_empty() {
        println!("ERROR: No base layers found.");
        println!("Run Windows Sandbox at least once to create base layers.");
        return Ok(());
    }

    let (base_layer_id, base_layer_path) = &layers[0];
    println!("Using base layer: {}", base_layer_id);
    println!("  Path: {}", base_layer_path);

    // Create sandbox storage directory
    let sandbox_dir = format!(r"C:\HcsSandboxes\{}", name);
    std::fs::create_dir_all(&sandbox_dir)?;
    let sandbox_vhdx = format!(r"{}\sandbox.vhdx", sandbox_dir);

    println!("\nSandbox storage: {}", sandbox_dir);

    // Create writable sandbox VHDX if it doesn't exist
    if !Path::new(&sandbox_vhdx).exists() {
        println!("Creating sandbox VHDX...");
        // Use PowerShell to create a differencing disk
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "New-VHD -Path '{}' -SizeBytes 20GB -Dynamic",
                    sandbox_vhdx
                ),
            ])
            .output()?;

        if !output.status.success() {
            println!("Warning: Could not create VHDX: {}", String::from_utf8_lossy(&output.stderr));
        } else {
            println!("Created sandbox VHDX: {}", sandbox_vhdx);
        }
    }

    // Build HCS config
    let config = SandboxConfig::builder()
        .name(name)
        .memory_mb(memory)
        .cpu_count(cpus)
        .gpu_enabled(gpu)
        .build();

    let hcs_config = config.to_hcs_hyperv_config(base_layer_id, &sandbox_vhdx);
    let config_json = serde_json::to_string_pretty(&hcs_config)?;

    println!("\n--- HCS Configuration ---");
    println!("{}", config_json);
    println!("-------------------------\n");

    // Create the compute system
    println!("Creating HCS compute system...");
    match hcs_sandbox::hcs::ComputeSystem::create(name, &serde_json::to_string(&hcs_config)?) {
        Ok(cs) => {
            println!("Created compute system: {}", cs.id());

            // Try to start it
            println!("Starting compute system...");
            match cs.start() {
                Ok(()) => {
                    println!("\n=== SUCCESS ===");
                    println!("Sandbox '{}' is now running!", name);
                    println!("\nCommands:");
                    println!("  hcs-sandbox list              - List running sandboxes");
                    println!("  hcs-sandbox stop {}       - Stop this sandbox", name);
                    println!("  hcs-sandbox destroy {}    - Destroy this sandbox", name);

                    // Keep handle alive - in a real implementation we'd store this
                    println!("\nPress Enter to keep sandbox running (or Ctrl+C to exit)...");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                }
                Err(e) => {
                    println!("Failed to start: {}", e);
                    println!("\nThe compute system was created but couldn't start.");
                    println!("This might be due to missing UEFI/boot configuration.");
                }
            }
        }
        Err(e) => {
            println!("Failed to create compute system: {}", e);
            println!("\nPossible causes:");
            println!("  - Base layer might not be bootable");
            println!("  - Missing UEFI configuration");
            println!("  - HCS schema version mismatch");
        }
    }

    Ok(())
}
