pub mod discovery;
pub mod interface;

// Re-export key components for easier access
pub use discovery::{NodeDiscovery, NodeInfo};
pub use interface::NetworkInterface; 