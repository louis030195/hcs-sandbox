//! Pool management for warm sandboxes
//!
//! Provides Kubernetes-like pool management with pre-warmed sandboxes
//! that can be quickly acquired for task execution.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use crate::config::SandboxConfig;
use crate::{Error, Result};

/// Status of a sandbox in the pool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PooledSandboxStatus {
    /// Warm and ready for acquisition
    Available,
    /// Currently in use by a task
    Acquired,
    /// Being prepared (starting up)
    Warming,
    /// Error state, needs cleanup
    Failed,
}

/// Information about a pooled sandbox
#[derive(Debug, Clone)]
pub struct PooledSandbox {
    pub id: String,
    pub name: String,
    pub status: PooledSandboxStatus,
    pub acquired_at: Option<Instant>,
    pub acquired_by: Option<String>,
    pub vm_id: Option<String>,
}

/// Pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Pool name/identifier
    pub name: String,
    /// Minimum warm sandboxes to maintain
    pub min_warm: usize,
    /// Maximum total sandboxes
    pub max_total: usize,
    /// Sandbox configuration template
    pub sandbox_config: SandboxConfig,
    /// Timeout for sandbox acquisition
    pub acquire_timeout: Duration,
    /// Whether to reset sandboxes on release
    pub reset_on_release: bool,
}

impl PoolConfig {
    pub fn new(name: impl Into<String>, sandbox_config: SandboxConfig) -> Self {
        Self {
            name: name.into(),
            min_warm: 2,
            max_total: 10,
            sandbox_config,
            acquire_timeout: Duration::from_secs(30),
            reset_on_release: true,
        }
    }

    pub fn min_warm(mut self, n: usize) -> Self {
        self.min_warm = n;
        self
    }

    pub fn max_total(mut self, n: usize) -> Self {
        self.max_total = n;
        self
    }

    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = timeout;
        self
    }

    pub fn reset_on_release(mut self, reset: bool) -> Self {
        self.reset_on_release = reset;
        self
    }
}

/// Pool status information
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub name: String,
    pub total: usize,
    pub available: usize,
    pub acquired: usize,
    pub warming: usize,
    pub failed: usize,
}

/// Manages a pool of warm sandboxes
pub struct Pool {
    config: PoolConfig,
    sandboxes: Arc<RwLock<HashMap<String, PooledSandbox>>>,
    base_path: std::path::PathBuf,
}

impl Pool {
    /// Create a new pool
    pub fn new(config: PoolConfig, base_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            config,
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
            base_path: base_path.into(),
        }
    }

    /// Get pool configuration
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// Get current pool status
    pub fn status(&self) -> PoolStatus {
        let sandboxes = self.sandboxes.read();
        PoolStatus {
            name: self.config.name.clone(),
            total: sandboxes.len(),
            available: sandboxes.values().filter(|s| s.status == PooledSandboxStatus::Available).count(),
            acquired: sandboxes.values().filter(|s| s.status == PooledSandboxStatus::Acquired).count(),
            warming: sandboxes.values().filter(|s| s.status == PooledSandboxStatus::Warming).count(),
            failed: sandboxes.values().filter(|s| s.status == PooledSandboxStatus::Failed).count(),
        }
    }

    /// Provision sandboxes up to min_warm
    pub fn warm(&self, orchestrator: &crate::Orchestrator) -> Result<Vec<String>> {
        let status = self.status();
        let needed = self.config.min_warm.saturating_sub(status.available + status.warming);

        if needed == 0 {
            return Ok(Vec::new());
        }

        let mut created = Vec::new();
        for _ in 0..needed {
            let name = format!("{}-{}", self.config.name, uuid::Uuid::new_v4().to_string()[..8].to_string());
            let mut sandbox_config = self.config.sandbox_config.clone();
            sandbox_config.name = name.clone();

            // Mark as warming
            {
                let mut sandboxes = self.sandboxes.write();
                sandboxes.insert(name.clone(), PooledSandbox {
                    id: name.clone(),
                    name: name.clone(),
                    status: PooledSandboxStatus::Warming,
                    acquired_at: None,
                    acquired_by: None,
                    vm_id: None,
                });
            }

            // Create and start the sandbox
            match orchestrator.create_and_start(sandbox_config) {
                Ok(id) => {
                    let mut sandboxes = self.sandboxes.write();
                    if let Some(sb) = sandboxes.get_mut(&name) {
                        sb.vm_id = Some(id.clone());
                        sb.status = PooledSandboxStatus::Available;
                    }
                    created.push(id);
                    tracing::info!(pool = %self.config.name, sandbox = %name, "Sandbox warmed");
                }
                Err(e) => {
                    let mut sandboxes = self.sandboxes.write();
                    if let Some(sb) = sandboxes.get_mut(&name) {
                        sb.status = PooledSandboxStatus::Failed;
                    }
                    tracing::error!(pool = %self.config.name, sandbox = %name, error = %e, "Failed to warm sandbox");
                }
            }
        }

        Ok(created)
    }

    /// Acquire an available sandbox from the pool
    pub fn acquire(&self, task_id: &str) -> Result<PooledSandbox> {
        let mut sandboxes = self.sandboxes.write();

        // Find first available sandbox
        let available = sandboxes.values_mut()
            .find(|s| s.status == PooledSandboxStatus::Available);

        match available {
            Some(sandbox) => {
                sandbox.status = PooledSandboxStatus::Acquired;
                sandbox.acquired_at = Some(Instant::now());
                sandbox.acquired_by = Some(task_id.to_string());
                tracing::info!(pool = %self.config.name, sandbox = %sandbox.name, task = %task_id, "Sandbox acquired");
                Ok(sandbox.clone())
            }
            None => {
                tracing::warn!(pool = %self.config.name, task = %task_id, "No sandbox available");
                Err(Error::NoSandboxAvailable)
            }
        }
    }

    /// Release a sandbox back to the pool
    pub fn release(&self, sandbox_id: &str, _orchestrator: &crate::Orchestrator) -> Result<()> {
        let _vm_id = {
            let sandboxes = self.sandboxes.read();
            sandboxes.get(sandbox_id).and_then(|s| s.vm_id.clone())
        };

        if self.config.reset_on_release {
            // For now, we mark as available without reset
            // In future: pause sandbox, restore checkpoint, resume
            let mut sandboxes = self.sandboxes.write();
            if let Some(sandbox) = sandboxes.get_mut(sandbox_id) {
                sandbox.status = PooledSandboxStatus::Available;
                sandbox.acquired_at = None;
                sandbox.acquired_by = None;
                tracing::info!(pool = %self.config.name, sandbox = %sandbox_id, "Sandbox released");
            }
        } else {
            let mut sandboxes = self.sandboxes.write();
            if let Some(sandbox) = sandboxes.get_mut(sandbox_id) {
                sandbox.status = PooledSandboxStatus::Available;
                sandbox.acquired_at = None;
                sandbox.acquired_by = None;
            }
        }

        Ok(())
    }

    /// Destroy a sandbox and remove from pool
    pub fn destroy(&self, sandbox_id: &str, orchestrator: &crate::Orchestrator) -> Result<()> {
        let vm_id = {
            let sandboxes = self.sandboxes.read();
            sandboxes.get(sandbox_id).and_then(|s| s.vm_id.clone())
        };

        if let Some(id) = vm_id {
            orchestrator.destroy(&id)?;
        }

        self.sandboxes.write().remove(sandbox_id);
        tracing::info!(pool = %self.config.name, sandbox = %sandbox_id, "Sandbox destroyed");
        Ok(())
    }

    /// Destroy all sandboxes in the pool
    pub fn destroy_all(&self, orchestrator: &crate::Orchestrator) -> Vec<Result<()>> {
        let ids: Vec<_> = self.sandboxes.read().keys().cloned().collect();
        ids.iter().map(|id| self.destroy(id, orchestrator)).collect()
    }

    /// List all sandboxes in the pool
    pub fn list(&self) -> Vec<PooledSandbox> {
        self.sandboxes.read().values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config() {
        let sandbox_config = SandboxConfig::builder().name("test").build();
        let pool_config = PoolConfig::new("test-pool", sandbox_config)
            .min_warm(3)
            .max_total(10);

        assert_eq!(pool_config.name, "test-pool");
        assert_eq!(pool_config.min_warm, 3);
        assert_eq!(pool_config.max_total, 10);
    }

    #[test]
    fn test_pool_status() {
        let sandbox_config = SandboxConfig::builder().name("test").build();
        let pool_config = PoolConfig::new("test-pool", sandbox_config);
        let pool = Pool::new(pool_config, "C:\\Test");

        let status = pool.status();
        assert_eq!(status.total, 0);
        assert_eq!(status.available, 0);
    }
}
