use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{Result, anyhow};
use log::{debug, info, warn, error};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use tonic::transport::{Channel, Server};

// Import generated protobuf code
pub mod node {
    tonic::include_proto!("node");
}

use node::node_service_server::{NodeService, NodeServiceServer};
use node::node_service_client::NodeServiceClient;
use node::{PingRequest, PongResponse, HealthCheckRequest, HealthCheckResponse};

use super::discovery::NodeInfo;

/// Node communication service implementing the gRPC interface
pub struct NodeCommunicationService {
    node_id: String,
    node_name: String,
    health_status: Mutex<node::health_check_response::Status>,
    health_metrics: Mutex<HashMap<String, String>>,
}

impl NodeCommunicationService {
    pub fn new(node_id: String, node_name: String) -> Self {
        Self {
            node_id,
            node_name,
            health_status: Mutex::new(node::health_check_response::Status::Healthy),
            health_metrics: Mutex::new(HashMap::new()),
        }
    }

    /// Update the health status of this node
    pub async fn update_health_status(&self, status: node::health_check_response::Status) {
        let mut current_status = self.health_status.lock().await;
        *current_status = status;
    }

    /// Update health metrics
    pub async fn update_health_metrics(&self, metrics: HashMap<String, String>) {
        let mut current_metrics = self.health_metrics.lock().await;
        *current_metrics = metrics;
    }

    /// Add or update a specific health metric
    pub async fn set_health_metric(&self, key: &str, value: &str) {
        let mut metrics = self.health_metrics.lock().await;
        metrics.insert(key.to_string(), value.to_string());
    }
}

#[tonic::async_trait]
impl NodeService for NodeCommunicationService {
    /// Handle ping requests from other nodes
    async fn ping(&self, request: Request<PingRequest>) -> Result<Response<PongResponse>, Status> {
        let ping_req = request.into_inner();
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        
        info!("📨 Received ping from '{}' with message: {}", ping_req.sender_name, ping_req.message);
        
        // Construct the pong response
        let response = PongResponse {
            responder_id: self.node_id.clone(),
            responder_name: self.node_name.clone(),
            message: format!("Hello, {}! Your message was: {}", ping_req.sender_name, ping_req.message),
            request_timestamp: ping_req.timestamp,
            response_timestamp: now,
        };
        
        Ok(Response::new(response))
    }

    /// Handle health check requests from other nodes
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let health_req = request.into_inner();
        
        debug!("Received health check from {}", health_req.sender_id);
        
        // Get current health status and metrics
        let status = *self.health_status.lock().await;
        let metrics = self.health_metrics.lock().await.clone();
        
        // Construct the health check response
        let response = HealthCheckResponse {
            responder_id: self.node_id.clone(),
            responder_name: self.node_name.clone(),
            status: status as i32,
            metrics,
        };
        
        Ok(Response::new(response))
    }
}

/// Client for communicating with other nodes
pub struct NodeClient {
    clients: Mutex<HashMap<String, NodeServiceClient<Channel>>>,
}

impl NodeClient {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }
    
    /// Get or create a client for a specific node
    async fn get_client(&self, node: &NodeInfo) -> Result<NodeServiceClient<Channel>> {
        let mut clients = self.clients.lock().await;
        
        if let Some(client) = clients.get(&node.id) {
            Ok(client.clone())
        } else {
            let addr = format!("http://{}:{}", node.ip, node.port);
            debug!("Creating new client for node {} at {}", node.name, addr);
            
            match NodeServiceClient::connect(addr.clone()).await {
                Ok(client) => {
                    let client_clone = client.clone();
                    clients.insert(node.id.clone(), client);
                    Ok(client_clone)
                },
                Err(e) => Err(anyhow!("Failed to connect to node at {}: {}", addr, e)),
            }
        }
    }
    
    /// Send a ping to a specific node
    pub async fn ping(&self, node: &NodeInfo, message: &str, local_node: &NodeInfo) -> Result<PongResponse> {
        let mut client = self.get_client(node).await?;
        
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        
        let request = PingRequest {
            sender_id: local_node.id.clone(),
            sender_name: local_node.name.clone(),
            message: message.to_string(),
            timestamp: now,
        };
        
        match client.ping(request).await {
            Ok(response) => {
                let resp = response.into_inner();
                debug!("Received pong from {} ({}): {}",
                      resp.responder_name, resp.responder_id, resp.message);
                Ok(resp)
            },
            Err(e) => Err(anyhow!("Ping failed: {}", e)),
        }
    }
    
    /// Check the health of a specific node
    pub async fn health_check(&self, node: &NodeInfo, local_node: &NodeInfo) -> Result<HealthCheckResponse> {
        let mut client = self.get_client(node).await?;
        
        let request = HealthCheckRequest {
            sender_id: local_node.id.clone(),
        };
        
        match client.health_check(request).await {
            Ok(response) => {
                let resp = response.into_inner();
                debug!("Health check response from {} ({}): status={:?}",
                      resp.responder_name, resp.responder_id, resp.status);
                Ok(resp)
            },
            Err(e) => Err(anyhow!("Health check failed: {}", e)),
        }
    }
}

/// Starts the gRPC server for node communication
pub async fn start_grpc_server(
    node_info: NodeInfo,
    addr: SocketAddr,
) -> Result<()> {
    // Create the service
    let service = NodeCommunicationService::new(
        node_info.id.clone(),
        node_info.name.clone(),
    );
    
    info!("Starting gRPC server for node {} on {}...", node_info.name, addr);
    
    // Create the server
    let server = Server::builder()
        .add_service(NodeServiceServer::new(service))
        .serve(addr);
    
    // Start the server in the background
    tokio::spawn(async move {
        match server.await {
            Ok(_) => info!("gRPC server shutdown gracefully"),
            Err(e) => error!("gRPC server error: {}", e),
        }
    });
    
    info!("gRPC server for node '{}' listening on {}", node_info.name, addr);
    
    Ok(())
} 