//! Error types for hyperv-kube

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("PowerShell error: {0}")]
    PowerShell(String),

    #[error("VM not found: {0}")]
    VMNotFound(String),

    #[error("VM already exists: {0}")]
    VMAlreadyExists(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Pool not found: {0}")]
    PoolNotFound(String),

    #[error("No VM available in pool")]
    NoVMAvailable,

    #[error("Invalid state: VM is {current}, expected {expected}")]
    InvalidState { current: String, expected: String },

    #[error("Timeout waiting for VM")]
    Timeout,

    #[error("VM has no IP address")]
    NoIP,

    #[error("Guest not responding")]
    GuestNotResponding,

    #[error("Insufficient resources: need {required}MB, have {available}MB")]
    InsufficientMemory { required: u64, available: u64 },

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Hyper-V not available - enable Hyper-V feature")]
    HyperVNotAvailable,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let e = Error::VMNotFound("vm-123".to_string());
        assert_eq!(e.to_string(), "VM not found: vm-123");

        let e = Error::TemplateNotFound("tmpl-1".to_string());
        assert_eq!(e.to_string(), "Template not found: tmpl-1");

        let e = Error::NoVMAvailable;
        assert_eq!(e.to_string(), "No VM available in pool");
    }

    #[test]
    fn test_error_invalid_state() {
        let e = Error::InvalidState {
            current: "Off".to_string(),
            expected: "Saved".to_string(),
        };
        assert!(e.to_string().contains("Off"));
        assert!(e.to_string().contains("Saved"));
    }

    #[test]
    fn test_error_insufficient_memory() {
        let e = Error::InsufficientMemory {
            required: 8192,
            available: 4096,
        };
        assert!(e.to_string().contains("8192"));
        assert!(e.to_string().contains("4096"));
    }

    #[test]
    fn test_error_debug() {
        let e = Error::Timeout;
        let debug = format!("{:?}", e);
        assert!(debug.contains("Timeout"));
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<i32> {
            Ok(42)
        }
        
        fn returns_err() -> Result<i32> {
            Err(Error::NoVMAvailable)
        }

        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }
}
