//! HvSocket (AF_HYPERV) communication bindings
//!
//! Provides host↔guest communication without networking using Hyper-V sockets.
//! This allows the orchestrator to communicate with agents running inside sandboxes.

use std::time::Duration;
use crate::Result;

/// Well-known HvSocket service GUIDs
pub mod service_ids {
    /// Wildcard - matches any service
    pub const WILDCARD: &str = "00000000-0000-0000-0000-000000000000";
    /// Parent partition (host)
    pub const PARENT: &str = "a42e7cda-d03f-480c-9cc2-a4de20abb878";
    /// Default agent service ID (custom)
    pub const AGENT: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
}

/// HvSocket address for connecting to a VM
#[derive(Debug, Clone)]
pub struct HvSocketAddr {
    /// VM GUID (from HCS compute system)
    pub vm_id: String,
    /// Service GUID
    pub service_id: String,
}

impl HvSocketAddr {
    pub fn new(vm_id: impl Into<String>, service_id: impl Into<String>) -> Self {
        Self {
            vm_id: vm_id.into(),
            service_id: service_id.into(),
        }
    }

    /// Create address for connecting to agent service
    pub fn agent(vm_id: impl Into<String>) -> Self {
        Self::new(vm_id, service_ids::AGENT)
    }
}

/// Message protocol for agent communication
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentMessage {
    /// Message type
    pub msg_type: String,
    /// Payload (JSON)
    pub payload: serde_json::Value,
    /// Request ID for correlation
    pub request_id: Option<String>,
}

impl AgentMessage {
    pub fn new(msg_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            msg_type: msg_type.into(),
            payload,
            request_id: Some(uuid::Uuid::new_v4().to_string()),
        }
    }

    /// Create a ping message
    pub fn ping() -> Self {
        Self::new("ping", serde_json::json!({}))
    }

    /// Create an execute command message
    pub fn execute(command: &str, args: &[&str]) -> Self {
        Self::new("execute", serde_json::json!({
            "command": command,
            "args": args,
        }))
    }

    /// Create a workflow execute message
    pub fn workflow(workflow_yaml: &str) -> Self {
        Self::new("workflow", serde_json::json!({
            "yaml": workflow_yaml,
        }))
    }
}

/// Response from agent
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentResponse {
    pub success: bool,
    pub request_id: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Client for communicating with an agent in a sandbox via HvSocket
///
/// Note: This is a placeholder implementation. Actual HvSocket support
/// requires Windows-specific socket code using AF_HYPERV (34).
pub struct AgentClient {
    addr: HvSocketAddr,
    timeout: Duration,
}

impl AgentClient {
    pub fn new(addr: HvSocketAddr) -> Self {
        Self {
            addr,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Connect to the agent (placeholder - needs Windows HvSocket impl)
    pub fn connect(&self) -> Result<()> {
        // TODO: Implement actual HvSocket connection
        // This requires:
        // 1. Create socket with AF_HYPERV (34)
        // 2. Set up SOCKADDR_HV with VM ID and Service ID
        // 3. Connect
        tracing::info!(vm_id = %self.addr.vm_id, service = %self.addr.service_id, "Would connect to agent");
        Ok(())
    }

    /// Send a message and wait for response (placeholder)
    pub fn send(&self, msg: &AgentMessage) -> Result<AgentResponse> {
        // TODO: Implement actual send/receive over HvSocket
        tracing::info!(msg_type = %msg.msg_type, "Would send message to agent");
        Ok(AgentResponse {
            success: true,
            request_id: msg.request_id.clone(),
            result: Some(serde_json::json!({"status": "placeholder"})),
            error: None,
        })
    }

    /// Ping the agent to check if it's alive
    pub fn ping(&self) -> Result<bool> {
        let response = self.send(&AgentMessage::ping())?;
        Ok(response.success)
    }

    /// Execute a workflow on the agent
    pub fn execute_workflow(&self, workflow_yaml: &str) -> Result<AgentResponse> {
        self.send(&AgentMessage::workflow(workflow_yaml))
    }
}

/// Listener for incoming HvSocket connections (for agent side)
pub struct HvSocketListener {
    service_id: String,
}

impl HvSocketListener {
    pub fn new(service_id: impl Into<String>) -> Self {
        Self {
            service_id: service_id.into(),
        }
    }

    /// Bind and listen (placeholder)
    pub fn bind(&self) -> Result<()> {
        // TODO: Implement actual HvSocket bind/listen
        tracing::info!(service = %self.service_id, "Would bind HvSocket listener");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hvsocket_addr() {
        let addr = HvSocketAddr::agent("12345678-1234-1234-1234-123456789abc");
        assert_eq!(addr.service_id, service_ids::AGENT);
    }

    #[test]
    fn test_agent_message() {
        let msg = AgentMessage::ping();
        assert_eq!(msg.msg_type, "ping");
        assert!(msg.request_id.is_some());
    }

    #[test]
    fn test_execute_message() {
        let msg = AgentMessage::execute("notepad.exe", &[]);
        assert_eq!(msg.msg_type, "execute");
    }
}
