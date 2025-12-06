//! Error types for HCS Sandbox

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HCS operation failed: {0}")]
    Hcs(String),

    #[error("Windows API error: {0}")]
    Windows(#[from] windows::core::Error),

    #[error("Sandbox not found: {0}")]
    SandboxNotFound(String),

    #[error("Sandbox already exists: {0}")]
    SandboxAlreadyExists(String),

    #[error("Invalid state: sandbox is {current}, expected {expected}")]
    InvalidState { current: String, expected: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Layer error: {0}")]
    Layer(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Insufficient privileges - run as Administrator or add to Hyper-V Administrators")]
    InsufficientPrivileges,

    #[error("Hyper-V not available - enable Hyper-V and Containers features")]
    HyperVNotAvailable,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Check if this is a privilege error
    pub fn is_privilege_error(&self) -> bool {
        matches!(self, Error::InsufficientPrivileges)
    }

    /// Create from Windows HRESULT if it's a privilege error
    pub fn from_hresult(hr: i32, context: &str) -> Self {
        // 0x8037011B = HCS_E_ACCESS_DENIED
        if hr == 0x8037011Bu32 as i32 {
            Error::InsufficientPrivileges
        } else {
            Error::Hcs(format!("{}: HRESULT 0x{:08X}", context, hr))
        }
    }
}
