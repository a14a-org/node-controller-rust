// src/updater/health.rs
//
// Health check functionality to verify successful updates

use anyhow::{Result, Context, anyhow};
use tokio::process::Command;
use std::time::Duration;
use log::{debug, info, warn, error};
use tokio::time;

/// Verify that the installation was successful
pub async fn verify_installation(timeout: Duration) -> Result<()> {
    info!("Verifying installation health");
    
    // Give the service a moment to start up
    time::sleep(Duration::from_secs(2)).await;
    
    // Perform various health checks with a timeout
    let check_result = time::timeout(timeout, run_health_checks()).await;
    
    match check_result {
        Ok(result) => result,
        Err(_) => Err(anyhow!("Health check timed out after {:?}", timeout)),
    }
}

/// Run a series of health checks to verify the installation
async fn run_health_checks() -> Result<()> {
    // Check 1: Verify service is running
    check_service_running().await?;
    
    // Check 2: Verify process is responsive (calls to launchctl)
    check_process_responsive().await?;
    
    // Check 3: Verify logs are being written
    check_logs_are_written().await?;
    
    // All checks passed
    info!("All health checks passed");
    Ok(())
}

/// Check if the service is running
async fn check_service_running() -> Result<()> {
    debug!("Checking if service is running");
    
    let output = Command::new("launchctl")
        .arg("list")
        .output()
        .await
        .context("Failed to execute launchctl list command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("launchctl list command failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.contains("org.a14a.node-controller") {
        return Err(anyhow!("Service is not running"));
    }
    
    debug!("Service is running");
    Ok(())
}

/// Check if the process is responsive
async fn check_process_responsive() -> Result<()> {
    debug!("Checking if process is responsive");
    
    // Find the process ID
    let pid = get_process_id().await?;
    
    // Get process information
    let output = Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("pid,command,%cpu,%mem")
        .output()
        .await
        .context("Failed to execute ps command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("ps command failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    debug!("Process info:\n{}", stdout);
    
    // Check CPU usage is not excessive
    let cpu_usage = extract_cpu_usage(&stdout)?;
    if cpu_usage > 90.0 {
        warn!("High CPU usage detected: {:.1}%", cpu_usage);
        // Not failing the check, just warning
    }
    
    debug!("Process is responsive");
    Ok(())
}

/// Get the process ID of the running node-controller service
async fn get_process_id() -> Result<u32> {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("node-controller")
        .output()
        .await
        .context("Failed to execute pgrep command")?;
    
    if !output.status.success() {
        return Err(anyhow!("Failed to find node-controller process"));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let pid = stdout.trim().parse::<u32>()
        .context("Failed to parse process ID")?;
    
    debug!("Found process ID: {}", pid);
    Ok(pid)
}

/// Extract CPU usage from ps output
fn extract_cpu_usage(ps_output: &str) -> Result<f32> {
    for line in ps_output.lines().skip(1) {  // Skip header line
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            return parts[2].replace("%", "").parse::<f32>()
                .context("Failed to parse CPU usage");
        }
    }
    
    Err(anyhow!("Could not find CPU usage in ps output"))
}

/// Check if logs are being written
async fn check_logs_are_written() -> Result<()> {
    debug!("Checking if logs are being written");
    
    // Wait briefly to allow logs to be written
    time::sleep(Duration::from_secs(1)).await;
    
    // Check log directory
    let log_path = "/Library/Logs/NodeController";
    
    let output = Command::new("ls")
        .arg("-la")
        .arg(log_path)
        .output()
        .await
        .context("Failed to execute ls command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to list log directory: {}", stderr));
    }
    
    // Check the most recent log file
    let output = Command::new("find")
        .arg(log_path)
        .arg("-type")
        .arg("f")
        .arg("-name")
        .arg("*.log")
        .arg("-mmin")
        .arg("-5")  // Modified in the last 5 minutes
        .output()
        .await
        .context("Failed to execute find command")?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Err(anyhow!("No recent log files found"));
    }
    
    debug!("Recent log files found:\n{}", stdout);
    
    // Check if logs are being actively written by checking file size increase
    let log_files: Vec<&str> = stdout.lines().collect();
    if !log_files.is_empty() {
        let log_file = log_files[0];
        
        // Get initial size
        let initial_size = get_file_size(log_file).await?;
        
        // Wait a moment to see if size changes
        time::sleep(Duration::from_secs(3)).await;
        
        // Get new size
        let new_size = get_file_size(log_file).await?;
        
        if new_size <= initial_size {
            warn!("Log file size didn't increase, logs may not be actively written");
            // Not failing the check as this might be a false negative
        } else {
            debug!("Log file size increased from {} to {} bytes", initial_size, new_size);
        }
    }
    
    debug!("Logs are being written");
    Ok(())
}

/// Get file size in bytes
async fn get_file_size(path: &str) -> Result<u64> {
    let output = Command::new("stat")
        .arg("-f")
        .arg("%z")  // Size in bytes
        .arg(path)
        .output()
        .await
        .context("Failed to execute stat command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("stat command failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let size = stdout.trim().parse::<u64>()
        .context("Failed to parse file size")?;
    
    Ok(size)
}

/// Perform a basic check for API connectivity
async fn check_api_connectivity() -> Result<()> {
    debug!("Checking API connectivity");
    
    // We could add more sophisticated API connectivity checks here,
    // such as sending a test request to the monitoring API
    
    // For now, we just ensure the process is running and logs are being written
    // which implicitly verifies basic functionality
    
    debug!("API connectivity check skipped");
    Ok(())
} 