// src/updater/download.rs
//
// Download and verification of release assets

use anyhow::{Result, Context, anyhow};
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use log::{debug, info, warn};
use crate::updater::github::GithubReleaseInfo;
use std::process::Command;
use tokio::process::Command as TokioCommand;

/// Download a release asset to the specified directory
pub async fn download_release(
    release: &GithubReleaseInfo,
    update_dir: &Path,
) -> Result<PathBuf> {
    // Create the update directory if it doesn't exist
    fs::create_dir_all(update_dir).await
        .context("Failed to create update directory")?;
        
    // Determine file name from download URL
    let file_name = extract_filename_from_url(&release.download_url)?;
    let download_path = update_dir.join(file_name);
    
    info!("Downloading update from {} to {}", release.download_url, download_path.display());
    
    // Create the HTTP client
    let client = reqwest::Client::builder()
        .user_agent("node-controller-updater")
        .build()?;
        
    // Download the file with progress tracking
    let response = client.get(&release.download_url)
        .send()
        .await
        .context("Failed to start download")?;
        
    if !response.status().is_success() {
        return Err(anyhow!("Download failed with status: {}", response.status()));
    }
    
    // Get content length for progress tracking
    let total_size = response.content_length().unwrap_or(0);
    
    // Create the output file
    let mut file = File::create(&download_path).await
        .context(format!("Failed to create file at {}", download_path.display()))?;
        
    // Download the file in chunks
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    
    use futures_util::StreamExt;
    while let Some(item) = stream.next().await {
        let chunk = item.context("Error while downloading file")?;
        file.write_all(&chunk).await
            .context("Error while writing to file")?;
            
        // Update progress
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            let progress = (downloaded * 100) / total_size;
            debug!("Download progress: {}%", progress);
        }
    }
    
    // Close the file
    file.flush().await.context("Failed to flush file")?;
    
    info!("Download completed: {}", download_path.display());
    
    Ok(download_path)
}

/// Verify the integrity of a downloaded release
pub async fn verify_release(download_path: &Path, release: &GithubReleaseInfo) -> Result<()> {
    info!("Verifying downloaded update: {}", download_path.display());
    
    // Verify file exists
    if !download_path.exists() {
        return Err(anyhow!("Downloaded file doesn't exist at {}", download_path.display()));
    }
    
    // Verify file size
    let metadata = fs::metadata(download_path).await
        .context("Failed to get file metadata")?;
        
    let file_size = metadata.len();
    if release.size > 0 && file_size != release.size {
        return Err(anyhow!(
            "File size mismatch: expected {}, got {}",
            release.size, file_size
        ));
    }
    
    // Verify checksum if available
    if let Some(expected_sha256) = &release.sha256 {
        let calculated_sha256 = calculate_sha256(download_path).await
            .context("Failed to calculate SHA256 checksum")?;
            
        if calculated_sha256 != *expected_sha256 {
            return Err(anyhow!(
                "SHA256 checksum mismatch: expected {}, got {}",
                expected_sha256, calculated_sha256
            ));
        }
        
        info!("SHA256 checksum verified successfully");
    } else {
        warn!("No SHA256 checksum provided for verification");
    }
    
    // If it's a zip or tar.gz file, verify it can be extracted
    if download_path.extension().map_or(false, |ext| ext == "zip") {
        verify_zip_archive(download_path).await?;
    } else if download_path
        .to_string_lossy()
        .ends_with(".tar.gz") || download_path.extension().map_or(false, |ext| ext == "gz") 
    {
        verify_tar_archive(download_path).await?;
    }
    
    info!("Downloaded file verified successfully");
    Ok(())
}

/// Extract filename from download URL
fn extract_filename_from_url(url: &str) -> Result<String> {
    url.split('/')
        .last()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Could not extract filename from URL: {}", url))
}

/// Calculate SHA256 checksum of a file
async fn calculate_sha256(file_path: &Path) -> Result<String> {
    debug!("Calculating SHA256 checksum for {}", file_path.display());
    
    // Use shasum command available on macOS
    let output = TokioCommand::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(file_path)
        .output()
        .await
        .context("Failed to execute shasum command")?;
        
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("shasum command failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let checksum = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("Invalid shasum output"))?
        .to_string();
        
    debug!("Calculated SHA256: {}", checksum);
    
    Ok(checksum)
}

/// Verify that a zip file can be extracted
async fn verify_zip_archive(file_path: &Path) -> Result<()> {
    debug!("Verifying zip archive: {}", file_path.display());
    
    // Use unzip command with -t flag to test archive
    let output = TokioCommand::new("unzip")
        .arg("-t")
        .arg(file_path)
        .output()
        .await
        .context("Failed to execute unzip command")?;
        
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("zip archive verification failed: {}", stderr));
    }
    
    debug!("Zip archive verification successful");
    Ok(())
}

/// Verify that a tar.gz file can be extracted
async fn verify_tar_archive(file_path: &Path) -> Result<()> {
    debug!("Verifying tar.gz archive: {}", file_path.display());
    
    // Use tar command with -t flag to test archive
    let output = TokioCommand::new("tar")
        .arg("-tzf")
        .arg(file_path)
        .output()
        .await
        .context("Failed to execute tar command")?;
        
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("tar.gz archive verification failed: {}", stderr));
    }
    
    debug!("Tar.gz archive verification successful");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_filename_from_url() {
        assert_eq!(
            extract_filename_from_url("https://github.com/a14a-org/node-controller-rust/releases/download/v1.0.0/node-controller-macos.zip").unwrap(),
            "node-controller-macos.zip"
        );
        
        assert_eq!(
            extract_filename_from_url("https://example.com/path/to/file.tar.gz").unwrap(),
            "file.tar.gz"
        );
    }
} 