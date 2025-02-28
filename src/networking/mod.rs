pub mod discovery;
pub mod interface;
pub mod communication;
pub mod file_transfer;

// Re-export key components for easier access
pub use discovery::{NodeDiscovery, NodeInfo};
pub use interface::NetworkInterface;
pub use interface::InterfaceType;
pub use communication::NodeClient;
pub use communication::start_grpc_server;
pub use file_transfer::{FileTransferManager, FileTransferConfig, TransferStatus}; 