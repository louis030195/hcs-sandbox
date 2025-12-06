//! HyperV-Kube: Lightweight VM orchestrator for UI automation agents
//!
//! A simple Kubernetes-like orchestrator for Windows VMs using Hyper-V.
//! Uses Save/Resume for blazing fast (~2-5 second) VM startup.
//!
//! # Key Features
//!
//! - **Fast boot** - VMs resume from saved state in 2-5 seconds
//! - **Simple** - Uses Hyper-V directly via PowerShell (no HCS complexity)
//! - **Pool management** - Pre-warm VMs for instant availability
//! - **Template-based** - Clone VMs from golden images using differencing disks
//!
//! # Example
//!
//! ```no_run
//! use hyperv_kube::{Orchestrator, models::Template};
//!
//! // Create orchestrator
//! let orch = Orchestrator::new()?;
//!
//! // Register a template
//! let template = Template::new("win11-chrome", r"C:\Templates\win11.vhdx")
//!     .with_memory(4096)
//!     .with_cpus(2);
//! orch.register_template(template)?;
//!
//! // Create a pool
//! let pool = hyperv_kube::models::VMPool::new("browser-pool", "tmpl-xxx");
//! orch.create_pool(pool)?;
//!
//! // Provision VMs (creates and prepares them)
//! orch.provision_pool("pool-xxx", 3)?;
//!
//! // Acquire a VM (resumes in 2-5 seconds!)
//! let vm = orch.acquire_vm("pool-xxx")?;
//! println!("VM ready at: {}", vm.ip_address.unwrap());
//!
//! // Release back to pool
//! orch.release_vm(&vm.id, false)?;
//! # Ok::<(), hyperv_kube::Error>(())
//! ```

pub mod db;
pub mod error;
pub mod hyperv;
pub mod models;
pub mod orchestrator;

pub use error::{Error, Result};
pub use orchestrator::{Orchestrator, OrchestratorConfig};
