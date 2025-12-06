//! API request/response types

use serde::{Deserialize, Serialize};

// === Templates ===

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub vhdx_path: String,
    #[serde(default = "default_memory")]
    pub memory_mb: u64,
    #[serde(default = "default_cpus")]
    pub cpu_count: u32,
    #[serde(default)]
    pub gpu_enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_memory() -> u64 { 4096 }
fn default_cpus() -> u32 { 2 }

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateResponse {
    pub id: String,
    pub name: String,
    pub vhdx_path: String,
    pub memory_mb: u64,
    pub cpu_count: u32,
    pub gpu_enabled: bool,
    pub description: Option<String>,
    pub created_at: String,
}

// === Pools ===

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePoolRequest {
    pub name: String,
    pub template_name: String,
    #[serde(default = "default_count")]
    pub desired_count: usize,
    #[serde(default = "default_warm")]
    pub warm_count: usize,
}

fn default_count() -> usize { 3 }
fn default_warm() -> usize { 1 }

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolResponse {
    pub id: String,
    pub name: String,
    pub template_id: String,
    pub desired_count: usize,
    pub warm_count: usize,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolStatusResponse {
    pub id: String,
    pub name: String,
    pub template_id: String,
    pub desired_count: usize,
    pub total_vms: usize,
    pub running_vms: usize,
    pub saved_vms: usize,
    pub off_vms: usize,
    pub error_vms: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProvisionRequest {
    #[serde(default = "default_provision_count")]
    pub count: usize,
}

fn default_provision_count() -> usize { 1 }

// === VMs ===

#[derive(Debug, Serialize, Deserialize)]
pub struct VMResponse {
    pub id: String,
    pub name: String,
    pub template_id: Option<String>,
    pub pool_id: Option<String>,
    pub state: String,
    pub ip_address: Option<String>,
    pub memory_mb: u64,
    pub cpu_count: u32,
    pub gpu_enabled: bool,
    pub created_at: String,
    pub last_resumed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResumeResponse {
    pub vm_id: String,
    pub vm_name: String,
    pub ip_address: String,
    pub resume_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AcquireVMRequest {
    pub pool_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseVMRequest {
    #[serde(default)]
    pub reset: bool,
}

// === Agents ===

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub pool_name: Option<String>,
    pub workflow: String,
    #[serde(default)]
    pub input: serde_json::Value,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub requires_gpu: bool,
}

fn default_timeout() -> u64 { 300 }

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub name: String,
    pub pool_id: Option<String>,
    pub vm_id: Option<String>,
    pub status: String,
    pub workflow: String,
    pub created_at: String,
    pub scheduled_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

// === Generic ===

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiSuccess {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}
