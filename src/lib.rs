//! HCS Sandbox Orchestrator
//!
//! A Rust library for creating and managing Windows sandboxes using the
//! Host Compute Service (HCS) API - the same low-level API that powers
//! Windows Sandbox and Docker on Windows.
//!
//! # Key Features
//!
//! - **No Docker required** - Uses HCS directly with dynamic base OS layers
//! - **Fast boot** - Sandboxes start in 2-5 seconds vs minutes for VMs
//! - **UI Automation ready** - HyperV isolation with GPU passthrough
//! - **High density** - Run 10-20 sandboxes per host
//!
//! # Example
//!
//! ```no_run
//! use hcs_sandbox::{Orchestrator, SandboxConfig};
//!
//! let mut orchestrator = Orchestrator::new()?;
//!
//! let config = SandboxConfig::builder()
//!     .name("my-sandbox")
//!     .memory_mb(4096)
//!     .cpu_count(2)
//!     .gpu_enabled(true)
//!     .build();
//!
//! let sandbox_id = orchestrator.create(config)?;
//! orchestrator.start(&sandbox_id)?;
//!
//! // Run UI automation...
//!
//! orchestrator.destroy(&sandbox_id)?;
//! # Ok::<(), hcs_sandbox::Error>(())
//! ```

pub mod config;
pub mod error;
pub mod hcs;
pub mod network;
pub mod orchestrator;
pub mod sandbox;

pub use config::SandboxConfig;
pub use error::{Error, Result};
pub use orchestrator::Orchestrator;
pub use sandbox::{Sandbox, SandboxState};
