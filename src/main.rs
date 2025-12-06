//! HCS (Host Compute Service) exploration
//! Low-level Windows container/sandbox API - what Docker and Windows Sandbox use
//!
//! This demonstrates creating sandboxes WITHOUT Docker using the same APIs that
//! Windows Sandbox uses internally.
//!
//! Requires: Windows 10/11 Pro with Hyper-V and Containers features enabled
//! Run as Administrator!

mod hcs;

use hcs::config::ComputeSystemConfig;
use hcs::system;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HCS (Host Compute Service) Explorer ===\n");

    if !is_elevated() {
        println!("WARNING: Not running as Administrator. HCS operations will likely fail.\n");
    }

    // Get HCS service properties
    println!("[*] Getting HCS service properties...");
    match system::get_service_properties() {
        Ok(props) => {
            let parsed: serde_json::Value = serde_json::from_str(&props)?;
            println!("    {}", serde_json::to_string_pretty(&parsed)?);
        }
        Err(e) => println!("    Error: {}", e),
    }

    // Enumerate existing compute systems
    println!("\n[*] Enumerating existing compute systems...");
    match system::enumerate_compute_systems(None) {
        Ok(result) => {
            if result.is_empty() || result == "[]" {
                println!("    No compute systems found");
            } else {
                let parsed: serde_json::Value = serde_json::from_str(&result)?;
                println!("    {}", serde_json::to_string_pretty(&parsed)?);
            }
        }
        Err(e) => println!("    Error: {}", e),
    }

    // Show example configs
    println!("\n[*] Example HyperV container config (for UI automation):");
    let config = ComputeSystemConfig::for_ui_automation(
        "hcs-sandbox",
        r"C:\Sandbox\BaseLayer",
    );
    println!("{}", serde_json::to_string_pretty(&config)?);

    // Explain the Docker-free approach
    print_docker_free_guide();

    Ok(())
}

fn print_docker_free_guide() {
    println!(r#"
=== Creating Sandboxes WITHOUT Docker ===

Windows Sandbox uses HCS directly with a "dynamic base layer" from the host OS.
Here's the flow:

1. CREATE WRITABLE LAYER VHD
   ```rust
   let vhd = layer::create_layer_vhd(r"C:\Sandbox\writable.vhdx", 20)?; // 20GB
   layer::format_writable_layer_vhd(vhd)?;
   ```

2. SETUP BASE OS LAYER (the magic - no Docker images needed!)
   ```rust
   // Creates a copy-on-write view of your Windows installation
   layer::setup_base_os_layer(r"C:\Sandbox\BaseLayer", vhd)?;
   ```

3. CREATE HYPERV CONTAINER WITH GPU
   ```rust
   let config = json!({{
       "SchemaVersion": {{"Major": 2, "Minor": 1}},
       "Owner": "my-sandbox",
       "VirtualMachine": {{
           "Chipset": {{"UseUtc": true}},
           "ComputeTopology": {{
               "Memory": {{"SizeInMB": 4096}},
               "Processor": {{"Count": 4}}
           }},
           "Devices": {{
               "VideoMonitor": {{}},
               "Gpu": {{"AllowVendorExtension": true}}, // GPU passthrough!
               "Keyboard": {{}},
               "Mouse": {{}},
               "EnhancedModeVideo": {{"ConnectionOptions": {{}}}}
           }}
       }}
   }});
   
   let sandbox = system::ComputeSystem::create("my-sandbox", &config)?;
   sandbox.start()?;
   ```

4. CONNECT VIA ENHANCED SESSION (RDP-like)
   - Use HvSocket or vmconnect.exe
   - Or use the HCS process API to run commands inside

=== Key Insight ===
For UI Automation you NEED:
- HyperV isolation (full VM, not process isolation)
- GPU passthrough (AllowVendorExtension)
- Desktop session (EnhancedModeVideo or similar)

Process-isolated containers (standard Docker) share the host kernel
and have NO desktop session - they're headless.
"#);
}

fn is_elevated() -> bool {
    std::process::Command::new("net")
        .args(["session"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
