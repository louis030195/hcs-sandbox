//! HCS Compute System wrapper

use std::ffi::c_void;
use windows::{
    core::{HSTRING, PCWSTR},
    Win32::System::HostComputeSystem::*,
};
use crate::{Error, Result};
use super::operation::Operation;

/// Wrapper around HCS_SYSTEM handle
pub struct ComputeSystem {
    handle: HCS_SYSTEM,
    id: String,
}

impl ComputeSystem {
    /// Create a new compute system with the given JSON configuration
    pub fn create(id: &str, config_json: &str) -> Result<Self> {
        unsafe {
            let id_hstring = HSTRING::from(id);
            let config_hstring = HSTRING::from(config_json);
            let operation = Operation::new();

            let handle = HcsCreateComputeSystem(
                PCWSTR(id_hstring.as_ptr()),
                PCWSTR(config_hstring.as_ptr()),
                operation.handle(),
                None,
            )?;

            // Wait for create operation to complete
            match operation.wait_and_get_result() {
                Ok(result) => {
                    if !result.is_empty() {
                        eprintln!("Create result: {}", result);
                    }
                }
                Err(e) => {
                    eprintln!("Create operation failed: {}", e);
                    return Err(e);
                }
            }

            Ok(Self {
                handle,
                id: id.to_string(),
            })
        }
    }

    /// Open an existing compute system by ID
    pub fn open(id: &str) -> Result<Self> {
        unsafe {
            let id_hstring = HSTRING::from(id);
            
            let handle = HcsOpenComputeSystem(
                PCWSTR(id_hstring.as_ptr()),
                0x10000000, // GENERIC_ALL
            )?;

            Ok(Self {
                handle,
                id: id.to_string(),
            })
        }
    }

    /// Start the compute system
    pub fn start(&self) -> Result<()> {
        unsafe {
            let operation = Operation::new();
            HcsStartComputeSystem(
                self.handle,
                operation.handle(),
                PCWSTR::null(),
            )?;
            // Wait for the operation to complete and check for errors
            let result = operation.wait_and_get_result();
            match &result {
                Ok(s) if !s.is_empty() => {
                    eprintln!("Start operation result: {}", s);
                }
                Err(e) => {
                    eprintln!("Start operation error: {}", e);
                    // Try to get any result document that might have error details
                    if let Ok(details) = operation.get_result() {
                        if !details.is_empty() {
                            eprintln!("Error details: {}", details);
                        }
                    }
                }
                _ => {}
            }
            result.map(|_| ())
        }
    }

    /// Pause the compute system
    pub fn pause(&self) -> Result<()> {
        unsafe {
            let operation = Operation::new();
            HcsPauseComputeSystem(
                self.handle,
                operation.handle(),
                PCWSTR::null(),
            )?;
            Ok(())
        }
    }

    /// Resume a paused compute system
    pub fn resume(&self) -> Result<()> {
        unsafe {
            let operation = Operation::new();
            HcsResumeComputeSystem(
                self.handle,
                operation.handle(),
                PCWSTR::null(),
            )?;
            Ok(())
        }
    }

    /// Terminate the compute system
    pub fn terminate(&self) -> Result<()> {
        unsafe {
            let operation = Operation::new();
            HcsTerminateComputeSystem(
                self.handle,
                operation.handle(),
                PCWSTR::null(),
            )?;
            Ok(())
        }
    }

    /// Save/checkpoint the compute system
    pub fn save(&self, options: Option<&str>) -> Result<()> {
        unsafe {
            let operation = Operation::new();
            let options_hstring = options.map(HSTRING::from);
            let options_pcwstr = options_hstring
                .as_ref()
                .map(|h| PCWSTR(h.as_ptr()))
                .unwrap_or(PCWSTR::null());

            HcsSaveComputeSystem(
                self.handle,
                operation.handle(),
                options_pcwstr,
            )?;
            Ok(())
        }
    }

    /// Get compute system properties
    pub fn get_properties(&self, query: Option<&str>) -> Result<String> {
        unsafe {
            let operation = Operation::new();
            let query_str = query.unwrap_or("{}");
            let query_hstring = HSTRING::from(query_str);

            HcsGetComputeSystemProperties(
                self.handle,
                operation.handle(),
                PCWSTR(query_hstring.as_ptr()),
            )?;

            operation.get_result()
        }
    }

    /// Modify compute system configuration
    pub fn modify(&self, config: &str) -> Result<()> {
        unsafe {
            let operation = Operation::new();
            let config_hstring = HSTRING::from(config);

            HcsModifyComputeSystem(
                self.handle,
                operation.handle(),
                PCWSTR(config_hstring.as_ptr()),
                windows::Win32::Foundation::HANDLE::default(),
            )?;
            Ok(())
        }
    }

    /// Get the ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the raw handle (for advanced operations)
    pub fn handle(&self) -> HCS_SYSTEM {
        self.handle
    }
}

impl Drop for ComputeSystem {
    fn drop(&mut self) {
        unsafe {
            HcsCloseComputeSystem(self.handle);
        }
    }
}

/// Enumerate all compute systems
pub fn enumerate_compute_systems(query: Option<&str>) -> Result<Vec<ComputeSystemInfo>> {
    unsafe {
        let query_str = query.unwrap_or(r#"{"Owners": null}"#);
        let query_hstring = HSTRING::from(query_str);
        let operation = Operation::new();

        HcsEnumerateComputeSystems(
            PCWSTR(query_hstring.as_ptr()),
            operation.handle(),
        )?;

        let result = operation.get_result()?;
        
        if result.is_empty() || result == "[]" {
            return Ok(Vec::new());
        }

        let systems: Vec<ComputeSystemInfo> = serde_json::from_str(&result)
            .map_err(|e| Error::Hcs(format!("Failed to parse enumerate result: {}", e)))?;

        Ok(systems)
    }
}

/// Get HCS service properties
pub fn get_service_properties() -> Result<ServiceProperties> {
    unsafe {
        let result = HcsGetServiceProperties(PCWSTR::null())?;
        let result_str = result.to_string().unwrap_or_default();
        windows::Win32::System::Com::CoTaskMemFree(Some(result.as_ptr() as *const c_void));
        
        let props: ServiceProperties = serde_json::from_str(&result_str)
            .map_err(|e| Error::Hcs(format!("Failed to parse service properties: {}", e)))?;

        Ok(props)
    }
}

/// Information about a compute system from enumeration
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ComputeSystemInfo {
    pub id: String,
    pub owner: Option<String>,
    pub state: Option<String>,
    pub system_type: Option<String>,
    #[serde(rename = "RuntimeId")]
    pub runtime_id: Option<String>,
}

/// HCS Service properties
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceProperties {
    pub supported_schema_versions: Option<Vec<SchemaVersion>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
}
