//! Individual sandbox management

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use crate::config::SandboxConfig;
use crate::hcs::{ComputeSystem, Layer};
use crate::{Error, Result};

/// State of a sandbox
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxState {
    Creating,
    Created,
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Error,
}

impl std::fmt::Display for SandboxState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxState::Creating => write!(f, "Creating"),
            SandboxState::Created => write!(f, "Created"),
            SandboxState::Starting => write!(f, "Starting"),
            SandboxState::Running => write!(f, "Running"),
            SandboxState::Paused => write!(f, "Paused"),
            SandboxState::Stopping => write!(f, "Stopping"),
            SandboxState::Stopped => write!(f, "Stopped"),
            SandboxState::Error => write!(f, "Error"),
        }
    }
}

/// A sandbox instance
pub struct Sandbox {
    id: String,
    config: SandboxConfig,
    state: Arc<RwLock<SandboxState>>,
    compute_system: Option<ComputeSystem>,
    layer_path: PathBuf,
    vhdx_path: PathBuf,
}

impl Sandbox {
    /// Create a new sandbox (does not start it)
    pub fn new(config: SandboxConfig, base_path: &Path) -> Result<Self> {
        config.validate()?;

        let id = format!("hcs-sandbox-{}", uuid::Uuid::new_v4());
        let sandbox_dir = base_path.join(&config.name);
        let layer_path = sandbox_dir.join("layer");
        let vhdx_path = config.writable_layer_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| sandbox_dir.join("writable.vhdx"));

        Ok(Self {
            id,
            config,
            state: Arc::new(RwLock::new(SandboxState::Creating)),
            compute_system: None,
            layer_path,
            vhdx_path,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn state(&self) -> SandboxState {
        *self.state.read()
    }

    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Initialize the sandbox (create layers, VHD, etc.)
    pub fn initialize(&mut self) -> Result<()> {
        self.set_state(SandboxState::Creating);

        std::fs::create_dir_all(self.layer_path.parent().unwrap())?;
        std::fs::create_dir_all(&self.layer_path)?;

        if !self.vhdx_path.exists() {
            tracing::info!("Creating writable VHDX at {:?}", self.vhdx_path);
            crate::hcs::layer::vhd::create(
                self.vhdx_path.to_str().unwrap(),
                20,
            )?;
        }

        self.set_state(SandboxState::Created);
        Ok(())
    }

    /// Create the HCS compute system
    pub fn create_compute_system(&mut self) -> Result<()> {
        let hcs_config = self.config.to_hcs_config();
        let config_json = serde_json::to_string(&hcs_config)?;

        tracing::info!("Creating compute system: {}", self.id);

        let cs = ComputeSystem::create(&self.id, &config_json)?;
        self.compute_system = Some(cs);
        self.set_state(SandboxState::Created);

        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        self.ensure_state(SandboxState::Created)?;
        self.set_state(SandboxState::Starting);

        if let Some(ref cs) = self.compute_system {
            cs.start()?;
            self.set_state(SandboxState::Running);
            Ok(())
        } else {
            self.set_state(SandboxState::Error);
            Err(Error::InvalidState {
                current: "no compute system".into(),
                expected: "compute system created".into(),
            })
        }
    }

    pub fn pause(&mut self) -> Result<()> {
        self.ensure_state(SandboxState::Running)?;

        if let Some(ref cs) = self.compute_system {
            cs.pause()?;
            self.set_state(SandboxState::Paused);
            Ok(())
        } else {
            Err(Error::InvalidState {
                current: "no compute system".into(),
                expected: "compute system exists".into(),
            })
        }
    }

    pub fn resume(&mut self) -> Result<()> {
        self.ensure_state(SandboxState::Paused)?;

        if let Some(ref cs) = self.compute_system {
            cs.resume()?;
            self.set_state(SandboxState::Running);
            Ok(())
        } else {
            Err(Error::InvalidState {
                current: "no compute system".into(),
                expected: "compute system exists".into(),
            })
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        let state = self.state();
        if state != SandboxState::Running && state != SandboxState::Paused {
            return Err(Error::InvalidState {
                current: state.to_string(),
                expected: "Running or Paused".into(),
            });
        }

        self.set_state(SandboxState::Stopping);

        if let Some(ref cs) = self.compute_system {
            cs.terminate()?;
            self.set_state(SandboxState::Stopped);
            Ok(())
        } else {
            self.set_state(SandboxState::Error);
            Err(Error::InvalidState {
                current: "no compute system".into(),
                expected: "compute system exists".into(),
            })
        }
    }

    pub fn destroy(mut self) -> Result<()> {
        let state = self.state();
        if state == SandboxState::Running || state == SandboxState::Paused {
            self.stop()?;
        }

        self.compute_system = None;

        if self.layer_path.exists() {
            let _ = Layer::destroy(self.layer_path.to_str().unwrap());
            let _ = std::fs::remove_dir_all(&self.layer_path);
        }

        if self.vhdx_path.exists() {
            let _ = crate::hcs::layer::vhd::dismount(self.vhdx_path.to_str().unwrap());
            let _ = std::fs::remove_file(&self.vhdx_path);
        }

        Ok(())
    }

    pub fn get_properties(&self) -> Result<serde_json::Value> {
        if let Some(ref cs) = self.compute_system {
            let props = cs.get_properties(None)?;
            Ok(serde_json::from_str(&props)?)
        } else {
            Ok(serde_json::json!({
                "id": self.id,
                "name": self.config.name,
                "state": self.state().to_string(),
            }))
        }
    }

    pub fn checkpoint(&self, path: &str) -> Result<()> {
        self.ensure_state(SandboxState::Running)?;

        if let Some(ref cs) = self.compute_system {
            let options = serde_json::json!({ "SaveStateFilePath": path });
            cs.save(Some(&options.to_string()))?;
            Ok(())
        } else {
            Err(Error::InvalidState {
                current: "no compute system".into(),
                expected: "compute system exists".into(),
            })
        }
    }

    fn set_state(&self, state: SandboxState) {
        *self.state.write() = state;
    }

    fn ensure_state(&self, expected: SandboxState) -> Result<()> {
        let current = self.state();
        if current != expected {
            Err(Error::InvalidState {
                current: current.to_string(),
                expected: expected.to_string(),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_state_display() {
        assert_eq!(SandboxState::Running.to_string(), "Running");
        assert_eq!(SandboxState::Stopped.to_string(), "Stopped");
    }

    #[test]
    fn test_sandbox_new() {
        let config = SandboxConfig::builder()
            .name("test-sandbox")
            .memory_mb(4096)
            .build();

        let base_path = Path::new("C:\\Sandboxes");
        let sandbox = Sandbox::new(config, base_path).unwrap();

        assert!(sandbox.id().starts_with("hcs-sandbox-"));
        assert_eq!(sandbox.name(), "test-sandbox");
        assert_eq!(sandbox.state(), SandboxState::Creating);
    }

    #[test]
    fn test_sandbox_state_transitions() {
        let config = SandboxConfig::builder()
            .name("test")
            .build();

        let sandbox = Sandbox::new(config, Path::new("C:\\Test")).unwrap();
        assert_eq!(sandbox.state(), SandboxState::Creating);

        let mut sandbox = sandbox;
        assert!(sandbox.start().is_err());
    }
}
