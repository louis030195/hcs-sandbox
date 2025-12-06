//! Task scheduler with acquire/execute/release pattern
//!
//! Provides high-level workflow execution on pooled sandboxes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use crate::hvsocket::{AgentClient, AgentMessage, AgentResponse, HvSocketAddr};
use crate::pool::{Pool, PooledSandbox};
use crate::{Error, Orchestrator, Result};

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Acquiring,
    Running,
    Completed,
    Failed,
}

/// A task to be executed in a sandbox
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub workflow_yaml: String,
    pub timeout: Duration,
    pub created_at: Instant,
}

impl Task {
    pub fn new(workflow_yaml: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workflow_yaml: workflow_yaml.into(),
            timeout: Duration::from_secs(300), // 5 min default
            created_at: Instant::now(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Result of task execution
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub sandbox_id: Option<String>,
    pub response: Option<AgentResponse>,
    pub error: Option<String>,
    pub duration: Duration,
}

/// Scheduler for executing tasks on pooled sandboxes
pub struct Scheduler {
    pool: Arc<Pool>,
    orchestrator: Arc<Orchestrator>,
    active_tasks: Arc<RwLock<HashMap<String, TaskExecution>>>,
}

struct TaskExecution {
    task: Task,
    sandbox: PooledSandbox,
    status: TaskStatus,
    started_at: Instant,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(pool: Arc<Pool>, orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            pool,
            orchestrator,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute a task: acquire sandbox, run workflow, release
    pub fn execute(&self, task: Task) -> Result<TaskResult> {
        let task_id = task.id.clone();
        let start = Instant::now();

        tracing::info!(task = %task_id, "Executing task");

        // 1. ACQUIRE - Get a sandbox from the pool
        tracing::debug!(task = %task_id, "Acquiring sandbox");
        let sandbox = match self.pool.acquire(&task_id) {
            Ok(sb) => sb,
            Err(e) => {
                tracing::error!(task = %task_id, error = %e, "Failed to acquire sandbox");
                return Ok(TaskResult {
                    task_id,
                    status: TaskStatus::Failed,
                    sandbox_id: None,
                    response: None,
                    error: Some(format!("Failed to acquire sandbox: {}", e)),
                    duration: start.elapsed(),
                });
            }
        };

        let sandbox_id = sandbox.id.clone();
        tracing::info!(task = %task_id, sandbox = %sandbox_id, "Sandbox acquired");

        // Track active task
        {
            let mut tasks = self.active_tasks.write();
            tasks.insert(task_id.clone(), TaskExecution {
                task: task.clone(),
                sandbox: sandbox.clone(),
                status: TaskStatus::Running,
                started_at: Instant::now(),
            });
        }

        // 2. EXECUTE - Run the workflow via HvSocket
        let result = self.execute_workflow(&task, &sandbox);

        // 3. RELEASE - Return sandbox to pool
        tracing::debug!(task = %task_id, sandbox = %sandbox_id, "Releasing sandbox");
        if let Err(e) = self.pool.release(&sandbox_id, &self.orchestrator) {
            tracing::error!(task = %task_id, sandbox = %sandbox_id, error = %e, "Failed to release sandbox");
        }

        // Remove from active tasks
        self.active_tasks.write().remove(&task_id);

        let duration = start.elapsed();
        tracing::info!(task = %task_id, duration_ms = duration.as_millis(), "Task completed");

        match result {
            Ok(response) => Ok(TaskResult {
                task_id,
                status: TaskStatus::Completed,
                sandbox_id: Some(sandbox_id),
                response: Some(response),
                error: None,
                duration,
            }),
            Err(e) => Ok(TaskResult {
                task_id,
                status: TaskStatus::Failed,
                sandbox_id: Some(sandbox_id),
                response: None,
                error: Some(e.to_string()),
                duration,
            }),
        }
    }

    /// Execute workflow on a sandbox via HvSocket
    fn execute_workflow(&self, task: &Task, sandbox: &PooledSandbox) -> Result<AgentResponse> {
        // Get VM ID for HvSocket connection
        let vm_id = sandbox.vm_id.as_ref()
            .ok_or_else(|| Error::HvSocket("No VM ID for sandbox".into()))?;

        // Create HvSocket client
        let addr = HvSocketAddr::agent(vm_id);
        let client = AgentClient::new(addr)
            .with_timeout(task.timeout);

        // Connect to agent
        client.connect()?;

        // Send workflow
        let msg = AgentMessage::workflow(&task.workflow_yaml);
        let response = client.send(&msg)?;

        if response.success {
            Ok(response)
        } else {
            Err(Error::HvSocket(response.error.unwrap_or_else(|| "Unknown error".into())))
        }
    }

    /// Get active task count
    pub fn active_count(&self) -> usize {
        self.active_tasks.read().len()
    }

    /// Get list of active task IDs
    pub fn active_tasks(&self) -> Vec<String> {
        self.active_tasks.read().keys().cloned().collect()
    }

    /// Cancel a running task (best effort)
    pub fn cancel(&self, task_id: &str) -> Result<()> {
        let execution = self.active_tasks.write().remove(task_id);
        if let Some(exec) = execution {
            // Release the sandbox
            self.pool.release(&exec.sandbox.id, &self.orchestrator)?;
            tracing::info!(task = %task_id, "Task cancelled");
        }
        Ok(())
    }
}

/// High-level execute function for one-off tasks
pub fn execute_workflow(
    orchestrator: &Orchestrator,
    pool: &Pool,
    workflow_yaml: &str,
    timeout: Duration,
) -> Result<TaskResult> {
    let scheduler = Scheduler::new(
        Arc::new(Pool::new(pool.config().clone(), ".")),
        Arc::new(Orchestrator::new()?),
    );

    let task = Task::new(workflow_yaml).with_timeout(timeout);
    scheduler.execute(task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("steps:\n  - click: button");
        assert!(!task.id.is_empty());
        assert_eq!(task.timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_task_with_timeout() {
        let task = Task::new("steps:\n  - click: button")
            .with_timeout(Duration::from_secs(60));
        assert_eq!(task.timeout, Duration::from_secs(60));
    }
}
