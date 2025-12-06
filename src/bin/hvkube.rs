//! hvkube CLI - Lightweight Hyper-V orchestrator for UI automation

use clap::{Parser, Subcommand};
use hyperv_kube::models::*;
use hyperv_kube::{Orchestrator, OrchestratorConfig, Result, Server};
use std::net::SocketAddr;
use std::path::PathBuf;
use tabled::{Table, Tabled};

#[derive(Parser)]
#[command(name = "hvkube")]
#[command(about = "Lightweight Hyper-V orchestrator for UI automation agents")]
#[command(version)]
struct Cli {
    /// Path to data directory
    #[arg(long, global = true, default_value = r"C:\HyperVKube")]
    data_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Template management
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
    /// VM pool management
    Pool {
        #[command(subcommand)]
        action: PoolAction,
    },
    /// Individual VM operations
    Vm {
        #[command(subcommand)]
        action: VmAction,
    },
    /// Sync state with Hyper-V
    Reconcile,
    /// Start HTTP API server
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

#[derive(Subcommand)]
enum TemplateAction {
    /// Register a template (golden image)
    Register {
        /// Template name
        #[arg(short, long)]
        name: String,
        /// Path to VHDX file
        #[arg(short, long)]
        vhdx: PathBuf,
        /// Memory in MB
        #[arg(short, long, default_value = "4096")]
        memory: u64,
        /// CPU count
        #[arg(short, long, default_value = "2")]
        cpus: u32,
        /// Enable GPU
        #[arg(long)]
        gpu: bool,
    },
    /// List templates
    List,
    /// Delete a template
    Delete {
        /// Template name
        name: String,
    },
}

#[derive(Subcommand)]
enum PoolAction {
    /// Create a VM pool
    Create {
        /// Pool name
        #[arg(short, long)]
        name: String,
        /// Template name
        #[arg(short, long)]
        template: String,
        /// Number of VMs
        #[arg(short, long, default_value = "3")]
        count: usize,
    },
    /// List pools
    List,
    /// Show pool status
    Status {
        /// Pool name
        name: String,
    },
    /// Provision VMs for a pool
    Provision {
        /// Pool name
        name: String,
        /// Number of VMs to create
        #[arg(short, long, default_value = "1")]
        count: usize,
    },
    /// Prepare all VMs in pool (boot, checkpoint, save)
    Prepare {
        /// Pool name
        name: String,
    },
    /// Delete a pool
    Delete {
        /// Pool name
        name: String,
        /// Also delete VMs
        #[arg(long)]
        delete_vms: bool,
    },
}

#[derive(Subcommand)]
enum VmAction {
    /// List VMs
    List {
        /// Filter by pool
        #[arg(short, long)]
        pool: Option<String>,
    },
    /// Get VM info
    Info {
        /// VM name
        name: String,
    },
    /// Resume a saved VM (fast!)
    Resume {
        /// VM name
        name: String,
    },
    /// Save VM state
    Save {
        /// VM name
        name: String,
    },
    /// Reset VM to clean checkpoint
    Reset {
        /// VM name
        name: String,
    },
    /// Stop VM
    Stop {
        /// VM name
        name: String,
        /// Force stop
        #[arg(short, long)]
        force: bool,
    },
    /// Delete VM
    Delete {
        /// VM name
        name: String,
    },
    /// Open VM console
    Console {
        /// VM name
        name: String,
    },
    /// Prepare VM (boot, checkpoint, save)
    Prepare {
        /// VM name
        name: String,
    },
}

// Table display structs
#[derive(Tabled)]
struct TemplateRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Memory")]
    memory: String,
    #[tabled(rename = "CPUs")]
    cpus: u32,
    #[tabled(rename = "GPU")]
    gpu: String,
    #[tabled(rename = "VHDX")]
    vhdx: String,
}

#[derive(Tabled)]
struct PoolRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Template")]
    template: String,
    #[tabled(rename = "Desired")]
    desired: usize,
    #[tabled(rename = "Warm")]
    warm: usize,
}

#[derive(Tabled)]
struct VMRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "State")]
    state: String,
    #[tabled(rename = "Pool")]
    pool: String,
    #[tabled(rename = "IP")]
    ip: String,
    #[tabled(rename = "Memory")]
    memory: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("hyperv_kube=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    let config = OrchestratorConfig {
        vm_storage_path: cli.data_dir.join("VMs"),
        db_path: cli.data_dir.join("state.db"),
        ..Default::default()
    };

    let orch = Orchestrator::with_config(config)?;

    match cli.command {
        Commands::Template { action } => handle_template(&orch, action)?,
        Commands::Pool { action } => handle_pool(&orch, action)?,
        Commands::Vm { action } => handle_vm(&orch, action)?,
        Commands::Reconcile => {
            println!("Reconciling state with Hyper-V...");
            orch.reconcile()?;
            println!("Done.");
        }
        Commands::Serve { host, port } => {
            let addr: SocketAddr = format!("{}:{}", host, port).parse()
                .expect("Invalid address");

            println!("Starting API server on http://{}", addr);
            println!();
            println!("Endpoints:");
            println!("  GET  /health                    Health check");
            println!("  GET  /api/v1/templates          List templates");
            println!("  POST /api/v1/templates          Create template");
            println!("  GET  /api/v1/pools              List pools");
            println!("  POST /api/v1/pools              Create pool");
            println!("  GET  /api/v1/pools/:name        Pool status");
            println!("  POST /api/v1/pools/:name/provision  Provision VMs");
            println!("  GET  /api/v1/vms                List VMs");
            println!("  POST /api/v1/vms/:name/resume   Resume VM (fast!)");
            println!("  POST /api/v1/vms/:name/save     Save VM state");
            println!("  POST /api/v1/acquire            Acquire VM from pool");
            println!("  POST /api/v1/vms/:name/release  Release VM to pool");
            println!();

            let server = Server::new(orch, addr);
            server.run().await.map_err(|e| hyperv_kube::Error::Other(e.to_string()))?;
        }
    }

    Ok(())
}

fn handle_template(orch: &Orchestrator, action: TemplateAction) -> Result<()> {
    match action {
        TemplateAction::Register {
            name,
            vhdx,
            memory,
            cpus,
            gpu,
        } => {
            let template = Template::new(&name, &vhdx)
                .with_memory(memory)
                .with_cpus(cpus)
                .with_gpu(gpu);

            let id = orch.register_template(template)?;
            println!("Template registered: {} ({})", name, id);
        }
        TemplateAction::List => {
            let templates = orch.list_templates()?;
            if templates.is_empty() {
                println!("No templates registered.");
                return Ok(());
            }

            let rows: Vec<TemplateRow> = templates
                .iter()
                .map(|t| TemplateRow {
                    name: t.name.clone(),
                    memory: format!("{}MB", t.memory_mb),
                    cpus: t.cpu_count,
                    gpu: if t.gpu_enabled { "Yes" } else { "No" }.to_string(),
                    vhdx: t.vhdx_path.to_string_lossy().to_string(),
                })
                .collect();

            println!("{}", Table::new(rows));
        }
        TemplateAction::Delete { name } => {
            if let Some(t) = orch.get_template(&name)? {
                orch.db().delete_template(&t.id)?;
                println!("Template deleted: {}", name);
            } else {
                println!("Template not found: {}", name);
            }
        }
    }
    Ok(())
}

fn handle_pool(orch: &Orchestrator, action: PoolAction) -> Result<()> {
    match action {
        PoolAction::Create {
            name,
            template,
            count,
        } => {
            let tmpl = orch
                .get_template(&template)?
                .ok_or_else(|| hyperv_kube::Error::TemplateNotFound(template.clone()))?;

            let pool = VMPool::new(&name, &tmpl.id).with_count(count);
            let id = orch.create_pool(pool)?;
            println!("Pool created: {} ({})", name, id);
        }
        PoolAction::List => {
            let pools = orch.list_pools()?;
            if pools.is_empty() {
                println!("No pools created.");
                return Ok(());
            }

            let templates = orch.list_templates()?;
            let rows: Vec<PoolRow> = pools
                .iter()
                .map(|p| {
                    let tmpl_name = templates
                        .iter()
                        .find(|t| t.id == p.template_id)
                        .map(|t| t.name.clone())
                        .unwrap_or_else(|| "?".to_string());
                    PoolRow {
                        name: p.name.clone(),
                        template: tmpl_name,
                        desired: p.desired_count,
                        warm: p.warm_count,
                    }
                })
                .collect();

            println!("{}", Table::new(rows));
        }
        PoolAction::Status { name } => {
            let pool = orch
                .db()
                .get_pool_by_name(&name)?
                .ok_or_else(|| hyperv_kube::Error::PoolNotFound(name.clone()))?;

            let status = orch.get_pool_status(&pool.id)?;
            println!("Pool: {}", status.name);
            println!("  Desired: {}", status.desired_count);
            println!("  Total:   {}", status.total_vms);
            println!("  Running: {}", status.running_vms);
            println!("  Saved:   {}", status.saved_vms);
            println!("  Off:     {}", status.off_vms);
            println!("  Error:   {}", status.error_vms);
        }
        PoolAction::Provision { name, count } => {
            let pool = orch
                .db()
                .get_pool_by_name(&name)?
                .ok_or_else(|| hyperv_kube::Error::PoolNotFound(name.clone()))?;

            println!("Provisioning {} VMs for pool {}...", count, name);
            let ids = orch.provision_pool(&pool.id, count)?;
            println!("Created {} VMs:", ids.len());
            for id in ids {
                if let Some(vm) = orch.db().get_vm(&id)? {
                    println!("  - {}", vm.name);
                }
            }
        }
        PoolAction::Prepare { name } => {
            let pool = orch
                .db()
                .get_pool_by_name(&name)?
                .ok_or_else(|| hyperv_kube::Error::PoolNotFound(name.clone()))?;

            let vms = orch.db().list_vms_by_pool(&pool.id)?;
            let off_vms: Vec<_> = vms.iter().filter(|v| v.state == VMState::Off).collect();

            if off_vms.is_empty() {
                println!("No VMs to prepare in pool {}", name);
                return Ok(());
            }

            println!("Preparing {} VMs in pool {}...", off_vms.len(), name);
            for vm in off_vms {
                println!("Preparing {}...", vm.name);
                orch.prepare_vm(&vm.id)?;
            }
            println!("Done.");
        }
        PoolAction::Delete { name, delete_vms } => {
            let pool = orch
                .db()
                .get_pool_by_name(&name)?
                .ok_or_else(|| hyperv_kube::Error::PoolNotFound(name.clone()))?;

            if delete_vms {
                let vms = orch.db().list_vms_by_pool(&pool.id)?;
                for vm in vms {
                    println!("Deleting VM {}...", vm.name);
                    orch.delete_vm(&vm.id)?;
                }
            }

            orch.db().delete_pool(&pool.id)?;
            println!("Pool deleted: {}", name);
        }
    }
    Ok(())
}

fn handle_vm(orch: &Orchestrator, action: VmAction) -> Result<()> {
    match action {
        VmAction::List { pool } => {
            let vms = if let Some(pool_name) = pool {
                let p = orch
                    .db()
                    .get_pool_by_name(&pool_name)?
                    .ok_or_else(|| hyperv_kube::Error::PoolNotFound(pool_name.clone()))?;
                orch.db().list_vms_by_pool(&p.id)?
            } else {
                orch.list_vms()?
            };

            if vms.is_empty() {
                println!("No VMs found.");
                return Ok(());
            }

            let pools = orch.list_pools()?;
            let rows: Vec<VMRow> = vms
                .iter()
                .map(|v| {
                    let pool_name = v
                        .pool_id
                        .as_ref()
                        .and_then(|pid| pools.iter().find(|p| &p.id == pid))
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "-".to_string());
                    VMRow {
                        name: v.name.clone(),
                        state: v.state.to_string(),
                        pool: pool_name,
                        ip: v.ip_address.clone().unwrap_or_else(|| "-".to_string()),
                        memory: format!("{}MB", v.memory_mb),
                    }
                })
                .collect();

            println!("{}", Table::new(rows));
        }
        VmAction::Info { name } => {
            let vm = orch
                .get_vm(&name)?
                .ok_or_else(|| hyperv_kube::Error::VMNotFound(name.clone()))?;

            println!("VM: {}", vm.name);
            println!("  ID:       {}", vm.id);
            println!("  State:    {}", vm.state);
            println!("  IP:       {}", vm.ip_address.unwrap_or("-".into()));
            println!("  Memory:   {}MB", vm.memory_mb);
            println!("  CPUs:     {}", vm.cpu_count);
            println!("  GPU:      {}", if vm.gpu_enabled { "Yes" } else { "No" });
            println!("  VHDX:     {}", vm.vhdx_path.display());
            println!("  Created:  {}", vm.created_at);
            if let Some(t) = vm.last_resumed_at {
                println!("  Resumed:  {}", t);
            }
        }
        VmAction::Resume { name } => {
            let vm = orch
                .get_vm(&name)?
                .ok_or_else(|| hyperv_kube::Error::VMNotFound(name.clone()))?;

            println!("Resuming {}...", name);
            let start = std::time::Instant::now();
            let ip = orch.resume_vm(&vm.id)?;
            let elapsed = start.elapsed();
            println!("VM ready in {:.2}s at {}", elapsed.as_secs_f64(), ip);
        }
        VmAction::Save { name } => {
            let vm = orch
                .get_vm(&name)?
                .ok_or_else(|| hyperv_kube::Error::VMNotFound(name.clone()))?;

            println!("Saving {}...", name);
            orch.save_vm(&vm.id)?;
            println!("Done.");
        }
        VmAction::Reset { name } => {
            let vm = orch
                .get_vm(&name)?
                .ok_or_else(|| hyperv_kube::Error::VMNotFound(name.clone()))?;

            println!("Resetting {} to clean checkpoint...", name);
            orch.reset_vm(&vm.id)?;
            println!("Done.");
        }
        VmAction::Stop { name, force } => {
            let vm = orch
                .get_vm(&name)?
                .ok_or_else(|| hyperv_kube::Error::VMNotFound(name.clone()))?;

            println!("Stopping {}...", name);
            orch.stop_vm(&vm.id, force)?;
            println!("Done.");
        }
        VmAction::Delete { name } => {
            let vm = orch
                .get_vm(&name)?
                .ok_or_else(|| hyperv_kube::Error::VMNotFound(name.clone()))?;

            println!("Deleting {}...", name);
            orch.delete_vm(&vm.id)?;
            println!("Done.");
        }
        VmAction::Console { name } => {
            let vm = orch
                .get_vm(&name)?
                .ok_or_else(|| hyperv_kube::Error::VMNotFound(name.clone()))?;

            println!("Opening console for {}...", name);
            orch.open_console(&vm.id)?;
        }
        VmAction::Prepare { name } => {
            let vm = orch
                .get_vm(&name)?
                .ok_or_else(|| hyperv_kube::Error::VMNotFound(name.clone()))?;

            println!("Preparing {} (boot, checkpoint, save)...", name);
            orch.prepare_vm(&vm.id)?;
            println!("Done. VM is ready for fast resume.");
        }
    }
    Ok(())
}
