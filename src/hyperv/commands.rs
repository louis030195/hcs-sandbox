//! PowerShell wrappers for Hyper-V commands

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::{Duration, Instant};

/// VM information from Hyper-V
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperVInfo {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "State")]
    pub state: i32,
    #[serde(rename = "MemoryAssigned")]
    pub memory_assigned: Option<u64>,
    #[serde(rename = "Uptime")]
    pub uptime: Option<String>,
    #[serde(rename = "Id")]
    pub id: Option<String>,
}

impl HyperVInfo {
    pub fn state_str(&self) -> &'static str {
        match self.state {
            2 => "Off",
            3 => "Running",
            6 => "Saved",
            9 => "Paused",
            _ => "Unknown",
        }
    }
}

/// Hyper-V operations
pub struct HyperV;

impl HyperV {
    /// Check if Hyper-V is available
    pub fn is_available() -> Result<bool> {
        let output = powershell("Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V | Select-Object -ExpandProperty State")?;
        Ok(output.trim() == "Enabled")
    }

    /// List all VMs
    pub fn list_vms() -> Result<Vec<HyperVInfo>> {
        let output = powershell(
            r#"Get-VM | Select-Object Name, State, MemoryAssigned, @{N='Uptime';E={$_.Uptime.ToString()}}, Id | ConvertTo-Json -Compress"#,
        )?;

        if output.trim().is_empty() {
            return Ok(vec![]);
        }

        // Handle single vs array JSON
        if output.trim().starts_with('[') {
            Ok(serde_json::from_str(&output)?)
        } else {
            let single: HyperVInfo = serde_json::from_str(&output)?;
            Ok(vec![single])
        }
    }

    /// Get VM by name
    pub fn get_vm(name: &str) -> Result<Option<HyperVInfo>> {
        let output = powershell(&format!(
            r#"Get-VM -Name '{}' -ErrorAction SilentlyContinue | Select-Object Name, State, MemoryAssigned, @{{N='Uptime';E={{$_.Uptime.ToString()}}}}, Id | ConvertTo-Json -Compress"#,
            escape_ps(name)
        ))?;

        if output.trim().is_empty() {
            return Ok(None);
        }

        Ok(Some(serde_json::from_str(&output)?))
    }

    /// Create VM with existing VHDX
    pub fn create_vm(
        name: &str,
        vhdx_path: &str,
        memory_mb: u64,
        cpu_count: u32,
    ) -> Result<()> {
        let script = format!(
            r#"
            New-VM -Name '{}' -MemoryStartupBytes {}MB -Generation 2 -VHDPath '{}'
            Set-VM -Name '{}' -ProcessorCount {} -AutomaticStartAction Nothing -AutomaticStopAction Save
            Set-VMMemory -VMName '{}' -DynamicMemoryEnabled $true -MinimumBytes 512MB -MaximumBytes {}MB
            "#,
            escape_ps(name),
            memory_mb,
            escape_ps(vhdx_path),
            escape_ps(name),
            cpu_count,
            escape_ps(name),
            memory_mb * 2
        );
        powershell(&script)?;
        Ok(())
    }

    /// Create differencing disk (COW clone from parent)
    pub fn create_differencing_disk(parent_path: &str, child_path: &str) -> Result<()> {
        powershell(&format!(
            "New-VHD -Path '{}' -ParentPath '{}' -Differencing",
            escape_ps(child_path),
            escape_ps(parent_path)
        ))?;
        Ok(())
    }

    /// Start VM (resumes if saved, cold boots if off)
    pub fn start_vm(name: &str) -> Result<()> {
        powershell(&format!("Start-VM -Name '{}'", escape_ps(name)))?;
        Ok(())
    }

    /// Save VM state to disk (fast resume later)
    pub fn save_vm(name: &str) -> Result<()> {
        powershell(&format!("Save-VM -Name '{}'", escape_ps(name)))?;
        Ok(())
    }

    /// Stop VM (graceful shutdown)
    pub fn stop_vm(name: &str, force: bool) -> Result<()> {
        let force_flag = if force { " -Force" } else { "" };
        powershell(&format!("Stop-VM -Name '{}'{}", escape_ps(name), force_flag))?;
        Ok(())
    }

    /// Turn off VM immediately (like pulling power)
    pub fn turn_off_vm(name: &str) -> Result<()> {
        powershell(&format!("Stop-VM -Name '{}' -TurnOff -Force", escape_ps(name)))?;
        Ok(())
    }

    /// Delete VM (does not delete VHDX)
    pub fn remove_vm(name: &str) -> Result<()> {
        powershell(&format!("Remove-VM -Name '{}' -Force", escape_ps(name)))?;
        Ok(())
    }

    /// Create checkpoint (snapshot)
    pub fn create_checkpoint(vm_name: &str, checkpoint_name: &str) -> Result<()> {
        powershell(&format!(
            "Checkpoint-VM -Name '{}' -SnapshotName '{}'",
            escape_ps(vm_name),
            escape_ps(checkpoint_name)
        ))?;
        Ok(())
    }

    /// Restore to checkpoint
    pub fn restore_checkpoint(vm_name: &str, checkpoint_name: &str) -> Result<()> {
        powershell(&format!(
            "Restore-VMCheckpoint -VMName '{}' -Name '{}' -Confirm:$false",
            escape_ps(vm_name),
            escape_ps(checkpoint_name)
        ))?;
        Ok(())
    }

    /// Get VM IP address(es)
    pub fn get_vm_ip(name: &str) -> Result<Option<String>> {
        let output = powershell(&format!(
            r#"(Get-VMNetworkAdapter -VMName '{}').IPAddresses | Where-Object {{ $_ -match '^\d+\.\d+\.\d+\.\d+$' }} | Select-Object -First 1"#,
            escape_ps(name)
        ))?;

        let ip = output.trim();
        if ip.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ip.to_string()))
        }
    }

    /// Wait for VM to be running and have an IP
    pub fn wait_for_ready(name: &str, timeout: Duration) -> Result<String> {
        let start = Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(Error::Timeout);
            }

            // Check VM state
            if let Some(info) = Self::get_vm(name)? {
                if info.state != 3 {
                    // Not running yet
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            } else {
                return Err(Error::VMNotFound(name.to_string()));
            }

            // Check for IP
            if let Some(ip) = Self::get_vm_ip(name)? {
                // Try TCP connect to RDP port
                if let Ok(_) = std::net::TcpStream::connect_timeout(
                    &format!("{}:3389", ip).parse().unwrap(),
                    Duration::from_secs(2),
                ) {
                    return Ok(ip);
                }
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Wait for guest heartbeat (integration services)
    pub fn wait_for_heartbeat(name: &str, timeout: Duration) -> Result<()> {
        let start = Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(Error::GuestNotResponding);
            }

            let output = powershell(&format!(
                r#"(Get-VMIntegrationService -VMName '{}' -Name 'Heartbeat' -ErrorAction SilentlyContinue).PrimaryStatusDescription"#,
                escape_ps(name)
            ))?;

            if output.trim() == "OK" {
                return Ok(());
            }

            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Enable enhanced session mode
    pub fn enable_enhanced_session(name: &str) -> Result<()> {
        powershell(&format!(
            "Set-VM -Name '{}' -EnhancedSessionTransportType HvSocket",
            escape_ps(name)
        ))?;
        Ok(())
    }

    /// Add GPU partition adapter (GPU-PV)
    pub fn add_gpu(name: &str) -> Result<()> {
        powershell(&format!(
            r#"
            Add-VMGpuPartitionAdapter -VMName '{}'
            Set-VMGpuPartitionAdapter -VMName '{}' -MinPartitionVRAM 80000000 -MaxPartitionVRAM 100000000 -OptimalPartitionVRAM 100000000 -MinPartitionEncode 80000000 -MaxPartitionEncode 100000000 -OptimalPartitionEncode 100000000
            Set-VM -Name '{}' -GuestControlledCacheTypes $true -LowMemoryMappedIoSpace 1GB -HighMemoryMappedIoSpace 32GB
            "#,
            escape_ps(name),
            escape_ps(name),
            escape_ps(name)
        ))?;
        Ok(())
    }

    /// Configure network adapter
    pub fn set_network_adapter(name: &str, switch_name: &str) -> Result<()> {
        powershell(&format!(
            "Get-VMNetworkAdapter -VMName '{}' | Connect-VMNetworkAdapter -SwitchName '{}'",
            escape_ps(name),
            escape_ps(switch_name)
        ))?;
        Ok(())
    }

    /// Get available memory on host
    pub fn get_host_available_memory_mb() -> Result<u64> {
        let output = powershell(
            r#"[math]::Round((Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory / 1024)"#,
        )?;
        output
            .trim()
            .parse()
            .map_err(|_| Error::Parse("Failed to parse memory".into()))
    }

    /// Open VM console (vmconnect)
    pub fn open_console(name: &str) -> Result<()> {
        Command::new("vmconnect")
            .args(["localhost", name])
            .spawn()?;
        Ok(())
    }

    /// Wait for terminator MCP agent to be healthy (port 8080)
    pub fn wait_for_terminator(ip: &str, timeout: Duration) -> Result<()> {
        use std::io::{Read, Write};
        let start = Instant::now();
        let addr: std::net::SocketAddr = format!("{}:8080", ip).parse()
            .map_err(|_| Error::Parse(format!("Invalid IP: {}", ip)))?;

        loop {
            if start.elapsed() > timeout {
                return Err(Error::GuestNotResponding);
            }

            if let Ok(mut stream) = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = stream.write_all(b"GET /health HTTP/1.0\r\nHost: localhost\r\n\r\n");
                let mut buf = [0u8; 512];
                if let Ok(n) = stream.read(&mut buf) {
                    let response = String::from_utf8_lossy(&buf[..n]);
                    if response.contains("200") || response.contains("healthy") {
                        return Ok(());
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(300));
        }
    }
}

/// Execute PowerShell command
fn powershell(script: &str) -> Result<String> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(Error::PowerShell(format!(
            "Exit code: {:?}\nStderr: {}\nStdout: {}",
            output.status.code(),
            stderr,
            stdout
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Escape string for PowerShell
fn escape_ps(s: &str) -> String {
    s.replace("'", "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_ps() {
        assert_eq!(escape_ps("test"), "test");
        assert_eq!(escape_ps("test's"), "test''s");
    }
}
