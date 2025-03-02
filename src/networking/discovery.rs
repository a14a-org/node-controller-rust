use anyhow::{Result, anyhow};
use log::{debug, info, warn, error};
use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use uuid::Uuid;
use std::str::FromStr;

use super::interface::{self, NetworkInterface};

const SERVICE_TYPE: &str = "_node-controller._tcp.local.";
const DISCOVERY_PORT: u16 = 54321; // Default port for node discovery
const ADVERTISE_TTL: u32 = 60; // TTL for service advertisements in seconds
const REFRESH_INTERVAL: Duration = Duration::from_secs(55); // Re-advertise before TTL expires

/// Node information shared during discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub interface_type: String,
    pub capabilities: Vec<String>,
    pub version: String,
}

impl NodeInfo {
    pub fn new(name: String, interface: &NetworkInterface, port: u16) -> Self {
        // Generate a UUID for this node
        let uuid = Uuid::new_v4();
        
        Self {
            id: uuid.to_string(),
            name,
            ip: interface.ip.to_string(),
            port,
            interface_type: format!("{:?}", interface.interface_type),
            capabilities: vec!["discovery".to_string()], // Add more capabilities as they're implemented
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
    
    /// Attempt to parse NodeInfo from TXT records
    fn from_service_info(info: &ServiceInfo) -> Option<Self> {
        let ip_addr = info.get_addresses().iter().next().copied()?;
        
        // Extract TXT records
        let mut txt_records = HashMap::new();
        for prop in info.get_properties().iter() {
            if let Some(val) = prop.val() {
                if let Ok(value) = String::from_utf8(val.to_vec()) {
                    txt_records.insert(prop.key().to_string(), value);
                }
            }
        }
        
        Some(Self {
            id: txt_records.get("id")?.clone(),
            name: txt_records.get("name")?.clone(),
            ip: ip_addr.to_string(),
            port: info.get_port(),
            interface_type: txt_records.get("interface_type")?.clone(),
            capabilities: txt_records.get("capabilities")?.split(',').map(String::from).collect(),
            version: txt_records.get("version")?.clone(),
        })
    }
}

/// Main node discovery service
pub struct NodeDiscovery {
    mdns: ServiceDaemon,
    local_node: NodeInfo,
    discovered_nodes: Arc<Mutex<HashMap<String, (NodeInfo, Instant)>>>,
    service_name: String,
}

impl NodeDiscovery {
    /// Create a new node discovery service
    pub fn new(node_name: &str, port: Option<u16>) -> Result<Self> {
        // Get the best network interface for node communication
        let interface = interface::get_best_interface()?;
        
        // Create local node info
        let local_node = NodeInfo::new(
            node_name.to_string(),
            &interface,
            port.unwrap_or(DISCOVERY_PORT),
        );
        
        info!("Initializing node discovery for node {} on {:?} interface ({})...",
             local_node.name, interface.interface_type, interface.ip);
        
        // Create unique service name
        let service_name = format!("{}_{}", node_name, Uuid::new_v4().to_string());
        
        // Initialize mDNS service daemon
        let mdns = ServiceDaemon::new()?;
        
        Ok(Self {
            mdns,
            local_node,
            discovered_nodes: Arc::new(Mutex::new(HashMap::new())),
            service_name,
        })
    }
    
    /// Start the discovery service
    pub async fn start(&self) -> Result<()> {
        // Start advertising our service
        self.advertise_service()?;
        
        // Browse for other services
        self.browse_services().await?;
        
        Ok(())
    }
    
    /// Advertise this node as an available service
    fn advertise_service(&self) -> Result<()> {
        let ip_addr = self.local_node.ip.clone();
        let port = self.local_node.port;
        let hostname = format!("{}.local.", self.local_node.ip);
        
        // Create properties as a HashMap
        let mut properties = HashMap::new();
        properties.insert("id".to_string(), self.local_node.id.clone());
        properties.insert("name".to_string(), self.local_node.name.clone());
        properties.insert("interface_type".to_string(), self.local_node.interface_type.clone());
        properties.insert("capabilities".to_string(), self.local_node.capabilities.join(","));
        properties.insert("version".to_string(), self.local_node.version.clone());
        
        // Create the service info
        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &self.service_name,
            &hostname,
            &ip_addr,
            port,
            properties,
        )?;
        
        // Register the service
        self.mdns.register(service_info)?;
        info!("Node '{}' ready and advertising on {} port {}", 
             self.local_node.name, self.local_node.ip, port);
        
        // Setup periodic re-advertising
        let mdns = self.mdns.clone();
        let service_name = self.service_name.clone();
        let local_node = self.local_node.clone();
        
        tokio::spawn(async move {
            loop {
                sleep(REFRESH_INTERVAL).await;
                
                let ip_addr = local_node.ip.clone();
                let hostname = format!("{}.local.", local_node.ip);
                
                // Create properties as a HashMap for refresh
                let mut properties = HashMap::new();
                properties.insert("id".to_string(), local_node.id.clone());
                properties.insert("name".to_string(), local_node.name.clone());
                properties.insert("interface_type".to_string(), local_node.interface_type.clone());
                properties.insert("capabilities".to_string(), local_node.capabilities.join(","));
                properties.insert("version".to_string(), local_node.version.clone());
                
                match ServiceInfo::new(
                    SERVICE_TYPE,
                    &service_name,
                    &hostname,
                    &ip_addr,
                    local_node.port,
                    properties,
                ) {
                    Ok(service_info) => {
                        if let Err(e) = mdns.register(service_info) {
                            error!("Failed to refresh service advertisement: {}", e);
                        } else {
                            // Reduce logging - only log on debug level
                            debug!("Refreshed service advertisement");
                        }
                    },
                    Err(e) => error!("Failed to create service info for refresh: {}", e),
                }
            }
        });
        
        Ok(())
    }
    
    /// Browse for other node controller services
    async fn browse_services(&self) -> Result<()> {
        // Browse for services
        let receiver = self.mdns.browse(SERVICE_TYPE)?;
        
        // Store the discovered nodes
        let discovered_nodes = self.discovered_nodes.clone();
        let local_id = self.local_node.id.clone();
        
        // Process events in background
        tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        if let Some(node) = NodeInfo::from_service_info(&info) {
                            // Don't add ourselves to the discovered nodes
                            if node.id != local_id {
                                info!("✅ Discovered node: {} ({})", node.name, node.id);
                                let mut nodes = discovered_nodes.lock().unwrap();
                                nodes.insert(node.id.clone(), (node, Instant::now()));
                            }
                        }
                    },
                    ServiceEvent::ServiceRemoved(_, name) => {
                        let mut nodes = discovered_nodes.lock().unwrap();
                        // Find and remove by matching service name
                        let keys_to_remove: Vec<String> = nodes.iter()
                            .filter(|(_, (node, _))| {
                                // Extract the node name from the full service name
                                if let Some(node_part) = name.split('.').next() {
                                    node_part.contains(&node.name)
                                } else {
                                    false
                                }
                            })
                            .map(|(k, _)| k.clone())
                            .collect();
                        
                        for key in keys_to_remove {
                            if let Some((node, _)) = nodes.remove(&key) {
                                info!("👋 Node removed: {} ({})", node.name, node.id);
                            }
                        }
                    },
                    _ => {}
                }
            }
        });
        
        Ok(())
    }
    
    /// Get a copy of all currently discovered nodes
    pub fn get_discovered_nodes(&self) -> Vec<NodeInfo> {
        let now = Instant::now();
        let mut result = Vec::new();
        
        // Clean up expired nodes (older than 2*TTL)
        let expiration = Duration::from_secs(ADVERTISE_TTL as u64 * 2);
        
        let mut nodes = self.discovered_nodes.lock().unwrap();
        nodes.retain(|_, (node, timestamp)| {
            let expired = now.duration_since(*timestamp) > expiration;
            if expired {
                info!("Removing expired node: {} ({})", node.name, node.id);
                false
            } else {
                true
            }
        });
        
        // Add all active nodes to the result
        for (_, (node, _)) in nodes.iter() {
            result.push(node.clone());
        }
        
        result
    }
    
    /// Get information about the local node
    pub fn get_local_node(&self) -> NodeInfo {
        self.local_node.clone()
    }
    
    /// Stop the discovery service
    pub fn shutdown(&self) -> Result<()> {
        // Unregister our service
        if let Err(e) = self.mdns.unregister(&self.service_name) {
            warn!("Failed to unregister service: {}", e);
        }
        
        Ok(())
    }
} 