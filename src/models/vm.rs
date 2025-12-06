//! VM model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// State of a VM
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VMState {
    /// Never started or fully shutdown
    Off,
    /// Currently executing
    Running,
    /// State saved to disk, ready for fast resume (2-5s)
    Saved,
    /// Paused in memory
    Paused,
    /// Something went wrong
    Error,
}

impl std::fmt::Display for VMState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VMState::Off => write!(f, "Off"),
            VMState::Running => write!(f, "Running"),
            VMState::Saved => write!(f, "Saved"),
            VMState::Paused => write!(f, "Paused"),
            VMState::Error => write!(f, "Error"),
        }
    }
}

impl VMState {
    pub fn from_hyperv_state(state: i32) -> Self {
        match state {
            2 => VMState::Off,
            3 => VMState::Running,
            6 => VMState::Saved,
            9 => VMState::Paused,
            _ => VMState::Error,
        }
    }
}

/// A VM instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VM {
    /// Unique identifier
    pub id: String,
    /// Hyper-V VM name
    pub name: String,
    /// Template this VM was created from
    pub template_id: Option<String>,
    /// Pool this VM belongs to
    pub pool_id: Option<String>,
    /// Current state
    pub state: VMState,
    /// Path to VHDX file
    pub vhdx_path: PathBuf,
    /// IP address (if running and known)
    pub ip_address: Option<String>,
    /// Memory in MB
    pub memory_mb: u64,
    /// CPU count
    pub cpu_count: u32,
    /// GPU enabled
    pub gpu_enabled: bool,
    /// Currently assigned agent
    pub current_agent_id: Option<String>,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last resume time
    pub last_resumed_at: Option<DateTime<Utc>>,
    /// Error message if in error state
    pub error_message: Option<String>,
}

impl VM {
    pub fn new(name: String, vhdx_path: PathBuf, memory_mb: u64, cpu_count: u32) -> Self {
        Self {
            id: format!("vm-{}", uuid::Uuid::new_v4()),
            name,
            template_id: None,
            pool_id: None,
            state: VMState::Off,
            vhdx_path,
            ip_address: None,
            memory_mb,
            cpu_count,
            gpu_enabled: false,
            current_agent_id: None,
            created_at: Utc::now(),
            last_resumed_at: None,
            error_message: None,
        }
    }

    pub fn is_available(&self) -> bool {
        self.state == VMState::Saved && self.current_agent_id.is_none()
    }
}

/// Builder for VM configuration
#[derive(Debug, Clone, Default)]
pub struct VMConfig {
    pub name: String,
    pub template_id: Option<String>,
    pub pool_id: Option<String>,
    pub vhdx_path: Option<PathBuf>,
    pub memory_mb: u64,
    pub cpu_count: u32,
    pub gpu_enabled: bool,
}

impl VMConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            memory_mb: 4096,
            cpu_count: 2,
            ..Default::default()
        }
    }

    pub fn template(mut self, id: impl Into<String>) -> Self {
        self.template_id = Some(id.into());
        self
    }

    pub fn pool(mut self, id: impl Into<String>) -> Self {
        self.pool_id = Some(id.into());
        self
    }

    pub fn vhdx_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.vhdx_path = Some(path.into());
        self
    }

    pub fn memory_mb(mut self, mb: u64) -> Self {
        self.memory_mb = mb;
        self
    }

    pub fn cpu_count(mut self, count: u32) -> Self {
        self.cpu_count = count;
        self
    }

    pub fn gpu(mut self, enabled: bool) -> Self {
        self.gpu_enabled = enabled;
        self
    }
}
