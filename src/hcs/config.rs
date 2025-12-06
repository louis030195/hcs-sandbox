//! HCS Configuration types
//! Based on: https://learn.microsoft.com/en-us/virtualization/api/hcs/schemareference

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self { major: 2, minor: 1 }
    }
}

/// Container isolation type
#[derive(Debug, Serialize, Deserialize)]
pub enum IsolationType {
    /// Process isolation - shares host kernel, lighter weight
    /// NOT suitable for UI automation (no desktop session)
    Process,
    /// HyperV isolation - full VM with its own kernel
    /// Required for UI automation with GPU passthrough
    HyperV,
}

/// GPU configuration for HyperV isolation
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GpuConfiguration {
    /// Enable GPU-PV (paravirtualized GPU)
    pub allow_vendor_extension: bool,
}

/// Virtual SMB share for mapping host folders into container
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MappedDirectory {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

/// Container layer (base image layer)
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Layer {
    pub id: String,
    pub path: String,
}

/// Main container configuration
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerConfig {
    #[serde(rename = "Type")]
    pub isolation_type: IsolationType,
    pub layers: Vec<Layer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mapped_directories: Option<Vec<MappedDirectory>>,
}

/// HyperV-specific configuration (for UI automation)
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HyperVConfig {
    /// Memory in MB
    pub memory: MemoryConfig,
    /// Processor configuration
    pub processor: ProcessorConfig,
    /// GPU passthrough for UI rendering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuConfiguration>,
    /// Enable enhanced session mode (for RDP-like access)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhanced_mode_state: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MemoryConfig {
    #[serde(rename = "SizeInMB")]
    pub size_in_mb: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProcessorConfig {
    pub count: u32,
}

/// Root HCS compute system configuration
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ComputeSystemConfig {
    pub schema_version: SchemaVersion,
    pub owner: String,
    pub should_terminate_on_last_handle_closed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<ContainerConfig>,
    #[serde(rename = "VirtualMachine", skip_serializing_if = "Option::is_none")]
    pub virtual_machine: Option<HyperVConfig>,
}

impl ComputeSystemConfig {
    /// Create a config suitable for UI automation
    /// Uses HyperV isolation with GPU passthrough
    pub fn for_ui_automation(owner: &str, base_layer_path: &str) -> Self {
        Self {
            schema_version: SchemaVersion::default(),
            owner: owner.to_string(),
            should_terminate_on_last_handle_closed: true,
            container: Some(ContainerConfig {
                isolation_type: IsolationType::HyperV,
                layers: vec![Layer {
                    id: "base".to_string(),
                    path: base_layer_path.to_string(),
                }],
                mapped_directories: None,
            }),
            virtual_machine: Some(HyperVConfig {
                memory: MemoryConfig { size_in_mb: 4096 },
                processor: ProcessorConfig { count: 2 },
                gpu: Some(GpuConfiguration {
                    allow_vendor_extension: true,
                }),
                enhanced_mode_state: Some(true),
            }),
        }
    }
}
