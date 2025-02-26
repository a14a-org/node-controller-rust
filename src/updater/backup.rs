// src/updater/backup.rs
//
// Backup and restoration functionality for the auto-update system

use anyhow::{Result, Context, anyhow};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use log::{debug, info, warn, error};
use chrono::Utc;
use crate::updater::UpdateConfig;
use std::os::unix::fs::PermissionsExt;

/// Location of the current application binary
const APP_BINARY_PATH: &str = "/Applications/NodeController/bin/node-controller";

/// Location of the restore script that will be created
const RESTORE_SCRIPT_PATH: &str = "/Library/NodeController/updates/restore.sh";

/// Create a backup of the current installation
pub async fn create_backup(update_dir: &Path) -> Result<PathBuf> {
    info!("Creating backup of current installation");
    
    // Create backup directory
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_dir = update_dir.join(format!("backup_{}", timestamp));
    
    fs::create_dir_all(&backup_dir).await
        .context("Failed to create backup directory")?;
    
    info!("Backup directory created at {}", backup_dir.display());
    
    // Create directories in the backup
    let bin_dir = backup_dir.join("bin");
    fs::create_dir_all(&bin_dir).await
        .context("Failed to create bin directory in backup")?;
    
    // Check if the application binary exists
    if !Path::new(APP_BINARY_PATH).exists() {
        return Err(anyhow!("Application binary not found at {}", APP_BINARY_PATH));
    }
    
    // Copy application binary
    fs::copy(APP_BINARY_PATH, bin_dir.join("node-controller")).await
        .context("Failed to copy application binary to backup")?;
    
    info!("Application binary backed up successfully");
    
    // Backup configuration files
    backup_config_files(&backup_dir).await?;
    
    // Create restore script
    create_restore_script(&backup_dir).await?;
    
    Ok(backup_dir)
}

/// Backup configuration files
async fn backup_config_files(backup_dir: &Path) -> Result<()> {
    info!("Backing up configuration files");
    
    let config_dir = Path::new("/Library/NodeController/config");
    if !config_dir.exists() {
        warn!("Config directory not found, skipping config backup");
        return Ok(());
    }
    
    // Create config directory in backup
    let backup_config_dir = backup_dir.join("config");
    fs::create_dir_all(&backup_config_dir).await
        .context("Failed to create config directory in backup")?;
    
    // Copy all files from config directory
    copy_directory_contents(config_dir, &backup_config_dir).await
        .context("Failed to copy configuration files")?;
    
    info!("Configuration files backed up successfully");
    Ok(())
}

/// Copy all files from one directory to another
async fn copy_directory_contents(from: &Path, to: &Path) -> Result<()> {
    debug!("Copying directory contents from {} to {}", from.display(), to.display());
    
    // Create destination directory if it doesn't exist
    if !to.exists() {
        fs::create_dir_all(to).await
            .context("Failed to create destination directory")?;
    }
    
    // Get list of files in source directory
    let entries = fs::read_dir(from).await
        .context("Failed to read source directory")?;
    
    // Copy each entry
    let mut entry = entries.next_entry().await?;
    while let Some(entry_info) = entry {
        let src_path = entry_info.path();
        let dst_path = to.join(entry_info.file_name());
        
        let metadata = entry_info.metadata().await
            .context("Failed to read file metadata")?;
        
        if metadata.is_file() {
            // Copy file
            fs::copy(&src_path, &dst_path).await
                .context(format!(
                    "Failed to copy {} to {}", 
                    src_path.display(), 
                    dst_path.display()
                ))?;
                
            debug!("Copied file {} to {}", src_path.display(), dst_path.display());
        } else if metadata.is_dir() {
            // Recursively copy directory
            copy_directory_contents(&src_path, &dst_path).await?;
        }
        
        entry = entries.next_entry().await?;
    }
    
    Ok(())
}

/// Create a restore script that can recover from a failed update
async fn create_restore_script(backup_dir: &Path) -> Result<()> {
    info!("Creating restore script");
    
    let script_content = format!(
        r#"#!/bin/bash
# Auto-generated restore script for node-controller
# Created on: {}
# This script restores a backup from a failed update

set -e

BACKUP_DIR="{}"
APP_DIR="/Applications/NodeController"
CONFIG_DIR="/Library/NodeController/config"

echo "Restoring node-controller from backup..."

# Stop the service
if launchctl list | grep -q "org.a14a.node-controller"; then
    echo "Stopping node-controller service..."
    launchctl unload /Library/LaunchDaemons/org.a14a.node-controller.plist || true
fi

# Restore application binary
echo "Restoring application binary..."
mkdir -p "$APP_DIR/bin"
cp "$BACKUP_DIR/bin/node-controller" "$APP_DIR/bin/node-controller"
chmod 755 "$APP_DIR/bin/node-controller"
chown root:wheel "$APP_DIR/bin/node-controller"

# Restore configuration files
if [ -d "$BACKUP_DIR/config" ]; then
    echo "Restoring configuration files..."
    mkdir -p "$CONFIG_DIR"
    cp -R "$BACKUP_DIR/config/"* "$CONFIG_DIR/"
    chown -R root:wheel "$CONFIG_DIR"
fi

# Restart the service
echo "Restarting node-controller service..."
launchctl load /Library/LaunchDaemons/org.a14a.node-controller.plist

echo "Restore completed successfully!"
"#,
        Utc::now().to_rfc3339(),
        backup_dir.display()
    );
    
    // Write the restore script
    fs::write(RESTORE_SCRIPT_PATH, script_content).await
        .context("Failed to write restore script")?;
    
    // Make the script executable
    let mut perms = fs::metadata(RESTORE_SCRIPT_PATH).await?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    fs::set_permissions(RESTORE_SCRIPT_PATH, perms).await
        .context("Failed to set permissions on restore script")?;
    
    info!("Restore script created at {}", RESTORE_SCRIPT_PATH);
    Ok(())
}

/// Restore from a backup after a failed update
pub async fn restore_from_backup(backup_dir: &Path) -> Result<()> {
    info!("Restoring from backup at {}", backup_dir.display());
    
    // Execute the restore script
    let output = Command::new("sudo")
        .arg(RESTORE_SCRIPT_PATH)
        .output()
        .await
        .context("Failed to execute restore script")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Restore script failed: {}", stderr);
        return Err(anyhow!("Restore script failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("Restore completed: {}", stdout);
    
    Ok(())
}

/// Install an update from a downloaded file
pub async fn install_update(download_path: &Path, config: &UpdateConfig) -> Result<()> {
    info!("Installing update from {}", download_path.display());
    
    // Create a temporary directory for extraction
    let extract_dir = config.update_dir.join("extract_temp");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).await
            .context("Failed to remove existing temporary extraction directory")?;
    }
    
    fs::create_dir_all(&extract_dir).await
        .context("Failed to create temporary extraction directory")?;
    
    // Extract the archive
    if download_path.extension().map_or(false, |ext| ext == "zip") {
        extract_zip(download_path, &extract_dir).await?;
    } else if download_path.to_string_lossy().ends_with(".tar.gz") || 
              download_path.extension().map_or(false, |ext| ext == "gz") 
    {
        extract_tar(download_path, &extract_dir).await?;
    } else {
        return Err(anyhow!("Unknown archive format for {}", download_path.display()));
    }
    
    // Find the binary in the extracted files
    let binary_path = find_binary_in_directory(&extract_dir).await?;
    
    // Stop the service
    stop_service().await?;
    
    // Install the new binary
    fs::copy(&binary_path, APP_BINARY_PATH).await
        .context("Failed to copy new binary to installation directory")?;
    
    // Set proper permissions
    let mut perms = fs::metadata(APP_BINARY_PATH).await?.permissions();
    perms.set_mode(0o755); // rwxr-xr-x
    fs::set_permissions(APP_BINARY_PATH, perms).await
        .context("Failed to set permissions on new binary")?;
    
    // Set ownership
    set_ownership(APP_BINARY_PATH, "root", "wheel").await?;
    
    // Start the service
    start_service().await?;
    
    // Execute any post-update commands
    for cmd in &config.post_update_commands {
        execute_post_update_command(cmd).await?;
    }
    
    // Clean up
    fs::remove_dir_all(&extract_dir).await
        .context("Failed to clean up temporary extraction directory")?;
    
    info!("Update installed successfully");
    Ok(())
}

/// Extract a zip archive
async fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<()> {
    debug!("Extracting zip archive: {} to {}", zip_path.display(), target_dir.display());
    
    let output = Command::new("unzip")
        .arg("-q")  // quiet
        .arg("-o")  // overwrite
        .arg(zip_path)
        .arg("-d")  // destination
        .arg(target_dir)
        .output()
        .await
        .context("Failed to execute unzip command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to extract zip archive: {}", stderr));
    }
    
    debug!("Zip archive extracted successfully");
    Ok(())
}

/// Extract a tar.gz archive
async fn extract_tar(tar_path: &Path, target_dir: &Path) -> Result<()> {
    debug!("Extracting tar.gz archive: {} to {}", tar_path.display(), target_dir.display());
    
    let output = Command::new("tar")
        .arg("-xzf")  // extract, gzip, file
        .arg(tar_path)
        .arg("-C")    // change directory
        .arg(target_dir)
        .output()
        .await
        .context("Failed to execute tar command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to extract tar.gz archive: {}", stderr));
    }
    
    debug!("Tar.gz archive extracted successfully");
    Ok(())
}

/// Find the application binary in the extracted directory
async fn find_binary_in_directory(dir: &Path) -> Result<PathBuf> {
    debug!("Looking for binary in {}", dir.display());
    
    // Define possible binary names and paths
    let possible_binaries = [
        "node-controller",
        "node-controller-rust",
        "bin/node-controller",
        "bin/node-controller-rust",
    ];
    
    // First try direct matches
    for bin_name in &possible_binaries {
        let bin_path = dir.join(bin_name);
        if bin_path.exists() {
            let metadata = fs::metadata(&bin_path).await?;
            if metadata.is_file() {
                // Check if it's executable
                let perms = metadata.permissions();
                if perms.mode() & 0o111 != 0 {
                    debug!("Found binary at {}", bin_path.display());
                    return Ok(bin_path);
                }
            }
        }
    }
    
    // If not found, do a recursive search
    find_executable_file(dir).await
        .context("Could not find executable binary in extracted files")
}

/// Recursively find an executable file
async fn find_executable_file(dir: &Path) -> Result<PathBuf> {
    debug!("Recursively searching for executable in {}", dir.display());
    
    let mut entries = fs::read_dir(dir).await
        .context("Failed to read directory")?;
        
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = entry.metadata().await?;
        
        if metadata.is_file() {
            // Check if file is executable
            let perms = metadata.permissions();
            let is_executable = perms.mode() & 0o111 != 0;
            
            if is_executable {
                // Check file name to ensure it's our binary
                let filename = path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                    
                if filename.contains("node-controller") {
                    debug!("Found executable at {}", path.display());
                    return Ok(path);
                }
            }
        } else if metadata.is_dir() {
            // Recursively search subdirectories
            match find_executable_file(&path).await {
                Ok(result) => return Ok(result),
                Err(_) => continue,  // Continue searching if not found in this subdirectory
            }
        }
    }
    
    Err(anyhow!("No executable found in directory {}", dir.display()))
}

/// Stop the node-controller service
async fn stop_service() -> Result<()> {
    info!("Stopping node-controller service");
    
    let output = Command::new("sudo")
        .arg("launchctl")
        .arg("unload")
        .arg("/Library/LaunchDaemons/org.a14a.node-controller.plist")
        .output()
        .await
        .context("Failed to execute launchctl unload command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Warning when stopping service: {}", stderr);
        // Don't return an error, as the service might not be running
    }
    
    // Add a small delay to ensure the service is stopped
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    info!("Service stopped successfully");
    Ok(())
}

/// Start the node-controller service
async fn start_service() -> Result<()> {
    info!("Starting node-controller service");
    
    let output = Command::new("sudo")
        .arg("launchctl")
        .arg("load")
        .arg("/Library/LaunchDaemons/org.a14a.node-controller.plist")
        .output()
        .await
        .context("Failed to execute launchctl load command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to start service: {}", stderr));
    }
    
    info!("Service started successfully");
    Ok(())
}

/// Set ownership of a file
async fn set_ownership(path: &str, user: &str, group: &str) -> Result<()> {
    debug!("Setting ownership of {} to {}:{}", path, user, group);
    
    let output = Command::new("sudo")
        .arg("chown")
        .arg(format!("{}:{}", user, group))
        .arg(path)
        .output()
        .await
        .context("Failed to execute chown command")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to set ownership: {}", stderr));
    }
    
    debug!("Ownership set successfully");
    Ok(())
}

/// Execute a post-update command
async fn execute_post_update_command(command: &str) -> Result<()> {
    info!("Executing post-update command: {}", command);
    
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await
        .context(format!("Failed to execute command: {}", command))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Post-update command failed: {}", stderr);
        // Don't return an error, as these are optional
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("Command output: {}", stdout);
    }
    
    Ok(())
}

/// Clean up old backups, keeping only the most recent ones
pub async fn cleanup_old_backups(update_dir: &Path, max_backups: usize) -> Result<()> {
    info!("Cleaning up old backups, keeping {} most recent", max_backups);
    
    // Find all backup directories
    let mut backups = Vec::new();
    let mut entries = fs::read_dir(update_dir).await
        .context("Failed to read update directory")?;
        
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let filename = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
            
        if filename.starts_with("backup_") && path.is_dir().await {
            backups.push(path);
        }
    }
    
    // Sort backups by date (newest first)
    backups.sort_by(|a, b| {
        let a_name = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let b_name = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        b_name.cmp(a_name)  // Reverse order
    });
    
    // Remove old backups
    if backups.len() > max_backups {
        for old_backup in backups.iter().skip(max_backups) {
            info!("Removing old backup: {}", old_backup.display());
            fs::remove_dir_all(old_backup).await
                .context(format!("Failed to remove old backup: {}", old_backup.display()))?;
        }
    }
    
    info!("Backup cleanup completed");
    Ok(())
} 