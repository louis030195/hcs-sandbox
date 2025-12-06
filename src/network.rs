//! HCS Network namespace management (simplified)
//! 
//! Note: Full HCN support requires additional Windows features.
//! This module provides basic network configuration helpers.

use crate::{Error, Result};
use std::process::Command;

/// Network configuration for sandboxes
pub struct NetworkConfig {
    pub nat_enabled: bool,
    pub subnet: String,
    pub gateway: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            nat_enabled: true,
            subnet: "172.28.0.0/16".to_string(),
            gateway: "172.28.0.1".to_string(),
        }
    }
}

/// Create a NAT network using PowerShell (simpler than raw HCN API)
pub fn create_nat_network(name: &str, config: &NetworkConfig) -> Result<()> {
    let script = format!(
        r#"
        $existing = Get-HnsNetwork | Where-Object {{ $_.Name -eq '{}' }}
        if ($existing) {{
            Write-Host "Network already exists"
            return
        }}
        New-HnsNetwork -Type NAT -Name '{}' -AddressPrefix '{}' -Gateway '{}'
        "#,
        name, name, config.subnet, config.gateway
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()?;

    if !output.status.success() {
        return Err(Error::Network(format!(
            "Failed to create NAT network: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

/// Delete a network
pub fn delete_network(name: &str) -> Result<()> {
    let script = format!(
        r#"
        $net = Get-HnsNetwork | Where-Object {{ $_.Name -eq '{}' }}
        if ($net) {{
            Remove-HnsNetwork -Id $net.Id
        }}
        "#,
        name
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()?;

    if !output.status.success() {
        return Err(Error::Network(format!(
            "Failed to delete network: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

/// List existing HNS networks
pub fn list_networks() -> Result<Vec<NetworkInfo>> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-HnsNetwork | ConvertTo-Json -Depth 3",
        ])
        .output()?;

    if !output.status.success() {
        return Err(Error::Network(format!(
            "Failed to list networks: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(Vec::new());
    }

    // Handle both single object and array
    let networks: Vec<NetworkInfo> = if stdout.trim().starts_with('[') {
        serde_json::from_str(&stdout)?
    } else {
        vec![serde_json::from_str(&stdout)?]
    };

    Ok(networks)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkInfo {
    pub name: String,
    pub id: Option<String>,
    #[serde(rename = "Type")]
    pub network_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert!(config.nat_enabled);
        assert!(!config.subnet.is_empty());
    }
}
