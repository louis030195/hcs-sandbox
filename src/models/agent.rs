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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_display() {
        assert_eq!(AgentStatus::Pending.to_string(), "Pending");
        assert_eq!(AgentStatus::Running.to_string(), "Running");
        assert_eq!(AgentStatus::Completed.to_string(), "Completed");
        assert_eq!(AgentStatus::Failed.to_string(), "Failed");
    }

    #[test]
    fn test_task_new_defaults() {
        let t = Task::new("browser-automation");
        assert_eq!(t.workflow, "browser-automation");
        assert_eq!(t.timeout_seconds, 300);
        assert!(!t.requires_gpu);
        assert!(t.input.is_null());
    }

    #[test]
    fn test_task_builder() {
        let input = serde_json::json!({"url": "https://example.com"});
        let t = Task::new("screenshot")
            .with_input(input.clone())
            .with_timeout(60)
            .with_gpu(true);

        assert_eq!(t.timeout_seconds, 60);
        assert!(t.requires_gpu);
        assert_eq!(t.input["url"], "https://example.com");
    }

    #[test]
    fn test_agent_new() {
        let task = Task::new("test-workflow");
        let agent = Agent::new("test-agent", task);

        assert!(agent.id.starts_with("agent-"));
        assert_eq!(agent.name, "test-agent");
        assert_eq!(agent.status, AgentStatus::Pending);
        assert!(agent.pool_id.is_none());
        assert!(agent.vm_id.is_none());
    }

    #[test]
    fn test_agent_with_pool() {
        let task = Task::new("workflow");
        let agent = Agent::new("agent", task).with_pool("pool-123");
        assert_eq!(agent.pool_id, Some("pool-123".to_string()));
    }

    #[test]
    fn test_agent_result() {
        let result = AgentResult {
            success: true,
            output: serde_json::json!({"status": "done"}),
            screenshots: vec!["s1.png".into(), "s2.png".into()],
            duration_seconds: 45,
        };
        
        assert!(result.success);
        assert_eq!(result.screenshots.len(), 2);
        assert_eq!(result.duration_seconds, 45);
    }

    #[test]
    fn test_agent_serialization() {
        let task = Task::new("test");
        let agent = Agent::new("test", task);
        
        let json = serde_json::to_string(&agent).unwrap();
        let parsed: Agent = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.id, agent.id);
        assert_eq!(parsed.status, AgentStatus::Pending);
    }
}
