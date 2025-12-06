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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_new_defaults() {
        let t = Template::new("win11", r"C:\templates\win11.vhdx");
        assert!(t.id.starts_with("tmpl-"));
        assert_eq!(t.name, "win11");
        assert_eq!(t.memory_mb, 4096);
        assert_eq!(t.cpu_count, 2);
        assert!(!t.gpu_enabled);
        assert!(t.installed_software.is_empty());
        assert!(t.description.is_none());
    }

    #[test]
    fn test_template_builder() {
        let t = Template::new("win11-chrome", r"C:\templates\win11.vhdx")
            .with_memory(8192)
            .with_cpus(4)
            .with_gpu(true)
            .with_software(vec!["Chrome".into(), "Node.js".into()])
            .with_description("Windows 11 with Chrome");

        assert_eq!(t.memory_mb, 8192);
        assert_eq!(t.cpu_count, 4);
        assert!(t.gpu_enabled);
        assert_eq!(t.installed_software.len(), 2);
        assert_eq!(t.description, Some("Windows 11 with Chrome".to_string()));
    }

    #[test]
    fn test_template_serialization() {
        let t = Template::new("test", r"C:\test.vhdx");
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        
        let parsed: Template = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, t.name);
        assert_eq!(parsed.id, t.id);
    }

    #[test]
    fn test_template_config() {
        let cfg = TemplateConfig::new("win11", r"C:\test.vhdx");
        assert_eq!(cfg.memory_mb, 4096);
        assert_eq!(cfg.cpu_count, 2);
    }
}
