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
