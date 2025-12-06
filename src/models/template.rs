//! Template model - golden images for VM creation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A VM template (golden image)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Path to base VHDX file
    pub vhdx_path: PathBuf,
    /// Default memory for VMs from this template
    pub memory_mb: u64,
    /// Default CPU count
    pub cpu_count: u32,
    /// Whether GPU is supported/configured
    pub gpu_enabled: bool,
    /// Software pre-installed in this template
    pub installed_software: Vec<String>,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Description
    pub description: Option<String>,
}

impl Template {
    pub fn new(name: impl Into<String>, vhdx_path: impl Into<PathBuf>) -> Self {
        Self {
            id: format!("tmpl-{}", uuid::Uuid::new_v4()),
            name: name.into(),
            vhdx_path: vhdx_path.into(),
            memory_mb: 4096,
            cpu_count: 2,
            gpu_enabled: false,
            installed_software: vec![],
            created_at: Utc::now(),
            description: None,
        }
    }

    pub fn with_memory(mut self, mb: u64) -> Self {
        self.memory_mb = mb;
        self
    }

    pub fn with_cpus(mut self, count: u32) -> Self {
        self.cpu_count = count;
        self
    }

    pub fn with_gpu(mut self, enabled: bool) -> Self {
        self.gpu_enabled = enabled;
        self
    }

    pub fn with_software(mut self, software: Vec<String>) -> Self {
        self.installed_software = software;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Builder for template registration
#[derive(Debug, Clone, Default)]
pub struct TemplateConfig {
    pub name: String,
    pub vhdx_path: PathBuf,
    pub memory_mb: u64,
    pub cpu_count: u32,
    pub gpu_enabled: bool,
    pub installed_software: Vec<String>,
    pub description: Option<String>,
}

impl TemplateConfig {
    pub fn new(name: impl Into<String>, vhdx_path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            vhdx_path: vhdx_path.into(),
            memory_mb: 4096,
            cpu_count: 2,
            ..Default::default()
        }
    }
}
