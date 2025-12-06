//! Sandbox configuration with builder pattern

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub name: String,
    pub memory_mb: u64,
    pub cpu_count: u32,
    pub gpu_enabled: bool,
    pub networking_enabled: bool,
    pub mapped_folders: Vec<MappedFolder>,
    pub clipboard_enabled: bool,
    pub audio_enabled: bool,
    pub printer_enabled: bool,
    pub startup_command: Option<String>,
    pub writable_layer_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedFolder {
    pub host_path: String,
    pub sandbox_path: String,
    pub read_only: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            memory_mb: 4096,
            cpu_count: 2,
            gpu_enabled: true,
            networking_enabled: true,
            mapped_folders: Vec::new(),
            clipboard_enabled: true,
            audio_enabled: false,
            printer_enabled: false,
            startup_command: None,
            writable_layer_path: None,
        }
    }
}

impl SandboxConfig {
    pub fn builder() -> SandboxConfigBuilder {
        SandboxConfigBuilder::default()
    }

    pub fn validate(&self) -> crate::Result<()> {
        if self.name.is_empty() {
            return Err(crate::Error::Config("name cannot be empty".into()));
        }
        if self.memory_mb < 512 {
            return Err(crate::Error::Config("memory must be at least 512MB".into()));
        }
        if self.cpu_count < 1 {
            return Err(crate::Error::Config("cpu_count must be at least 1".into()));
        }
        Ok(())
    }

    pub fn to_hcs_config(&self) -> serde_json::Value {
        let pipe_name = format!(r"\.\pipe\hcs-sandbox-{}", &self.name);
        let mut devices = serde_json::json!({
            "VideoMonitor": {},
            "Keyboard": {},
            "Mouse": {},
            "EnhancedModeVideo": {
                "ConnectionOptions": {
                    "AccessName": &self.name,
                    "NamedPipe": pipe_name
                }
            }
        });

        if self.gpu_enabled {
            devices["Gpu"] = serde_json::json!({
                "AllowVendorExtension": true
            });
        }

        if self.clipboard_enabled {
            devices["Clipboard"] = serde_json::json!({});
        }

        serde_json::json!({
            "SchemaVersion": { "Major": 2, "Minor": 1 },
            "Owner": "hcs-sandbox",
            "ShouldTerminateOnLastHandleClosed": true,
            "VirtualMachine": {
                "StopOnReset": true,
                "Chipset": { "UseUtc": true },
                "ComputeTopology": {
                    "Memory": { "SizeInMB": self.memory_mb, "AllowOvercommit": true },
                    "Processor": { "Count": self.cpu_count }
                },
                "Devices": devices,
                "GuestState": { "GuestStateFilePath": "", "RuntimeStateFilePath": "" }
            }
        })
    }
}

#[derive(Default)]
pub struct SandboxConfigBuilder {
    config: SandboxConfig,
}

impl SandboxConfigBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    pub fn memory_mb(mut self, mb: u64) -> Self {
        self.config.memory_mb = mb;
        self
    }

    pub fn cpu_count(mut self, count: u32) -> Self {
        self.config.cpu_count = count;
        self
    }

    pub fn gpu_enabled(mut self, enabled: bool) -> Self {
        self.config.gpu_enabled = enabled;
        self
    }

    pub fn networking_enabled(mut self, enabled: bool) -> Self {
        self.config.networking_enabled = enabled;
        self
    }

    pub fn map_folder(mut self, host: impl Into<String>, sandbox: impl Into<String>, read_only: bool) -> Self {
        self.config.mapped_folders.push(MappedFolder {
            host_path: host.into(),
            sandbox_path: sandbox.into(),
            read_only,
        });
        self
    }

    pub fn startup_command(mut self, cmd: impl Into<String>) -> Self {
        self.config.startup_command = Some(cmd.into());
        self
    }

    pub fn build(self) -> SandboxConfig {
        self.config
    }

    pub fn build_validated(self) -> crate::Result<SandboxConfig> {
        let config = self.build();
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = SandboxConfig::builder()
            .name("test-sandbox")
            .memory_mb(8192)
            .cpu_count(4)
            .gpu_enabled(true)
            .build();

        assert_eq!(config.name, "test-sandbox");
        assert_eq!(config.memory_mb, 8192);
        assert_eq!(config.cpu_count, 4);
        assert!(config.gpu_enabled);
    }

    #[test]
    fn test_config_validation() {
        let config = SandboxConfig::builder().build();
        assert!(config.validate().is_err());

        let config = SandboxConfig::builder().name("test").memory_mb(100).build();
        assert!(config.validate().is_err());

        let config = SandboxConfig::builder().name("test").build();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_hcs_config_generation() {
        let config = SandboxConfig::builder()
            .name("test")
            .memory_mb(4096)
            .cpu_count(2)
            .gpu_enabled(true)
            .build();

        let hcs = config.to_hcs_config();
        assert_eq!(hcs["SchemaVersion"]["Major"], 2);
        assert_eq!(hcs["VirtualMachine"]["ComputeTopology"]["Memory"]["SizeInMB"], 4096);
        assert!(hcs["VirtualMachine"]["Devices"]["Gpu"].is_object());
    }

    #[test]
    fn test_config_serialization() {
        let config = SandboxConfig::builder()
            .name("test")
            .map_folder(r"C:\Host", r"C:\Sandbox", true)
            .build();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SandboxConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.mapped_folders.len(), 1);
    }
}
