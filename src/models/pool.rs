//! VM Pool model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A pool of VMs from the same template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMPool {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Template to create VMs from
    pub template_id: String,
    /// Desired number of VMs in pool
    pub desired_count: usize,
    /// Number of VMs to keep in Saved state (warm pool)
    pub warm_count: usize,
    /// Maximum VMs per host
    pub max_per_host: usize,
    /// Creation time
    pub created_at: DateTime<Utc>,
}

impl VMPool {
    pub fn new(name: impl Into<String>, template_id: impl Into<String>) -> Self {
        Self {
            id: format!("pool-{}", uuid::Uuid::new_v4()),
            name: name.into(),
            template_id: template_id.into(),
            desired_count: 3,
            warm_count: 1,
            max_per_host: 10,
            created_at: Utc::now(),
        }
    }

    pub fn with_count(mut self, desired: usize) -> Self {
        self.desired_count = desired;
        self
    }

    pub fn with_warm_count(mut self, warm: usize) -> Self {
        self.warm_count = warm;
        self
    }

    pub fn with_max_per_host(mut self, max: usize) -> Self {
        self.max_per_host = max;
        self
    }
}

/// Pool status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
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
