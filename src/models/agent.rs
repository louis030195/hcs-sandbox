//! Agent/Task model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of an agent/task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Waiting for a VM
    Pending,
    /// VM assigned, preparing
    Scheduled,
    /// Running on VM
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Pending => write!(f, "Pending"),
            AgentStatus::Scheduled => write!(f, "Scheduled"),
            AgentStatus::Running => write!(f, "Running"),
            AgentStatus::Completed => write!(f, "Completed"),
            AgentStatus::Failed => write!(f, "Failed"),
            AgentStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// An automation task/agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Pool to run in (optional, for scheduling)
    pub pool_id: Option<String>,
    /// Assigned VM (once scheduled)
    pub vm_id: Option<String>,
    /// Current status
    pub status: AgentStatus,
    /// Task definition
    pub task: Task,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// When scheduled to a VM
    pub scheduled_at: Option<DateTime<Utc>>,
    /// When started executing
    pub started_at: Option<DateTime<Utc>>,
    /// When completed/failed
    pub completed_at: Option<DateTime<Utc>>,
    /// Result (on completion)
    pub result: Option<AgentResult>,
    /// Error message (on failure)
    pub error_message: Option<String>,
}

impl Agent {
    pub fn new(name: impl Into<String>, task: Task) -> Self {
        Self {
            id: format!("agent-{}", uuid::Uuid::new_v4()),
            name: name.into(),
            pool_id: None,
            vm_id: None,
            status: AgentStatus::Pending,
            task,
            created_at: Utc::now(),
            scheduled_at: None,
            started_at: None,
            completed_at: None,
            result: None,
            error_message: None,
        }
    }

    pub fn with_pool(mut self, pool_id: impl Into<String>) -> Self {
        self.pool_id = Some(pool_id.into());
        self
    }
}

/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Workflow name/type
    pub workflow: String,
    /// Input parameters
    pub input: serde_json::Value,
    /// Timeout in seconds
    pub timeout_seconds: u64,
    /// Whether GPU is required
    pub requires_gpu: bool,
}

impl Task {
    pub fn new(workflow: impl Into<String>) -> Self {
        Self {
            workflow: workflow.into(),
            input: serde_json::Value::Null,
            timeout_seconds: 300,
            requires_gpu: false,
        }
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn with_gpu(mut self, required: bool) -> Self {
        self.requires_gpu = required;
        self
    }
}

/// Result of agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Whether task succeeded
    pub success: bool,
    /// Output data
    pub output: serde_json::Value,
    /// Screenshots taken during execution
    pub screenshots: Vec<String>,
    /// Duration in seconds
    pub duration_seconds: u64,
}
