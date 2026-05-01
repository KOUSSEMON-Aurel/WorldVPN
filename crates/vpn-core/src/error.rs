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

    #[error("STUN protocol error: {0}")]
    StunError(String),

    #[error("P2P error: {0}")]
    P2pError(String),

    #[error("Abuse detection triggered: {0}")]
    AbuseDetected(String),
}

impl From<libp2p::swarm::ConnectionDenied> for VpnError {
    fn from(err: libp2p::swarm::ConnectionDenied) -> Self {
        VpnError::P2pError(err.to_string())
    }
}

impl From<stun::Error> for VpnError {
    fn from(err: stun::Error) -> Self {
        VpnError::StunError(err.to_string())
    }
}
