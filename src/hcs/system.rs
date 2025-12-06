//! HCS System operations wrapper
//! Uses operation-based async API pattern

use std::ffi::c_void;
use thiserror::Error;
use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    Win32::System::HostComputeSystem::*,
};

#[derive(Error, Debug)]
pub enum HcsError {
    #[error("HCS operation failed: {0}")]
    OperationFailed(String),
    #[error("Windows error: {0}")]
    Windows(#[from] windows::core::Error),
}

pub type Result<T> = std::result::Result<T, HcsError>;

/// Wrapper around HCS_SYSTEM handle
pub struct ComputeSystem {
    handle: HCS_SYSTEM,
    id: String,
}

impl ComputeSystem {
    /// Create a new compute system with the given configuration
    pub fn create(id: &str, config_json: &str) -> Result<Self> {
        unsafe {
            let id_hstring = HSTRING::from(id);
            let config_hstring = HSTRING::from(config_json);
            
            // Create an operation for sync usage (null callback = sync)
            let operation = HcsCreateOperation(None, None);
            
            let handle = HcsCreateComputeSystem(
                PCWSTR(id_hstring.as_ptr()),
                PCWSTR(config_hstring.as_ptr()),
                operation,
                None, // no security descriptor
            )?;
            
            // Get operation result (for error details)
            let mut result_doc: PWSTR = PWSTR::null();
            let _ = HcsGetOperationResult(operation, Some(&mut result_doc));
            
            if !result_doc.is_null() {
                let result_str = result_doc.to_string().unwrap_or_default();
                free_pwstr(&mut result_doc);
                if !result_str.is_empty() {
                    println!("  Create result: {}", result_str);
                }
            }
            
            HcsCloseOperation(operation);

            Ok(Self {
                handle,
                id: id.to_string(),
            })
        }
    }

    /// Start the compute system
    pub fn start(&self) -> Result<()> {
        unsafe {
            let operation = HcsCreateOperation(None, None);
            
            HcsStartComputeSystem(
                self.handle,
                operation,
                PCWSTR::null(), // no options
            )?;
            
            HcsCloseOperation(operation);
            Ok(())
        }
    }

    /// Pause the compute system
    pub fn pause(&self) -> Result<()> {
        unsafe {
            let operation = HcsCreateOperation(None, None);
            
            HcsPauseComputeSystem(
                self.handle,
                operation,
                PCWSTR::null(),
            )?;
            
            HcsCloseOperation(operation);
            Ok(())
        }
    }

    /// Resume a paused compute system
    pub fn resume(&self) -> Result<()> {
        unsafe {
            let operation = HcsCreateOperation(None, None);
            
            HcsResumeComputeSystem(
                self.handle,
                operation,
                PCWSTR::null(),
            )?;
            
            HcsCloseOperation(operation);
            Ok(())
        }
    }

    /// Terminate the compute system forcefully
    pub fn terminate(&self) -> Result<()> {
        unsafe {
            let operation = HcsCreateOperation(None, None);
            
            HcsTerminateComputeSystem(
                self.handle,
                operation,
                PCWSTR::null(),
            )?;
            
            HcsCloseOperation(operation);
            Ok(())
        }
    }

    /// Get properties of the compute system
    pub fn get_properties(&self, query: &str) -> Result<String> {
        unsafe {
            let query_hstring = HSTRING::from(query);
            let operation = HcsCreateOperation(None, None);

            HcsGetComputeSystemProperties(
                self.handle,
                operation,
                PCWSTR(query_hstring.as_ptr()),
            )?;

            let mut result_doc: PWSTR = PWSTR::null();
            HcsGetOperationResult(operation, Some(&mut result_doc))?;

            let result = if !result_doc.is_null() {
                let s = result_doc.to_string().unwrap_or_default();
                free_pwstr(&mut result_doc);
                s
            } else {
                String::new()
            };

            HcsCloseOperation(operation);
            Ok(result)
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl Drop for ComputeSystem {
    fn drop(&mut self) {
        unsafe {
            HcsCloseComputeSystem(self.handle);
        }
    }
}

/// Enumerate all compute systems (containers/VMs) on the host
pub fn enumerate_compute_systems(query: Option<&str>) -> Result<String> {
    unsafe {
        let query_str = query.unwrap_or(r#"{"Owners": null}"#);
        let query_hstring = HSTRING::from(query_str);
        let operation = HcsCreateOperation(None, None);

        HcsEnumerateComputeSystems(
            PCWSTR(query_hstring.as_ptr()),
            operation,
        )?;

        let mut result_doc: PWSTR = PWSTR::null();
        HcsGetOperationResult(operation, Some(&mut result_doc))?;

        let result_str = if !result_doc.is_null() {
            let s = result_doc.to_string().unwrap_or_default();
            free_pwstr(&mut result_doc);
            s
        } else {
            String::new()
        };

        HcsCloseOperation(operation);
        Ok(result_str)
    }
}

/// Open an existing compute system by ID
pub fn open_compute_system(id: &str) -> Result<ComputeSystem> {
    unsafe {
        let id_hstring = HSTRING::from(id);
        
        // GENERIC_ALL access
        let handle = HcsOpenComputeSystem(
            PCWSTR(id_hstring.as_ptr()),
            0x10000000, // GENERIC_ALL
        )?;

        Ok(ComputeSystem {
            handle,
            id: id.to_string(),
        })
    }
}

/// Get HCS service properties
pub fn get_service_properties() -> Result<String> {
    unsafe {
        let result = HcsGetServiceProperties(PCWSTR::null())?;
        let result_str = result.to_string().unwrap_or_default();
        // Note: HcsGetServiceProperties returns allocated memory we need to free
        windows::Win32::System::Com::CoTaskMemFree(Some(result.as_ptr() as *const c_void));
        Ok(result_str)
    }
}

fn free_pwstr(p: &mut PWSTR) {
    unsafe {
        if !p.is_null() {
            windows::Win32::System::Com::CoTaskMemFree(Some(p.as_ptr() as *const c_void));
            *p = PWSTR::null();
        }
    }
}
