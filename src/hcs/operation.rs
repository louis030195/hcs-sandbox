//! HCS Operation wrapper for async operations

use std::ffi::c_void;
use windows::{
    core::PWSTR,
    Win32::System::HostComputeSystem::*,
};
use crate::Result;

/// Wrapper around HCS_OPERATION for synchronous operations
pub struct Operation {
    handle: HCS_OPERATION,
}

impl Operation {
    /// Create a new synchronous operation
    pub fn new() -> Self {
        unsafe {
            let handle = HcsCreateOperation(None, None);
            Self { handle }
        }
    }

    /// Get the raw handle
    pub fn handle(&self) -> HCS_OPERATION {
        self.handle
    }

    /// Get the operation result as a string
    pub fn get_result(&self) -> Result<String> {
        unsafe {
            let mut result_doc: PWSTR = PWSTR::null();
            HcsGetOperationResult(self.handle, Some(&mut result_doc))?;

            let result = if !result_doc.is_null() {
                let s = result_doc.to_string().unwrap_or_default();
                windows::Win32::System::Com::CoTaskMemFree(Some(result_doc.as_ptr() as *const c_void));
                s
            } else {
                String::new()
            };

            Ok(result)
        }
    }

    /// Wait for the operation to complete and return result
    pub fn wait_and_get_result(&self) -> Result<String> {
        unsafe {
            let mut result_doc: PWSTR = PWSTR::null();
            
            // HcsWaitForOperationResult waits for completion
            HcsWaitForOperationResult(self.handle, u32::MAX, Some(&mut result_doc))?;

            let result = if !result_doc.is_null() {
                let s = result_doc.to_string().unwrap_or_default();
                windows::Win32::System::Com::CoTaskMemFree(Some(result_doc.as_ptr() as *const c_void));
                s
            } else {
                String::new()
            };

            Ok(result)
        }
    }
}

impl Default for Operation {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        unsafe {
            HcsCloseOperation(self.handle);
        }
    }
}
