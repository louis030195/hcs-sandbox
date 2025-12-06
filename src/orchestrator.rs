//! VM orchestration and lifecycle management

use crate::db::Database;
use crate::hyperv::HyperV;
use crate::models::*;
use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Configuration for the orchestrator
pub struct OrchestratorConfig {
    /// Base path for VM storage
    pub vm_storage_path: PathBuf,
    /// Database path
    pub db_path: PathBuf,
    /// Default switch name for VMs
    pub switch_name: String,
    /// Timeout for VM ready check
    pub ready_timeout: Duration,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            vm_storage_path: PathBuf::from(r"C:\HyperVKube\VMs"),
            db_path: PathBuf::from(r"C:\HyperVKube\state.db"),
            switch_name: "Default Switch".to_string(),
            ready_timeout: Duration::from_secs(120),
        }
    }
}

/// Main orchestrator for VM management
pub struct Orchestrator {
    db: Database,
    config: OrchestratorConfig,
}

impl Orchestrator {
    /// Create orchestrator with default config
    pub fn new() -> Result<Self> {
        Self::with_config(OrchestratorConfig::default())
    }

    /// Create orchestrator with custom config
    pub fn with_config(config: OrchestratorConfig) -> Result<Self> {
        // Ensure directories exist
        std::fs::create_dir_all(&config.vm_storage_path)?;
        std::fs::create_dir_all(config.db_path.parent().unwrap_or(Path::new(".")))?;

        let db = Database::open(&config.db_path)?;

        Ok(Self { db, config })
    }

    /// Get database reference
    pub fn db(&self) -> &Database {
        &self.db
    }

    // ===== Template Operations =====

    /// Register a template (golden image)
    pub fn register_template(&self, template: Template) -> Result<String> {
        // Verify VHDX exists
        if !template.vhdx_path.exists() {
            return Err(Error::Other(format!(
                "Template VHDX not found: {:?}",
                template.vhdx_path
            )));
        }

        let id = template.id.clone();
        self.db.insert_template(&template)?;
        tracing::info!(template = %template.name, id = %id, "Template registered");
        Ok(id)
    }

    /// List all templates
    pub fn list_templates(&self) -> Result<Vec<Template>> {
        self.db.list_templates()
    }

    /// Get template by name
    pub fn get_template(&self, name: &str) -> Result<Option<Template>> {
        self.db.get_template_by_name(name)
    }

    // ===== Pool Operations =====

    /// Create a VM pool
    pub fn create_pool(&self, pool: VMPool) -> Result<String> {
        // Verify template exists
        if self.db.get_template(&pool.template_id)?.is_none() {
            return Err(Error::TemplateNotFound(pool.template_id.clone()));
        }

        let id = pool.id.clone();
        self.db.insert_pool(&pool)?;
        tracing::info!(pool = %pool.name, id = %id, "Pool created");
        Ok(id)
    }

    /// List all pools
    pub fn list_pools(&self) -> Result<Vec<VMPool>> {
        self.db.list_pools()
    }

    /// Get pool status
    pub fn get_pool_status(&self, pool_id: &str) -> Result<PoolStatus> {
        let pool = self.db.get_pool(pool_id)?
            .ok_or_else(|| Error::PoolNotFound(pool_id.to_string()))?;

        let vms = self.db.list_vms_by_pool(pool_id)?;

        Ok(PoolStatus {
            id: pool.id,
            name: pool.name,
            template_id: pool.template_id,
            desired_count: pool.desired_count,
            total_vms: vms.len(),
            running_vms: vms.iter().filter(|v| v.state == VMState::Running).count(),
            saved_vms: vms.iter().filter(|v| v.state == VMState::Saved).count(),
            off_vms: vms.iter().filter(|v| v.state == VMState::Off).count(),
            error_vms: vms.iter().filter(|v| v.state == VMState::Error).count(),
        })
    }

    /// Provision VMs for a pool (create and prepare them)
    pub fn provision_pool(&self, pool_id: &str, count: usize) -> Result<Vec<String>> {
        let pool = self.db.get_pool(pool_id)?
            .ok_or_else(|| Error::PoolNotFound(pool_id.to_string()))?;

        let template = self.db.get_template(&pool.template_id)?
            .ok_or_else(|| Error::TemplateNotFound(pool.template_id.clone()))?;

        let existing = self.db.list_vms_by_pool(pool_id)?;
        let start_index = existing.len();

        let mut created_ids = Vec::new();

        for i in 0..count {
            let vm_name = format!("{}-{}", pool.name, start_index + i);
            let vm_dir = self.config.vm_storage_path.join(&vm_name);
            std::fs::create_dir_all(&vm_dir)?;

            let vhdx_path = vm_dir.join("disk.vhdx");

            tracing::info!(vm = %vm_name, "Creating differencing disk");
            HyperV::create_differencing_disk(
                template.vhdx_path.to_str().unwrap(),
                vhdx_path.to_str().unwrap(),
            )?;

            tracing::info!(vm = %vm_name, "Creating VM");
            HyperV::create_vm(
                &vm_name,
                vhdx_path.to_str().unwrap(),
                template.memory_mb,
                template.cpu_count,
            )?;

            // Configure network
            HyperV::set_network_adapter(&vm_name, &self.config.switch_name)?;

            // Enable enhanced session
            let _ = HyperV::enable_enhanced_session(&vm_name);

            // Add GPU if template has it
            if template.gpu_enabled {
                let _ = HyperV::add_gpu(&vm_name);
            }

            // Create VM record
            let mut vm = VM::new(vm_name.clone(), vhdx_path, template.memory_mb, template.cpu_count);
            vm.template_id = Some(template.id.clone());
            vm.pool_id = Some(pool.id.clone());
            vm.gpu_enabled = template.gpu_enabled;

            self.db.insert_vm(&vm)?;
            created_ids.push(vm.id.clone());

            tracing::info!(vm = %vm_name, "VM created (not yet booted)");
        }

        Ok(created_ids)
    }

    /// Boot a VM, create checkpoint, and save state (makes it ready for fast resume)
    pub fn prepare_vm(&self, vm_id: &str) -> Result<()> {
        let vm = self.db.get_vm(vm_id)?
            .ok_or_else(|| Error::VMNotFound(vm_id.to_string()))?;

        tracing::info!(vm = %vm.name, "Starting VM for first boot");
        HyperV::start_vm(&vm.name)?;
        self.db.update_vm_state(vm_id, VMState::Running)?;

        tracing::info!(vm = %vm.name, "Waiting for VM to be ready");
        let ip = HyperV::wait_for_ready(&vm.name, self.config.ready_timeout)?;
        self.db.update_vm_ip(vm_id, Some(&ip))?;

        // Wait a bit more for Windows to settle
        std::thread::sleep(Duration::from_secs(10));

        tracing::info!(vm = %vm.name, "Creating clean checkpoint");
        HyperV::create_checkpoint(&vm.name, "clean")?;

        tracing::info!(vm = %vm.name, "Saving VM state");
        HyperV::save_vm(&vm.name)?;
        self.db.update_vm_state(vm_id, VMState::Saved)?;

        tracing::info!(vm = %vm.name, "VM ready for fast resume");
        Ok(())
    }

    // ===== VM Operations =====

    /// List all VMs
    pub fn list_vms(&self) -> Result<Vec<VM>> {
        self.db.list_vms()
    }

    /// Get VM by name
    pub fn get_vm(&self, name: &str) -> Result<Option<VM>> {
        self.db.get_vm_by_name(name)
    }

    /// Resume a saved VM (fast, 2-5 seconds)
    pub fn resume_vm(&self, vm_id: &str) -> Result<String> {
        let vm = self.db.get_vm(vm_id)?
            .ok_or_else(|| Error::VMNotFound(vm_id.to_string()))?;

        if vm.state != VMState::Saved {
            return Err(Error::InvalidState {
                current: vm.state.to_string(),
                expected: "Saved".to_string(),
            });
        }

        let start = std::time::Instant::now();
        tracing::info!(vm = %vm.name, "Resuming VM");

        HyperV::start_vm(&vm.name)?;
        self.db.update_vm_state(vm_id, VMState::Running)?;
        self.db.update_vm_resumed(vm_id)?;

        // Wait for ready
        let ip = HyperV::wait_for_ready(&vm.name, Duration::from_secs(30))?;
        self.db.update_vm_ip(vm_id, Some(&ip))?;

        let elapsed = start.elapsed();
        tracing::info!(vm = %vm.name, elapsed_ms = elapsed.as_millis(), ip = %ip, "VM resumed");

        Ok(ip)
    }

    /// Save VM state (for fast resume later)
    pub fn save_vm(&self, vm_id: &str) -> Result<()> {
        let vm = self.db.get_vm(vm_id)?
            .ok_or_else(|| Error::VMNotFound(vm_id.to_string()))?;

        if vm.state != VMState::Running {
            return Err(Error::InvalidState {
                current: vm.state.to_string(),
                expected: "Running".to_string(),
            });
        }

        tracing::info!(vm = %vm.name, "Saving VM state");
        HyperV::save_vm(&vm.name)?;
        self.db.update_vm_state(vm_id, VMState::Saved)?;
        self.db.update_vm_agent(vm_id, None)?;

        Ok(())
    }

    /// Reset VM to clean checkpoint
    pub fn reset_vm(&self, vm_id: &str) -> Result<()> {
        let vm = self.db.get_vm(vm_id)?
            .ok_or_else(|| Error::VMNotFound(vm_id.to_string()))?;

        tracing::info!(vm = %vm.name, "Resetting VM to clean checkpoint");

        // Stop if running
        if vm.state == VMState::Running {
            HyperV::turn_off_vm(&vm.name)?;
        }

        HyperV::restore_checkpoint(&vm.name, "clean")?;
        self.db.update_vm_state(vm_id, VMState::Off)?;
        self.db.update_vm_agent(vm_id, None)?;
        self.db.update_vm_ip(vm_id, None)?;

        Ok(())
    }

    /// Stop VM
    pub fn stop_vm(&self, vm_id: &str, force: bool) -> Result<()> {
        let vm = self.db.get_vm(vm_id)?
            .ok_or_else(|| Error::VMNotFound(vm_id.to_string()))?;

        tracing::info!(vm = %vm.name, force = force, "Stopping VM");

        if force {
            HyperV::turn_off_vm(&vm.name)?;
        } else {
            HyperV::stop_vm(&vm.name, true)?;
        }

        self.db.update_vm_state(vm_id, VMState::Off)?;
        Ok(())
    }

    /// Delete VM completely
    pub fn delete_vm(&self, vm_id: &str) -> Result<()> {
        let vm = self.db.get_vm(vm_id)?
            .ok_or_else(|| Error::VMNotFound(vm_id.to_string()))?;

        tracing::info!(vm = %vm.name, "Deleting VM");

        // Stop if running
        if vm.state == VMState::Running || vm.state == VMState::Saved {
            let _ = HyperV::turn_off_vm(&vm.name);
        }

        // Remove from Hyper-V
        let _ = HyperV::remove_vm(&vm.name);

        // Delete VHDX
        if vm.vhdx_path.exists() {
            std::fs::remove_file(&vm.vhdx_path)?;
        }

        // Delete VM directory
        if let Some(parent) = vm.vhdx_path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }

        // Remove from DB
        self.db.delete_vm(vm_id)?;

        Ok(())
    }

    /// Open VM console
    pub fn open_console(&self, vm_id: &str) -> Result<()> {
        let vm = self.db.get_vm(vm_id)?
            .ok_or_else(|| Error::VMNotFound(vm_id.to_string()))?;

        HyperV::open_console(&vm.name)
    }

    // ===== Agent/Scheduling Operations =====

    /// Acquire a VM from pool (resumes saved VM)
    pub fn acquire_vm(&self, pool_id: &str) -> Result<VM> {
        let vm = self.db.find_available_vm_in_pool(pool_id)?
            .ok_or(Error::NoVMAvailable)?;

        self.resume_vm(&vm.id)?;

        // Refresh VM info
        self.db.get_vm(&vm.id)?
            .ok_or_else(|| Error::VMNotFound(vm.id.clone()))
    }

    /// Release VM back to pool
    pub fn release_vm(&self, vm_id: &str, reset: bool) -> Result<()> {
        if reset {
            self.reset_vm(vm_id)?;
            // Re-prepare after reset
            self.prepare_vm(vm_id)?;
        } else {
            self.save_vm(vm_id)?;
        }

        self.db.update_vm_agent(vm_id, None)?;
        Ok(())
    }

    /// Sync DB state with actual Hyper-V state
    pub fn reconcile(&self) -> Result<()> {
        let hyperv_vms = HyperV::list_vms()?;
        let db_vms = self.db.list_vms()?;

        for db_vm in db_vms {
            if let Some(hv_vm) = hyperv_vms.iter().find(|v| v.name == db_vm.name) {
                let actual_state = VMState::from_hyperv_state(hv_vm.state);
                if db_vm.state != actual_state {
                    tracing::info!(
                        vm = %db_vm.name,
                        db_state = %db_vm.state,
                        actual_state = %actual_state,
                        "Reconciling VM state"
                    );
                    self.db.update_vm_state(&db_vm.id, actual_state)?;
                }
            } else {
                tracing::warn!(vm = %db_vm.name, "VM not found in Hyper-V");
                self.db.update_vm_state(&db_vm.id, VMState::Error)?;
            }
        }

        Ok(())
    }
}
