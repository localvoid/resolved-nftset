use thiserror::Error;

#[derive(Error, Debug)]
pub enum NftError {
    #[error("Socket error: {0}")]
    SocketError(#[from] std::io::Error),

    #[error("Netlink error: {0}")]
    NetlinkError(i32),

    #[error("Set not found")]
    SetNotFound,

    #[error("Protocol error")]
    ProtocolError,
}
