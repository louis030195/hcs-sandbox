//! Orchestrator for managing multiple sandboxes

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::config::SandboxConfig;
use crate::sandbox::{Sandbox, SandboxState};
use crate::{Error, Result};

/// Manages multiple sandbox instances
pub struct Orchestrator {
    sandboxes: Arc<RwLock<HashMap<String, Sandbox>>>,
    base_path: PathBuf,
    max_sandboxes: usize,
}

impl Orchestrator {
    /// Create a new orchestrator with default settings
    pub fn new() -> Result<Self> {
        Self::with_config(OrchestratorConfig::default())
    }

    /// Create a new orchestrator with custom config
    pub fn with_config(config: OrchestratorConfig) -> Result<Self> {
        // Create base directory if it doesn't exist
        std::fs::create_dir_all(&config.base_path)?;

        Ok(Self {
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
            base_path: config.base_path,
            max_sandboxes: config.max_sandboxes,
        })
    }

    /// Create a new sandbox (does not start it)
    pub fn create(&self, config: SandboxConfig) -> Result<String> {
        let sandboxes = self.sandboxes.read();

        // Check limits
        if sandboxes.len() >= self.max_sandboxes {
            return Err(Error::Config(format!(
                "Maximum sandbox limit ({}) reached",
                self.max_sandboxes
            )));
        }

        // Check for duplicate name
        if sandboxes.values().any(|s| s.name() == config.name) {
            return Err(Error::SandboxAlreadyExists(config.name.clone()));
        }

        drop(sandboxes);

        // Create the sandbox
        let sandbox = Sandbox::new(config, &self.base_path)?;
        let id = sandbox.id().to_string();

        self.sandboxes.write().insert(id.clone(), sandbox);
        Ok(id)
    }

    /// Initialize a sandbox (create VHD, layers, etc.)
    pub fn initialize(&self, id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write();
        let sandbox = sandboxes.get_mut(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        sandbox.initialize()
    }

    /// Create the HCS compute system for a sandbox
    pub fn create_compute_system(&self, id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write();
        let sandbox = sandboxes.get_mut(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        sandbox.create_compute_system()
    }

    /// Start a sandbox
    pub fn start(&self, id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write();
        let sandbox = sandboxes.get_mut(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        sandbox.start()
    }

    /// Stop a sandbox
    pub fn stop(&self, id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write();
        let sandbox = sandboxes.get_mut(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        sandbox.stop()
    }

    /// Pause a sandbox
    pub fn pause(&self, id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write();
        let sandbox = sandboxes.get_mut(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        sandbox.pause()
    }

    /// Resume a paused sandbox
    pub fn resume(&self, id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write();
        let sandbox = sandboxes.get_mut(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        sandbox.resume()
    }

    /// Destroy a sandbox and clean up resources
    pub fn destroy(&self, id: &str) -> Result<()> {
        let sandbox = self.sandboxes.write().remove(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        sandbox.destroy()
    }

    /// Get sandbox state
    pub fn get_state(&self, id: &str) -> Result<SandboxState> {
        let sandboxes = self.sandboxes.read();
        let sandbox = sandboxes.get(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        Ok(sandbox.state())
    }

    /// Get sandbox properties
    pub fn get_properties(&self, id: &str) -> Result<serde_json::Value> {
        let sandboxes = self.sandboxes.read();
        let sandbox = sandboxes.get(id)
            .ok_or_else(|| Error::SandboxNotFound(id.to_string()))?;
        sandbox.get_properties()
    }

    /// List all sandbox IDs
    pub fn list(&self) -> Vec<String> {
        self.sandboxes.read().keys().cloned().collect()
    }

    /// List all sandboxes with their states
    pub fn list_with_state(&self) -> Vec<SandboxInfo> {
        self.sandboxes.read()
            .iter()
            .map(|(id, sandbox)| SandboxInfo {
                id: id.clone(),
                name: sandbox.name().to_string(),
                state: sandbox.state(),
            })
            .collect()
    }

    /// Get count of sandboxes
    pub fn count(&self) -> usize {
        self.sandboxes.read().len()
    }

    /// Get count of running sandboxes
    pub fn running_count(&self) -> usize {
        self.sandboxes.read()
            .values()
            .filter(|s| s.state() == SandboxState::Running)
            .count()
    }

    /// Stop all sandboxes
    pub fn stop_all(&self) -> Vec<Result<()>> {
        let ids: Vec<_> = self.list();
        ids.iter()
            .map(|id| self.stop(id))
            .collect()
    }

    /// Destroy all sandboxes
    pub fn destroy_all(&self) -> Vec<Result<()>> {
        let ids: Vec<_> = self.list();
        ids.iter()
            .map(|id| self.destroy(id))
            .collect()
    }

    /// Create and start a sandbox in one call
    pub fn create_and_start(&self, config: SandboxConfig) -> Result<String> {
        let id = self.create(config)?;

        if let Err(e) = self.initialize(&id) {
            let _ = self.destroy(&id);
            return Err(e);
        }

        if let Err(e) = self.create_compute_system(&id) {
            let _ = self.destroy(&id);
            return Err(e);
        }

        if let Err(e) = self.start(&id) {
            let _ = self.destroy(&id);
            return Err(e);
        }

        Ok(id)
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new().expect("Failed to create default orchestrator")
    }
}

/// Configuration for the orchestrator
pub struct OrchestratorConfig {
    /// Base path for sandbox data
    pub base_path: PathBuf,
    /// Maximum number of sandboxes
    pub max_sandboxes: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from(r"C:\HcsSandboxes"),
            max_sandboxes: 20,
        }
    }
}

impl OrchestratorConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn base_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.base_path = path.into();
        self
    }

    pub fn max_sandboxes(mut self, max: usize) -> Self {
        self.max_sandboxes = max;
        self
    }
}

/// Information about a sandbox
#[derive(Debug, Clone)]
pub struct SandboxInfo {
    pub id: String,
    pub name: String,
    pub state: SandboxState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_orchestrator() -> (Orchestrator, TempDir) {
        let temp = TempDir::new().unwrap();
        let config = OrchestratorConfig::new()
            .base_path(temp.path())
            .max_sandboxes(5);
        let orch = Orchestrator::with_config(config).unwrap();
        (orch, temp)
    }

    #[test]
    fn test_orchestrator_create() {
        let (orch, _temp) = test_orchestrator();

        let config = SandboxConfig::builder()
            .name("test-sandbox")
            .build();

        let id = orch.create(config).unwrap();
        assert!(id.starts_with("hcs-sandbox-"));
        assert_eq!(orch.count(), 1);
    }

    #[test]
    fn test_orchestrator_duplicate_name() {
        let (orch, _temp) = test_orchestrator();

        let config1 = SandboxConfig::builder().name("test").build();
        let config2 = SandboxConfig::builder().name("test").build();

        orch.create(config1).unwrap();
        let result = orch.create(config2);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::SandboxAlreadyExists(_)));
    }

    #[test]
    fn test_orchestrator_max_sandboxes() {
        let (orch, _temp) = test_orchestrator();

        // Create max sandboxes
        for i in 0..5 {
            let config = SandboxConfig::builder()
                .name(format!("sandbox-{}", i))
                .build();
            orch.create(config).unwrap();
        }

        // Try to create one more
        let config = SandboxConfig::builder().name("overflow").build();
        let result = orch.create(config);

        assert!(result.is_err());
    }

    #[test]
    fn test_orchestrator_list() {
        let (orch, _temp) = test_orchestrator();

        let config1 = SandboxConfig::builder().name("sandbox-1").build();
        let config2 = SandboxConfig::builder().name("sandbox-2").build();

        orch.create(config1).unwrap();
        orch.create(config2).unwrap();

        let list = orch.list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_orchestrator_destroy() {
        let (orch, _temp) = test_orchestrator();

        let config = SandboxConfig::builder().name("test").build();
        let id = orch.create(config).unwrap();

        assert_eq!(orch.count(), 1);
        orch.destroy(&id).unwrap();
        assert_eq!(orch.count(), 0);
    }

    #[test]
    fn test_orchestrator_get_state() {
        let (orch, _temp) = test_orchestrator();

        let config = SandboxConfig::builder().name("test").build();
        let id = orch.create(config).unwrap();

        let state = orch.get_state(&id).unwrap();
        assert_eq!(state, SandboxState::Creating);
    }

    #[test]
    fn test_orchestrator_not_found() {
        let (orch, _temp) = test_orchestrator();

        let result = orch.get_state("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::SandboxNotFound(_)));
    }
}
