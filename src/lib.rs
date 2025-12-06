//! HCS Kube - Minimal Kubernetes-like orchestrator for HCS sandboxes
//!
//! A Rust library for creating and managing Windows sandboxes using the
//! Host Compute Service (HCS) API - the same low-level API that powers
//! Windows Sandbox and Docker on Windows.
//!
//! # Key Features
//!
//! - **Pool management** - Pre-warmed sandboxes for instant acquisition
//! - **Fast boot** - Sandboxes resume in 2-5 seconds
//! - **HvSocket communication** - Direct host↔guest messaging without networking
//! - **Acquire/Execute/Release** - Kubernetes-like task scheduling
//!
//! # Example
//!
//! ```no_run
//! use hcs_kube::{Orchestrator, SandboxConfig, Pool, PoolConfig, Scheduler, Task};
//! use std::sync::Arc;
//!
//! // Create orchestrator
//! let orchestrator = Arc::new(Orchestrator::new()?);
//!
//! // Configure sandbox template
//! let sandbox_config = SandboxConfig::builder()
//!     .name("worker")
//!     .memory_mb(4096)
//!     .cpu_count(2)
//!     .build();
//!
//! // Create a pool with warm sandboxes
//! let pool_config = PoolConfig::new("worker-pool", sandbox_config)
//!     .min_warm(3)
//!     .max_total(10);
//!
//! let pool = Arc::new(Pool::new(pool_config, r"C:\HcsKube\pools"));
//! pool.warm(&orchestrator)?;
//!
//! // Create scheduler
//! let scheduler = Scheduler::new(pool, orchestrator);
//!
//! // Execute a task (acquire -> execute -> release)
//! let task = Task::new("steps:\n  - click: Start");
//! let result = scheduler.execute(task)?;
//!
//! println!("Task {} completed in {:?}", result.task_id, result.duration);
//! # Ok::<(), hcs_kube::Error>(())
//! ```

pub mod config;
pub mod error;
pub mod hcs;
pub mod hvsocket;
pub mod network;
pub mod orchestrator;
pub mod pool;
pub mod sandbox;
pub mod scheduler;

pub use config::SandboxConfig;
pub use error::{Error, Result};
pub use hvsocket::{AgentClient, AgentMessage, AgentResponse, HvSocketAddr};
pub use orchestrator::{Orchestrator, OrchestratorConfig};
pub use pool::{Pool, PoolConfig, PoolStatus, PooledSandbox, PooledSandboxStatus};
pub use sandbox::{Sandbox, SandboxState};
pub use scheduler::{Scheduler, Task, TaskResult, TaskStatus};
