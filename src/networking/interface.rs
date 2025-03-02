use anyhow::{Result, anyhow};
use if_addrs::{IfAddr, Interface, get_if_addrs};
use local_ip_address::{list_afinet_netifas, local_ip};
use log::{debug, info, warn, error};
use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceType {
    Thunderbolt,
    Ethernet,
    Wifi,
    Loopback,
    Other,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: IpAddr,
    pub interface_type: InterfaceType,
    pub priority: u8, // Higher number = higher priority
}

impl NetworkInterface {
    pub fn new(name: String, ip: IpAddr, interface_type: InterfaceType) -> Self {
        // Assign priority based on interface type
        let priority = match interface_type {
            InterfaceType::Thunderbolt => 100, // Highest priority
            InterfaceType::Ethernet => 80,
            InterfaceType::Wifi => 60,
            InterfaceType::Loopback => 10,
            InterfaceType::Other => 1,
        };

        Self {
            name,
            ip,
            interface_type,
            priority,
        }
    }

    /// Determines if this is likely a Thunderbolt interface
    fn is_thunderbolt(name: &str) -> bool {
        // Common patterns for Thunderbolt interfaces
        name.contains("thunderbolt") || 
        name.contains("tb") || 
        name.contains("bridge") ||
        name.starts_with("en") && (name.contains("5") || name.contains("6"))
    }

    /// Determines if this is likely an Ethernet interface
    fn is_ethernet(name: &str) -> bool {
        name.contains("eth") || 
        name.contains("en") ||
        name.starts_with("en")
    }

    /// Determines if this is likely a WiFi interface
    fn is_wifi(name: &str) -> bool {
        name.contains("wlan") || 
        name.contains("wifi") || 
        name.contains("wi-fi") ||
        name.starts_with("wl")
    }

    /// Determines if this is a loopback interface
    fn is_loopback(name: &str, ip: &IpAddr) -> bool {
        name.contains("lo") || 
        match ip {
            IpAddr::V4(addr) => addr.is_loopback(),
            IpAddr::V6(addr) => addr.is_loopback(),
        }
    }

    /// Detect the interface type based on interface name and properties
    fn detect_interface_type(name: &str, ip: &IpAddr) -> InterfaceType {
        if Self::is_loopback(name, ip) {
            InterfaceType::Loopback
        } else if Self::is_thunderbolt(name) {
            InterfaceType::Thunderbolt
        } else if Self::is_ethernet(name) {
            InterfaceType::Ethernet
        } else if Self::is_wifi(name) {
            InterfaceType::Wifi
        } else {
            InterfaceType::Other
        }
    }
}

/// Discover all network interfaces on the system
pub fn discover_interfaces() -> Result<Vec<NetworkInterface>> {
    let mut interfaces = Vec::new();
    
    // Get all network interfaces
    match get_if_addrs() {
        Ok(if_addrs) => {
            for interface in if_addrs {
                let ip = match interface.addr {
                    IfAddr::V4(addr) => IpAddr::V4(addr.ip),
                    IfAddr::V6(addr) => IpAddr::V6(addr.ip),
                };
                
                // Skip interfaces without a valid IP
                if ip.is_unspecified() || ip.is_multicast() {
                    continue;
                }
                
                let interface_type = NetworkInterface::detect_interface_type(&interface.name, &ip);
                
                debug!("Discovered interface: {} ({}), IP: {}, Type: {:?}", 
                      interface.name, interface.name, ip, interface_type);
                
                interfaces.push(NetworkInterface::new(
                    interface.name.clone(),
                    ip,
                    interface_type,
                ));
            }
        },
        Err(err) => {
            error!("Failed to get network interfaces: {}", err);
            return Err(anyhow!("Failed to get network interfaces: {}", err));
        }
    }
    
    // Sort interfaces by priority (highest first)
    interfaces.sort_by(|a, b| b.priority.cmp(&a.priority));
    
    for (idx, interface) in interfaces.iter().enumerate() {
        info!("Interface #{}: {} ({:?}) - {}", 
              idx + 1, interface.name, interface.interface_type, interface.ip);
    }
    
    if interfaces.is_empty() {
        warn!("No usable network interfaces found!");
    }
    
    Ok(interfaces)
}

/// Get the best interface for node-to-node communication
pub fn get_best_interface() -> Result<NetworkInterface> {
    let interfaces = discover_interfaces()?;
    
    // Get the highest priority non-loopback interface
    for interface in &interfaces {
        if interface.interface_type != InterfaceType::Loopback {
            return Ok(interface.clone());
        }
    }
    
    Err(anyhow!("No suitable network interface found"))
}

/// Get the local machine's main IP address
pub fn get_local_ip() -> Result<IpAddr> {
    match local_ip() {
        Ok(ip) => Ok(ip),
        Err(err) => {
            error!("Failed to determine local IP: {}", err);
            Err(anyhow!("Failed to determine local IP: {}", err))
        }
    }
} 