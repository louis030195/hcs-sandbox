//! SQLite state storage

use crate::models::*;
use crate::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Database for state storage
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open or create database
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Create in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                vhdx_path TEXT NOT NULL,
                memory_mb INTEGER NOT NULL,
                cpu_count INTEGER NOT NULL,
                gpu_enabled INTEGER NOT NULL,
                installed_software TEXT,
                description TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS pools (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                template_id TEXT NOT NULL,
                desired_count INTEGER NOT NULL,
                warm_count INTEGER NOT NULL,
                max_per_host INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (template_id) REFERENCES templates(id)
            );

            CREATE TABLE IF NOT EXISTS vms (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                template_id TEXT,
                pool_id TEXT,
                state TEXT NOT NULL,
                vhdx_path TEXT NOT NULL,
                ip_address TEXT,
                memory_mb INTEGER NOT NULL,
                cpu_count INTEGER NOT NULL,
                gpu_enabled INTEGER NOT NULL,
                current_agent_id TEXT,
                created_at TEXT NOT NULL,
                last_resumed_at TEXT,
                error_message TEXT,
                FOREIGN KEY (template_id) REFERENCES templates(id),
                FOREIGN KEY (pool_id) REFERENCES pools(id)
            );

            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                pool_id TEXT,
                vm_id TEXT,
                status TEXT NOT NULL,
                task TEXT NOT NULL,
                created_at TEXT NOT NULL,
                scheduled_at TEXT,
                started_at TEXT,
                completed_at TEXT,
                result TEXT,
                error_message TEXT,
                FOREIGN KEY (pool_id) REFERENCES pools(id),
                FOREIGN KEY (vm_id) REFERENCES vms(id)
            );

            CREATE INDEX IF NOT EXISTS idx_vms_pool ON vms(pool_id);
            CREATE INDEX IF NOT EXISTS idx_vms_state ON vms(state);
            CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
            "#,
        )?;
        Ok(())
    }

    // ===== Templates =====

    pub fn insert_template(&self, t: &Template) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO templates (id, name, vhdx_path, memory_mb, cpu_count, gpu_enabled, installed_software, description, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                t.id,
                t.name,
                t.vhdx_path.to_string_lossy(),
                t.memory_mb,
                t.cpu_count,
                t.gpu_enabled as i32,
                serde_json::to_string(&t.installed_software)?,
                t.description,
                t.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_template(&self, id: &str) -> Result<Option<Template>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, vhdx_path, memory_mb, cpu_count, gpu_enabled, installed_software, description, created_at FROM templates WHERE id = ?1",
            params![id],
            |row| {
                let software_json: String = row.get(6)?;
                Ok(Template {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    vhdx_path: row.get::<_, String>(2)?.into(),
                    memory_mb: row.get(3)?,
                    cpu_count: row.get(4)?,
                    gpu_enabled: row.get::<_, i32>(5)? != 0,
                    installed_software: serde_json::from_str(&software_json).unwrap_or_default(),
                    description: row.get(7)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?).unwrap().with_timezone(&chrono::Utc),
                })
            },
        ).optional().map_err(Into::into)
    }

    pub fn get_template_by_name(&self, name: &str) -> Result<Option<Template>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, vhdx_path, memory_mb, cpu_count, gpu_enabled, installed_software, description, created_at FROM templates WHERE name = ?1",
            params![name],
            |row| {
                let software_json: String = row.get(6)?;
                Ok(Template {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    vhdx_path: row.get::<_, String>(2)?.into(),
                    memory_mb: row.get(3)?,
                    cpu_count: row.get(4)?,
                    gpu_enabled: row.get::<_, i32>(5)? != 0,
                    installed_software: serde_json::from_str(&software_json).unwrap_or_default(),
                    description: row.get(7)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?).unwrap().with_timezone(&chrono::Utc),
                })
            },
        ).optional().map_err(Into::into)
    }

    pub fn list_templates(&self) -> Result<Vec<Template>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, vhdx_path, memory_mb, cpu_count, gpu_enabled, installed_software, description, created_at FROM templates ORDER BY name"
        )?;
        let templates = stmt.query_map([], |row| {
            let software_json: String = row.get(6)?;
            Ok(Template {
                id: row.get(0)?,
                name: row.get(1)?,
                vhdx_path: row.get::<_, String>(2)?.into(),
                memory_mb: row.get(3)?,
                cpu_count: row.get(4)?,
                gpu_enabled: row.get::<_, i32>(5)? != 0,
                installed_software: serde_json::from_str(&software_json).unwrap_or_default(),
                description: row.get(7)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?).unwrap().with_timezone(&chrono::Utc),
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(templates)
    }

    pub fn delete_template(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM templates WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    // ===== Pools =====

    pub fn insert_pool(&self, p: &VMPool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO pools (id, name, template_id, desired_count, warm_count, max_per_host, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                p.id,
                p.name,
                p.template_id,
                p.desired_count,
                p.warm_count,
                p.max_per_host,
                p.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_pool(&self, id: &str) -> Result<Option<VMPool>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, template_id, desired_count, warm_count, max_per_host, created_at FROM pools WHERE id = ?1",
            params![id],
            |row| Ok(VMPool {
                id: row.get(0)?,
                name: row.get(1)?,
                template_id: row.get(2)?,
                desired_count: row.get::<_, i64>(3)? as usize,
                warm_count: row.get::<_, i64>(4)? as usize,
                max_per_host: row.get::<_, i64>(5)? as usize,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap().with_timezone(&chrono::Utc),
            }),
        ).optional().map_err(Into::into)
    }

    pub fn get_pool_by_name(&self, name: &str) -> Result<Option<VMPool>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, template_id, desired_count, warm_count, max_per_host, created_at FROM pools WHERE name = ?1",
            params![name],
            |row| Ok(VMPool {
                id: row.get(0)?,
                name: row.get(1)?,
                template_id: row.get(2)?,
                desired_count: row.get::<_, i64>(3)? as usize,
                warm_count: row.get::<_, i64>(4)? as usize,
                max_per_host: row.get::<_, i64>(5)? as usize,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap().with_timezone(&chrono::Utc),
            }),
        ).optional().map_err(Into::into)
    }

    pub fn list_pools(&self) -> Result<Vec<VMPool>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, template_id, desired_count, warm_count, max_per_host, created_at FROM pools ORDER BY name"
        )?;
        let pools = stmt.query_map([], |row| {
            Ok(VMPool {
                id: row.get(0)?,
                name: row.get(1)?,
                template_id: row.get(2)?,
                desired_count: row.get::<_, i64>(3)? as usize,
                warm_count: row.get::<_, i64>(4)? as usize,
                max_per_host: row.get::<_, i64>(5)? as usize,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap().with_timezone(&chrono::Utc),
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(pools)
    }

    pub fn delete_pool(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM pools WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    // ===== VMs =====

    pub fn insert_vm(&self, vm: &VM) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO vms (id, name, template_id, pool_id, state, vhdx_path, ip_address, memory_mb, cpu_count, gpu_enabled, current_agent_id, created_at, last_resumed_at, error_message)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
            params![
                vm.id,
                vm.name,
                vm.template_id,
                vm.pool_id,
                format!("{:?}", vm.state),
                vm.vhdx_path.to_string_lossy(),
                vm.ip_address,
                vm.memory_mb,
                vm.cpu_count,
                vm.gpu_enabled as i32,
                vm.current_agent_id,
                vm.created_at.to_rfc3339(),
                vm.last_resumed_at.map(|t| t.to_rfc3339()),
                vm.error_message,
            ],
        )?;
        Ok(())
    }

    pub fn get_vm(&self, id: &str) -> Result<Option<VM>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, template_id, pool_id, state, vhdx_path, ip_address, memory_mb, cpu_count, gpu_enabled, current_agent_id, created_at, last_resumed_at, error_message FROM vms WHERE id = ?1",
            params![id],
            Self::row_to_vm,
        ).optional().map_err(Into::into)
    }

    pub fn get_vm_by_name(&self, name: &str) -> Result<Option<VM>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, template_id, pool_id, state, vhdx_path, ip_address, memory_mb, cpu_count, gpu_enabled, current_agent_id, created_at, last_resumed_at, error_message FROM vms WHERE name = ?1",
            params![name],
            Self::row_to_vm,
        ).optional().map_err(Into::into)
    }

    pub fn list_vms(&self) -> Result<Vec<VM>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, template_id, pool_id, state, vhdx_path, ip_address, memory_mb, cpu_count, gpu_enabled, current_agent_id, created_at, last_resumed_at, error_message FROM vms ORDER BY name"
        )?;
        let vms = stmt.query_map([], Self::row_to_vm)?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(vms)
    }

    pub fn list_vms_by_pool(&self, pool_id: &str) -> Result<Vec<VM>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, template_id, pool_id, state, vhdx_path, ip_address, memory_mb, cpu_count, gpu_enabled, current_agent_id, created_at, last_resumed_at, error_message FROM vms WHERE pool_id = ?1 ORDER BY name"
        )?;
        let vms = stmt.query_map(params![pool_id], Self::row_to_vm)?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(vms)
    }

    pub fn find_available_vm_in_pool(&self, pool_id: &str) -> Result<Option<VM>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, template_id, pool_id, state, vhdx_path, ip_address, memory_mb, cpu_count, gpu_enabled, current_agent_id, created_at, last_resumed_at, error_message FROM vms WHERE pool_id = ?1 AND state = 'Saved' AND current_agent_id IS NULL LIMIT 1",
            params![pool_id],
            Self::row_to_vm,
        ).optional().map_err(Into::into)
    }

    pub fn update_vm_state(&self, id: &str, state: VMState) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE vms SET state = ?1 WHERE id = ?2",
            params![format!("{:?}", state), id],
        )?;
        Ok(())
    }

    pub fn update_vm_ip(&self, id: &str, ip: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE vms SET ip_address = ?1 WHERE id = ?2",
            params![ip, id],
        )?;
        Ok(())
    }

    pub fn update_vm_agent(&self, vm_id: &str, agent_id: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE vms SET current_agent_id = ?1 WHERE id = ?2",
            params![agent_id, vm_id],
        )?;
        Ok(())
    }

    pub fn update_vm_resumed(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE vms SET last_resumed_at = ?1 WHERE id = ?2",
            params![chrono::Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn delete_vm(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM vms WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    fn row_to_vm(row: &rusqlite::Row) -> rusqlite::Result<VM> {
        let state_str: String = row.get(4)?;
        let state = match state_str.as_str() {
            "Off" => VMState::Off,
            "Running" => VMState::Running,
            "Saved" => VMState::Saved,
            "Paused" => VMState::Paused,
            _ => VMState::Error,
        };
        let last_resumed: Option<String> = row.get(12)?;
        Ok(VM {
            id: row.get(0)?,
            name: row.get(1)?,
            template_id: row.get(2)?,
            pool_id: row.get(3)?,
            state,
            vhdx_path: row.get::<_, String>(5)?.into(),
            ip_address: row.get(6)?,
            memory_mb: row.get(7)?,
            cpu_count: row.get(8)?,
            gpu_enabled: row.get::<_, i32>(9)? != 0,
            current_agent_id: row.get(10)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?).unwrap().with_timezone(&chrono::Utc),
            last_resumed_at: last_resumed.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            error_message: row.get(13)?,
        })
    }

    // ===== Agents =====

    pub fn insert_agent(&self, a: &Agent) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO agents (id, name, pool_id, vm_id, status, task, created_at, scheduled_at, started_at, completed_at, result, error_message)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            params![
                a.id,
                a.name,
                a.pool_id,
                a.vm_id,
                format!("{:?}", a.status),
                serde_json::to_string(&a.task)?,
                a.created_at.to_rfc3339(),
                a.scheduled_at.map(|t| t.to_rfc3339()),
                a.started_at.map(|t| t.to_rfc3339()),
                a.completed_at.map(|t| t.to_rfc3339()),
                a.result.as_ref().map(|r| serde_json::to_string(r).unwrap()),
                a.error_message,
            ],
        )?;
        Ok(())
    }

    pub fn get_agent(&self, id: &str) -> Result<Option<Agent>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, name, pool_id, vm_id, status, task, created_at, scheduled_at, started_at, completed_at, result, error_message FROM agents WHERE id = ?1",
            params![id],
            Self::row_to_agent,
        ).optional().map_err(Into::into)
    }

    pub fn list_agents(&self) -> Result<Vec<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, pool_id, vm_id, status, task, created_at, scheduled_at, started_at, completed_at, result, error_message FROM agents ORDER BY created_at DESC"
        )?;
        let agents = stmt.query_map([], Self::row_to_agent)?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(agents)
    }

    pub fn list_pending_agents(&self) -> Result<Vec<Agent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, pool_id, vm_id, status, task, created_at, scheduled_at, started_at, completed_at, result, error_message FROM agents WHERE status = 'Pending' ORDER BY created_at"
        )?;
        let agents = stmt.query_map([], Self::row_to_agent)?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(agents)
    }

    pub fn update_agent_status(&self, id: &str, status: AgentStatus) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE agents SET status = ?1 WHERE id = ?2",
            params![format!("{:?}", status), id],
        )?;
        Ok(())
    }

    pub fn update_agent_vm(&self, agent_id: &str, vm_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE agents SET vm_id = ?1, scheduled_at = ?2 WHERE id = ?3",
            params![vm_id, chrono::Utc::now().to_rfc3339(), agent_id],
        )?;
        Ok(())
    }

    fn row_to_agent(row: &rusqlite::Row) -> rusqlite::Result<Agent> {
        let status_str: String = row.get(4)?;
        let status = match status_str.as_str() {
            "Pending" => AgentStatus::Pending,
            "Scheduled" => AgentStatus::Scheduled,
            "Running" => AgentStatus::Running,
            "Completed" => AgentStatus::Completed,
            "Failed" => AgentStatus::Failed,
            "Cancelled" => AgentStatus::Cancelled,
            _ => AgentStatus::Failed,
        };
        let task_json: String = row.get(5)?;
        let result_json: Option<String> = row.get(10)?;
        let scheduled: Option<String> = row.get(7)?;
        let started: Option<String> = row.get(8)?;
        let completed: Option<String> = row.get(9)?;

        Ok(Agent {
            id: row.get(0)?,
            name: row.get(1)?,
            pool_id: row.get(2)?,
            vm_id: row.get(3)?,
            status,
            task: serde_json::from_str(&task_json).unwrap(),
            created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap().with_timezone(&chrono::Utc),
            scheduled_at: scheduled.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            started_at: started.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            completed_at: completed.map(|s| chrono::DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&chrono::Utc)),
            result: result_json.map(|s| serde_json::from_str(&s).unwrap()),
            error_message: row.get(11)?,
        })
    }
}
