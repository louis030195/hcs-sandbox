//! HCS Sandbox CLI

use hcs_sandbox::SandboxConfig;
use hcs_sandbox::hcs::compute;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HCS Sandbox Orchestrator ===\n");

    if !is_elevated() {
        println!("WARNING: Not running as Administrator.\n");
    }

    println!("[*] Checking HCS service...");
    match compute::get_service_properties() {
        Ok(props) => {
            println!("    HCS service available");
            if let Some(versions) = props.supported_schema_versions {
                for v in versions {
                    println!("    Schema: {}.{}", v.major, v.minor);
                }
            }
        }
        Err(e) => println!("    Error: {}", e),
    }

    println!("\n[*] Existing compute systems:");
    match compute::enumerate_compute_systems(None) {
        Ok(systems) => {
            if systems.is_empty() {
                println!("    None found");
            } else {
                for sys in systems {
                    println!("    - {}", sys.id);
                }
            }
        }
        Err(e) => println!("    Error: {}", e),
    }

    println!("\n[*] Example sandbox config:");
    let config = SandboxConfig::builder()
        .name("ui-automation-sandbox")
        .memory_mb(4096)
        .cpu_count(2)
        .gpu_enabled(true)
        .build();

    println!("{}", serde_json::to_string_pretty(&config)?);
    println!("\n[*] HCS config:");
    println!("{}", serde_json::to_string_pretty(&config.to_hcs_config())?);

    Ok(())
}

fn is_elevated() -> bool {
    std::process::Command::new("net")
        .args(["session"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
