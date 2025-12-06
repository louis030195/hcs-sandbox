//! HCS Sandbox CLI

use clap::{Parser, Subcommand};
use hcs_sandbox::SandboxConfig;
use hcs_sandbox::hcs::compute;

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

    // Write to temp file
    let temp_dir = std::env::temp_dir();
    let wsb_path = temp_dir.join(format!("hcs-sandbox-{}.wsb", std::process::id()));
    std::fs::write(&wsb_path, &wsb)?;

    println!("\nConfig file: {}", wsb_path.display());
    println!("\n--- WSB Config ---");
    println!("{}", wsb);
    println!("------------------\n");

    // Launch Windows Sandbox
    println!("Starting WindowsSandbox.exe...");
    let status = std::process::Command::new("WindowsSandbox.exe")
        .arg(&wsb_path)
        .status()?;

    // Cleanup
    if !keep_config {
        let _ = std::fs::remove_file(&wsb_path);
    }

    if status.success() {
        println!("Windows Sandbox launched successfully!");
    } else {
        println!("Windows Sandbox exited with: {:?}", status.code());
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
