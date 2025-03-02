use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use log::{debug, error, info, warn};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use std::collections::HashMap;
use serde_json;
use std::io::BufReader;
use sha2::{Sha256, Digest};

// Constants for file transfer
const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks
const DEFAULT_PORT: u16 = 7879;
const BUFFER_POOL_SIZE: usize = 8; // Number of reusable buffers

/// Status of a file transfer, reported via progress callback
#[derive(Debug, Clone)]
pub enum TransferStatus {
    /// Transfer started
    Started {
        file_id: String,
        file_name: String,
        file_size: u64,
    },
    /// Transfer in progress
    Progress {
        file_id: String,
        bytes_transferred: u64,
        total_bytes: u64,
        percent_complete: f32,
    },
    /// Transfer completed successfully
    Completed {
        file_id: String,
        bytes_transferred: u64,
        elapsed_seconds: f32,
        throughput_mbps: f32,
    },
    /// Transfer failed
    Failed {
        file_id: String,
        error: String,
    },
}

/// Direction of file transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    /// Sending a file
    Send,
    /// Receiving a file
    Receive,
}

/// Type of progress callback for file transfers
pub type ProgressCallback = Arc<dyn Fn(TransferStatus) + Send + Sync>;

/// Configuration for file transfers
#[derive(Clone)]
pub struct FileTransferConfig {
    /// Size of chunks to use for file transfer
    pub chunk_size: usize,
    /// Port to use for file transfer server
    pub port: u16,
    /// Directory to store received files
    pub receive_dir: PathBuf,
    /// Optional progress callback
    pub progress_callback: Option<ProgressCallback>,
    /// Number of concurrent transfer streams
    pub concurrent_streams: usize,
}

impl Default for FileTransferConfig {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            port: DEFAULT_PORT,
            receive_dir: std::env::temp_dir().join("node_controller_files"),
            progress_callback: None,
            concurrent_streams: 4, // Default to 4 concurrent streams
        }
    }
}

/// File transfer manager using optimized TCP
pub struct FileTransferManager {
    config: FileTransferConfig,
    server_address: Arc<Mutex<Option<SocketAddr>>>,
    shutdown_sender: Option<mpsc::Sender<()>>,
    buffer_pool: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl FileTransferManager {
    /// Create a new file transfer manager
    pub fn new(config: FileTransferConfig) -> Self {
        // Ensure receive directory exists
        if !config.receive_dir.exists() {
            if let Err(e) = fs::create_dir_all(&config.receive_dir) {
                warn!(
                    "Failed to create receive directory {}: {}",
                    config.receive_dir.display(),
                    e
                );
            }
        }

        // Create buffer pool for reuse
        let mut buffer_pool = Vec::new();
        for _ in 0..BUFFER_POOL_SIZE {
            buffer_pool.push(vec![0u8; config.chunk_size]);
        }

        Self {
            config,
            server_address: Arc::new(Mutex::new(None)),
            shutdown_sender: None,
            buffer_pool: Arc::new(Mutex::new(buffer_pool)),
        }
    }

    /// Start the file transfer server
    pub async fn start_server(&mut self) -> Result<SocketAddr> {
        // Create a channel to signal shutdown
        let (tx, mut rx) = mpsc::channel(1);
        self.shutdown_sender = Some(tx);

        // Attempt to bind to the configured port
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        let listener = TcpListener::bind(addr).await?;
        let server_addr = listener.local_addr()?;
        
        // Store the server address
        {
            let mut addr_guard = self.server_address.lock().await;
            *addr_guard = Some(server_addr);
        }

        info!("File transfer server started on {}", server_addr);

        // Clone necessary items for the server task
        let config = self.config.clone();
        let buffer_pool = self.buffer_pool.clone();

        // Spawn the server task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Accept a new connection
                    conn_result = listener.accept() => {
                        match conn_result {
                            Ok((socket, addr)) => {
                                info!("New file transfer connection from {}", addr);
                                
                                // Clone items needed for the handler
                                let handler_config = config.clone();
                                let handler_pool = buffer_pool.clone();
                                
                                // Spawn a task to handle this connection
                                tokio::spawn(async move {
                                    if let Err(e) = handle_incoming_file(socket, handler_config, handler_pool).await {
                                        error!("Error handling file transfer from {}: {}", addr, e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Error accepting connection: {}", e);
                            }
                        }
                    }
                    
                    // Check for shutdown signal
                    _ = rx.recv() => {
                        info!("Shutting down file transfer server");
                        break;
                    }
                }
            }
        });

        Ok(server_addr)
    }

    /// Stop the file transfer server
    pub async fn stop_server(&mut self) {
        if let Some(tx) = self.shutdown_sender.take() {
            let _ = tx.send(()).await;
            info!("Sent shutdown signal to file transfer server");
        }
    }

    /// Calculate SHA256 hash of a file
    fn calculate_file_hash(path: &Path) -> Result<String> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024 * 1024]; // 1MB buffer for reading
        
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Send a file to a remote node
    pub async fn send_file<P: AsRef<Path>>(&self, path: P, target_addr: SocketAddr) -> Result<String> {
        let path = path.as_ref();
        
        // Generate a unique ID for this transfer
        let file_id = Uuid::new_v4().to_string();
        
        // Get file metadata
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for file {}", path.display()))?;
        
        let file_size = metadata.len();
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid file path"))?
            .to_string_lossy()
            .to_string();

        // Calculate file hash (for integrity verification)
        let file_hash = Self::calculate_file_hash(path)
            .with_context(|| format!("Failed to calculate hash for file {}", path.display()))?;
        info!("File hash (SHA256): {}", file_hash);

        // Notify of transfer start
        if let Some(callback) = &self.config.progress_callback {
            callback(TransferStatus::Started {
                file_id: file_id.clone(),
                file_name: file_name.clone(),
                file_size,
            });
        }

        // Start timing the transfer
        let start_time = std::time::Instant::now();

        // Open connections for transfer (multiple streams for parallelism)
        let mut handles = vec![];
        let chunk_count = (file_size as f64 / self.config.chunk_size as f64).ceil() as u64;
        let chunks_per_stream = (chunk_count as f64 / self.config.concurrent_streams as f64).ceil() as u64;

        // Track total bytes sent for progress updates
        let total_bytes_sent = Arc::new(Mutex::new(0u64));

        // Set up progress reporting task
        let progress_callback = self.config.progress_callback.clone();
        let total_bytes = file_size;
        let file_id_clone = file_id.clone();
        let progress_bytes_sent = total_bytes_sent.clone();
        
        let progress_task = if progress_callback.is_some() {
            let handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
                loop {
                    interval.tick().await;
                    let bytes_sent = *progress_bytes_sent.lock().await;
                    
                    if bytes_sent >= total_bytes {
                        break;
                    }
                    
                    if let Some(cb) = &progress_callback {
                        let percent = (bytes_sent as f32 / total_bytes as f32) * 100.0;
                        cb(TransferStatus::Progress {
                            file_id: file_id_clone.clone(),
                            bytes_transferred: bytes_sent,
                            total_bytes,
                            percent_complete: percent,
                        });
                    }
                }
            });
            Some(handle)
        } else {
            None
        };

        // Launch stream tasks
        for stream_idx in 0..self.config.concurrent_streams {
            let start_chunk = stream_idx as u64 * chunks_per_stream;
            let end_chunk = std::cmp::min((stream_idx as u64 + 1) * chunks_per_stream, chunk_count);
            
            if start_chunk >= end_chunk {
                break; // No more chunks to send
            }
            
            // Calculate byte ranges
            let start_pos = start_chunk * self.config.chunk_size as u64;
            let end_pos = std::cmp::min(end_chunk * self.config.chunk_size as u64, file_size);
            
            // Clone required values
            let path = path.to_path_buf();
            let target = target_addr;
            let chunk_size = self.config.chunk_size;
            let file_id = file_id.clone();
            let file_name = file_name.clone();
            let bytes_sent = total_bytes_sent.clone();
            let file_hash_clone = file_hash.clone();
            
            // Spawn a task for this stream
            let handle = tokio::spawn(async move {
                let stream_name = format!("Stream {}: range {}-{}", stream_idx, start_pos, end_pos);
                info!("Starting {}", stream_name);
                
                let result = send_file_range(
                    &path,
                    target,
                    file_id,
                    file_name,
                    start_pos,
                    end_pos,
                    chunk_size,
                    bytes_sent,
                    file_hash_clone,
                ).await;
                
                if let Err(e) = &result {
                    error!("Error in {}: {}", stream_name, e);
                } else {
                    info!("Completed {}", stream_name);
                }
                
                result
            });
            
            handles.push(handle);
        }

        // Wait for all transfers to complete
        let mut success = true;
        let mut errors = Vec::new();
        
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok(_)) => {
                    // This stream completed successfully
                }
                Ok(Err(e)) => {
                    success = false;
                    errors.push(format!("Stream {} failed: {}", i, e));
                }
                Err(e) => {
                    success = false;
                    errors.push(format!("Stream {} task panicked: {}", i, e));
                }
            }
        }
        
        // Cancel progress task if it's still running
        if let Some(handle) = progress_task {
            handle.abort();
        }
        
        // Calculate final statistics
        let elapsed = start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f32();
        let bytes_sent = *total_bytes_sent.lock().await;
        let throughput = if elapsed_secs > 0.0 {
            (bytes_sent as f32 / elapsed_secs) / (1024.0 * 1024.0)
        } else {
            0.0
        };

        // Report final status
        if let Some(callback) = &self.config.progress_callback {
            if success {
                callback(TransferStatus::Completed {
                    file_id: file_id.clone(),
                    bytes_transferred: file_size,
                    elapsed_seconds: elapsed_secs,
                    throughput_mbps: throughput,
                });
            } else {
                callback(TransferStatus::Failed {
                    file_id: file_id.clone(),
                    error: errors.join(", "),
                });
            }
        }

        if success {
            info!(
                "File transfer complete: {} ({:.2} MB/s)",
                path.display(),
                throughput
            );
            Ok(file_id)
        } else {
            Err(anyhow!("File transfer failed"))
        }
    }

    /// Get a buffer from the pool or create a new one if none are available
    async fn get_buffer(&self) -> Vec<u8> {
        let mut pool = self.buffer_pool.lock().await;
        if let Some(buffer) = pool.pop() {
            buffer
        } else {
            // Create a new buffer if pool is empty
            vec![0u8; self.config.chunk_size]
        }
    }

    /// Return a buffer to the pool
    async fn return_buffer(&self, mut buffer: Vec<u8>) {
        // Clear buffer data before returning to pool
        buffer.clear();
        buffer.resize(self.config.chunk_size, 0);
        
        let mut pool = self.buffer_pool.lock().await;
        pool.push(buffer);
    }

    /// Get the address of the file transfer server
    pub async fn server_address(&self) -> Option<SocketAddr> {
        let guard = self.server_address.lock().await;
        *guard
    }

    /// Get the receive directory path
    pub fn receive_directory(&self) -> PathBuf {
        self.config.receive_dir.clone()
    }
}

/// Handle an incoming file transfer
async fn handle_incoming_file(
    mut socket: TcpStream,
    config: FileTransferConfig,
    buffer_pool: Arc<Mutex<Vec<Vec<u8>>>>,
) -> Result<()> {
    // Read the header (file ID, file name, and file size)
    let mut id_len_buf = [0u8; 4];
    socket.read_exact(&mut id_len_buf).await?;
    let id_len = u32::from_be_bytes(id_len_buf) as usize;
    
    let mut id_buf = vec![0u8; id_len];
    socket.read_exact(&mut id_buf).await?;
    let file_id = String::from_utf8(id_buf)?;
    
    let mut name_len_buf = [0u8; 4];
    socket.read_exact(&mut name_len_buf).await?;
    let name_len = u32::from_be_bytes(name_len_buf) as usize;
    
    let mut name_buf = vec![0u8; name_len];
    socket.read_exact(&mut name_buf).await?;
    let file_name = String::from_utf8(name_buf)?;
    
    let mut size_buf = [0u8; 8];
    socket.read_exact(&mut size_buf).await?;
    let file_size = u64::from_be_bytes(size_buf);
    
    // Additional fields for partial transfers
    let mut start_pos_buf = [0u8; 8];
    socket.read_exact(&mut start_pos_buf).await?;
    let start_pos = u64::from_be_bytes(start_pos_buf);
    
    let mut end_pos_buf = [0u8; 8];
    socket.read_exact(&mut end_pos_buf).await?;
    let end_pos = u64::from_be_bytes(end_pos_buf);
    
    // Read file hash
    let mut hash_len_buf = [0u8; 4];
    socket.read_exact(&mut hash_len_buf).await?;
    let hash_len = u32::from_be_bytes(hash_len_buf) as usize;
    
    let mut hash_buf = vec![0u8; hash_len];
    socket.read_exact(&mut hash_buf).await?;
    let expected_hash = String::from_utf8(hash_buf)?;
    
    info!(
        "Receiving file: {} (ID: {}), size: {}B, range: {}-{}, expected hash: {}",
        file_name, file_id, file_size, start_pos, end_pos, expected_hash
    );
    
    // Notify of transfer start
    if let Some(callback) = &config.progress_callback {
        callback(TransferStatus::Started {
            file_id: file_id.clone(),
            file_name: file_name.clone(),
            file_size,
        });
    }
    
    // Prepare output file
    let file_path = config.receive_dir.join(&file_name);
    
    // Use a file tracking mechanism for multi-part transfers
    let tracking_path = config.receive_dir.join(format!("{}.parts", file_id));
    let range_key = format!("{}-{}", start_pos, end_pos);
    
    // Store hash in hash tracking file
    let hash_tracking_path = config.receive_dir.join(format!("{}.hash", file_id));
    if !hash_tracking_path.exists() {
        fs::write(&hash_tracking_path, &expected_hash)?;
    }
    
    // Create or update the tracking file to indicate this part is being transferred
    {
        let mut parts = if tracking_path.exists() {
            let content = fs::read_to_string(&tracking_path)?;
            serde_json::from_str::<HashMap<String, bool>>(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };
        
        // Mark this range as in progress
        parts.insert(range_key.clone(), false); // not completed yet
        
        fs::write(&tracking_path, serde_json::to_string(&parts)?)?;
    }
    
    let file = Arc::new(Mutex::new({
        // Ensure the file exists and has the right size
        if !file_path.exists() {
            // Create new file and set its size
            let mut file = File::create(&file_path)?;
            file.set_len(file_size)?;
            file
        } else {
            // File exists, open for writing
            let mut file = File::options()
                .read(true)
                .write(true)
                .open(&file_path)?;
            
            // If file size is wrong, fix it
            if file.metadata()?.len() != file_size {
                file.set_len(file_size)?;
            }
            file
        }
    }));
    
    // Seek to the correct position for this part
    {
        let mut file_guard = file.lock().await;
        file_guard.seek(SeekFrom::Start(start_pos))?;
    }
    
    // Start time for throughput calculation
    let start_time = std::time::Instant::now();
    
    // Read and process data
    let mut bytes_received = 0;
    let mut buffer = if let Ok(mut pool) = buffer_pool.try_lock() {
        pool.pop().unwrap_or_else(|| vec![0u8; config.chunk_size])
    } else {
        vec![0u8; config.chunk_size]
    };
    
    while bytes_received < (end_pos - start_pos) {
        let max_bytes = std::cmp::min(
            buffer.len() as u64,
            (end_pos - start_pos) - bytes_received,
        ) as usize;
        
        let read_buf = &mut buffer[..max_bytes];
        let n = socket.read(read_buf).await?;
        
        if n == 0 {
            // EOF before expected end
            return Err(anyhow!("Connection closed prematurely"));
        }
        
        // Write to file
        {
            let mut file_guard = file.lock().await;
            file_guard.write_all(&buffer[..n])?;
        }
        
        bytes_received += n as u64;
        
        // Report progress
        if let Some(callback) = &config.progress_callback {
            let total_received = start_pos + bytes_received;
            let percent = (total_received as f32 / file_size as f32) * 100.0;
            callback(TransferStatus::Progress {
                file_id: file_id.clone(),
                bytes_transferred: total_received,
                total_bytes: file_size,
                percent_complete: percent,
            });
        }
    }
    
    // Calculate throughput
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs_f32();
    let throughput = if elapsed_secs > 0.0 {
        (bytes_received as f32 / elapsed_secs) / (1024.0 * 1024.0)
    } else {
        0.0
    };
    
    // Report completion
    if let Some(callback) = &config.progress_callback {
        callback(TransferStatus::Completed {
            file_id: file_id.clone(),
            bytes_transferred: start_pos + bytes_received,
            elapsed_seconds: elapsed_secs,
            throughput_mbps: throughput,
        });
    }
    
    // Return buffer to pool
    if let Ok(mut pool) = buffer_pool.try_lock() {
        buffer.clear();
        buffer.resize(config.chunk_size, 0);
        pool.push(buffer);
    }
    
    info!(
        "File received: {} ({:.2} MB/s)",
        file_name, throughput
    );
    
    // Update the tracking file to mark this part as complete
    {
        let mut parts = if tracking_path.exists() {
            let content = fs::read_to_string(&tracking_path)?;
            serde_json::from_str::<HashMap<String, bool>>(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };
        
        // Mark this range as completed
        parts.insert(range_key, true);
        
        // Check if all parts are complete
        let all_complete = parts.values().all(|&complete| complete);
        
        fs::write(&tracking_path, serde_json::to_string(&parts)?)?;
        
        // If all parts are complete, we can verify the hash and clean up
        if all_complete {
            info!("All parts of file {} received successfully", file_name);
            
            // Verify file integrity with hash
            let hash_tracking_path = config.receive_dir.join(format!("{}.hash", file_id));
            if hash_tracking_path.exists() {
                let expected_hash = fs::read_to_string(&hash_tracking_path)?;
                
                // Calculate actual hash of the complete file
                match FileTransferManager::calculate_file_hash(&file_path) {
                    Ok(actual_hash) => {
                        if actual_hash == expected_hash {
                            info!("✅ Hash verification successful: File integrity confirmed");
                        } else {
                            error!("❌ Hash verification failed: File may be corrupted");
                            error!("Expected: {}", expected_hash);
                            error!("Actual:   {}", actual_hash);
                        }
                    }
                    Err(e) => {
                        error!("Failed to calculate hash for verification: {}", e);
                    }
                }
            }
            
            // Clean up tracking files
            let _ = fs::remove_file(&tracking_path);
            let _ = fs::remove_file(&hash_tracking_path);
        } else {
            info!("Partial transfer of {}: {}/{} parts complete", 
                   file_name, 
                   parts.values().filter(|&&v| v).count(),
                   parts.len());
        }
    }
    
    Ok(())
}

/// Send a range of a file over a TCP connection
async fn send_file_range(
    path: &Path,
    target_addr: SocketAddr,
    file_id: String,
    file_name: String,
    start_pos: u64,
    end_pos: u64,
    chunk_size: usize,
    bytes_sent_counter: Arc<Mutex<u64>>,
    file_hash: String,
) -> Result<()> {
    // Connect to target
    let mut socket = TcpStream::connect(target_addr).await?;
    
    // Open the file
    let mut file = File::open(path)?;
    
    // Send header
    let id_bytes = file_id.as_bytes();
    let id_len = id_bytes.len() as u32;
    socket.write_all(&id_len.to_be_bytes()).await?;
    socket.write_all(id_bytes).await?;
    
    let name_bytes = file_name.as_bytes();
    let name_len = name_bytes.len() as u32;
    socket.write_all(&name_len.to_be_bytes()).await?;
    socket.write_all(name_bytes).await?;
    
    // Get total file size
    let file_size = file.metadata()?.len();
    socket.write_all(&file_size.to_be_bytes()).await?;
    
    // Send range information
    socket.write_all(&start_pos.to_be_bytes()).await?;
    socket.write_all(&end_pos.to_be_bytes()).await?;
    
    // Send file hash for integrity verification
    let hash_bytes = file_hash.as_bytes();
    let hash_len = hash_bytes.len() as u32;
    socket.write_all(&hash_len.to_be_bytes()).await?;
    socket.write_all(hash_bytes).await?;
    
    // Seek to start position
    file.seek(SeekFrom::Start(start_pos))?;
    
    // Send file data
    let mut buffer = vec![0u8; chunk_size];
    let mut position = start_pos;
    
    while position < end_pos {
        let max_bytes = std::cmp::min(chunk_size as u64, end_pos - position) as usize;
        let n = file.read(&mut buffer[..max_bytes])?;
        
        if n == 0 {
            break; // EOF
        }
        
        socket.write_all(&buffer[..n]).await?;
        position += n as u64;
        
        // Update the shared counter
        {
            let mut counter = bytes_sent_counter.lock().await;
            *counter += n as u64;
        }
    }
    
    debug!("Completed sending range {}-{}", start_pos, end_pos);
    Ok(())
}

/// Test the file transfer functionality with a loopback transfer
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_loopback_transfer() -> Result<()> {
        let _ = env_logger::builder().is_test(true).try_init();
        
        // Create temporary directories
        let send_dir = tempdir()?;
        let receive_dir = tempdir()?;
        
        // Create a test file
        let test_file_path = send_dir.path().join("test_file.dat");
        let file_size = 5 * 1024 * 1024; // 5MB
        {
            let mut file = File::create(&test_file_path)?;
            let data = vec![0x55u8; 1024]; // Pattern to repeat
            
            for _ in 0..(file_size / 1024) {
                file.write_all(&data)?;
            }
        }
        
        // Set up progress tracking
        let received_started = Arc::new(AtomicBool::new(false));
        let received_completed = Arc::new(AtomicBool::new(false));
        let received_bytes = Arc::new(Mutex::new(0u64));
        
        let r_started = received_started.clone();
        let r_completed = received_completed.clone();
        let r_bytes = received_bytes.clone();
        
        let progress_callback: ProgressCallback = Arc::new(move |status| {
            match status {
                TransferStatus::Started { .. } => {
                    r_started.store(true, Ordering::SeqCst);
                }
                TransferStatus::Progress { bytes_transferred, .. } => {
                    let mut b = r_bytes.blocking_lock();
                    *b = bytes_transferred;
                }
                TransferStatus::Completed { .. } => {
                    r_completed.store(true, Ordering::SeqCst);
                }
                TransferStatus::Failed { error, .. } => {
                    panic!("Transfer failed: {}", error);
                }
            }
        });
        
        // Create and start the file transfer manager
        let config = FileTransferConfig {
            chunk_size: 64 * 1024, // 64KB chunks for faster test
            port: 0, // Use a random port
            receive_dir: receive_dir.path().to_path_buf(),
            progress_callback: Some(progress_callback),
            concurrent_streams: 2,
        };
        
        let mut manager = FileTransferManager::new(config);
        let server_addr = manager.start_server().await?;
        
        // Send the file to ourselves
        let file_id = manager.send_file(&test_file_path, server_addr).await?;
        
        // Verify the transfer completed
        assert!(received_started.load(Ordering::SeqCst), "Transfer never started");
        assert!(received_completed.load(Ordering::SeqCst), "Transfer never completed");
        
        {
            let bytes = received_bytes.lock().await;
            assert_eq!(*bytes, file_size as u64, "Incorrect number of bytes transferred");
        }
        
        // Verify the received file
        let received_file_path = receive_dir.path().join("test_file.dat");
        assert!(received_file_path.exists(), "Received file does not exist");
        
        let received_metadata = fs::metadata(&received_file_path)?;
        assert_eq!(received_metadata.len(), file_size as u64, "Received file has incorrect size");
        
        // Shutdown the server
        manager.stop_server().await;
        
        Ok(())
    }
} 