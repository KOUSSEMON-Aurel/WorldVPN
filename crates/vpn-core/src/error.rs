use thiserror::Error;

pub type Result<T> = std::result::Result<T, VpnError>;

/// Unified error type for the entire VPN core library
#[derive(Debug, Error)]
pub enum VpnError {
    #[error("Internal hardware/OS error: {0}")]
    Internal(String),

    #[error("Invalid configuration provided: {0}")]
    InvalidConfig(String),

    #[error("Failed to establish tunnel connection: {0}")]
    ConnectionFailed(String),

    #[error("Network error during data transfer: {0}")]
    NetworkError(#[from] std::io::Error),

    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),

    #[error("Authentication failed or unauthorized")]
    AuthFailed,

    #[error("NAT traversal failed: {0}")]
    NatTraversalFailed(String),

    #[error("Protocol specific error: {0}")]
    ProtocolError(String),

    #[error("Abuse detection triggered: {0}")]
    AbuseDetected(String),
}
