// src/updater/github.rs
//
// GitHub API integration for the auto-update system
// Handles checking for updates and retrieving release information

use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use crate::updater::Version;
use log::{debug, error, info};

/// Information about a GitHub release
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GithubReleaseInfo {
    /// The release version as a string (e.g., "1.2.3")
    pub version: String,
    
    /// The tag name for this release
    pub tag_name: String,
    
    /// The full name of the release
    pub name: String,
    
    /// Release notes/description in markdown format
    pub body: String,
    
    /// Whether this is a pre-release
    pub prerelease: bool,
    
    /// When the release was published
    pub published_at: String,
    
    /// Direct download URL for the Mac binary asset
    pub download_url: String,
    
    /// File size in bytes
    pub size: u64,
    
    /// SHA256 checksum for verification
    pub sha256: Option<String>,
}

/// Check for updates from GitHub releases
pub async fn check_for_updates(
    repository: &str,
    tag_prefix: &str,
    current_version: &Version,
) -> Result<Option<GithubReleaseInfo>> {
    debug!("Checking for updates in repository {} with tag prefix {}", repository, tag_prefix);
    
    let github_releases = fetch_github_releases(repository).await
        .context("Failed to fetch GitHub releases")?;
    
    // Find the latest matching release
    let latest_release = find_latest_release(&github_releases, tag_prefix, current_version)?;
    
    Ok(latest_release)
}

/// Fetch releases from GitHub API
async fn fetch_github_releases(repository: &str) -> Result<Vec<serde_json::Value>> {
    let client = reqwest::Client::builder()
        .user_agent("node-controller-updater")
        .build()?;
    
    let url = format!("https://api.github.com/repos/{}/releases", repository);
    debug!("Fetching releases from GitHub API: {}", url);
    
    let response = client.get(&url)
        .send()
        .await
        .context("Failed to send request to GitHub API")?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        error!("GitHub API returned error status {}: {}", status, body);
        return Err(anyhow!("GitHub API returned error status {}", status));
    }
    
    let releases: Vec<serde_json::Value> = response.json().await
        .context("Failed to parse GitHub API response")?;
    
    Ok(releases)
}

/// Find the latest release that matches our criteria
fn find_latest_release(
    releases: &[serde_json::Value],
    tag_prefix: &str,
    current_version: &Version,
) -> Result<Option<GithubReleaseInfo>> {
    debug!("Looking for releases with tag prefix {}", tag_prefix);
    
    let mut latest_release: Option<GithubReleaseInfo> = None;
    let mut latest_version: Option<Version> = None;
    
    // Filter releases and find the latest
    for release in releases {
        // Extract release information
        let tag_name = release["tag_name"].as_str()
            .ok_or_else(|| anyhow!("Release missing tag_name"))?;
        
        // Check if this release matches our tag prefix
        if !tag_name.starts_with(tag_prefix) {
            debug!("Skipping release {}: doesn't match prefix", tag_name);
            continue;
        }
        
        // Extract version from tag name
        let version_str = extract_version_from_tag(tag_name, tag_prefix)?;
        let version = match Version::from_str(&version_str) {
            Ok(v) => v,
            Err(e) => {
                debug!("Skipping release {}: invalid version format: {}", tag_name, e);
                continue;
            }
        };
        
        // Skip versions that are not newer than current
        if version <= *current_version {
            debug!("Skipping release {}: not newer than current version {}", version, current_version);
            continue;
        }
        
        // Check if we found a pre-release
        let is_prerelease = release["prerelease"].as_bool().unwrap_or(false);
        
        // Skip pre-releases unless tag explicitly looks for them
        if is_prerelease && !tag_prefix.contains("beta") && !tag_prefix.contains("alpha") {
            debug!("Skipping pre-release {}: not looking for pre-releases", tag_name);
            continue;
        }
        
        // Find the download URL for macOS binary asset
        let assets = release["assets"].as_array()
            .ok_or_else(|| anyhow!("Release missing assets"))?;
        
        let mac_asset = find_mac_asset(assets)?;
        if mac_asset.is_none() {
            debug!("Skipping release {}: no macOS asset found", tag_name);
            continue;
        }
        
        let (download_url, size) = mac_asset.unwrap();
        
        // If we found a newer version, update our "latest"
        if latest_version.is_none() || version > *latest_version.as_ref().unwrap() {
            debug!("Found newer version: {}", version);
            
            // Extract other release information
            let name = release["name"].as_str()
                .unwrap_or("Unnamed Release")
                .to_string();
                
            let body = release["body"].as_str()
                .unwrap_or("")
                .to_string();
                
            let published_at = release["published_at"].as_str()
                .unwrap_or("")
                .to_string();
                
            // Look for SHA256 checksum in release notes
            let sha256 = extract_sha256_from_body(&body);
            
            // Create release info
            let release_info = GithubReleaseInfo {
                version: version_str,
                tag_name: tag_name.to_string(),
                name,
                body,
                prerelease: is_prerelease,
                published_at,
                download_url,
                size,
                sha256,
            };
            
            latest_release = Some(release_info);
            latest_version = Some(version);
        }
    }
    
    if let Some(release) = &latest_release {
        info!("Found update: {} ({})", release.version, release.tag_name);
    } else {
        debug!("No newer version found");
    }
    
    Ok(latest_release)
}

/// Extract version string from tag name
fn extract_version_from_tag(tag: &str, prefix: &str) -> Result<String> {
    // Handle various tag formats
    if tag.starts_with(prefix) {
        let remaining = tag[prefix.len()..].trim_start_matches('-').trim_start_matches('.');
        if remaining.is_empty() {
            return Err(anyhow!("Tag doesn't contain version information"));
        }
        return Ok(remaining.to_string());
    }
    
    // Try simple extraction for tags like "v1.2.3"
    if tag.starts_with('v') && tag.len() > 1 {
        return Ok(tag[1..].to_string());
    }
    
    // Tag format didn't match expected patterns
    Err(anyhow!("Could not extract version from tag {}", tag))
}

/// Find the macOS asset in the release assets
fn find_mac_asset(assets: &[serde_json::Value]) -> Result<Option<(String, u64)>> {
    for asset in assets {
        let name = asset["name"].as_str()
            .ok_or_else(|| anyhow!("Asset missing name"))?;
            
        // Check for macOS binary asset
        if (name.contains("macos") || name.contains("darwin") || 
            name.contains("mac") || name.contains("apple")) && 
           (name.ends_with(".zip") || name.ends_with(".tar.gz") || name.contains(".app."))
        {
            let download_url = asset["browser_download_url"].as_str()
                .ok_or_else(|| anyhow!("Asset missing download URL"))?
                .to_string();
                
            let size = asset["size"].as_u64()
                .ok_or_else(|| anyhow!("Asset missing size"))?;
                
            return Ok(Some((download_url, size)));
        }
    }
    
    Ok(None)
}

/// Extract SHA256 checksum from release notes
fn extract_sha256_from_body(body: &str) -> Option<String> {
    // Look for common formats of SHA256 checksums in release notes
    for line in body.lines() {
        // Look for SHA256 keyword followed by a hash
        if line.to_lowercase().contains("sha256") || line.to_lowercase().contains("checksum") {
            let hash_pattern = line.split_whitespace()
                .find(|word| {
                    word.len() == 64 && word.chars().all(|c| c.is_ascii_hexdigit())
                });
                
            if let Some(hash) = hash_pattern {
                return Some(hash.to_string());
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    #[test]
    fn test_extract_version_from_tag() {
        // Test with standard prefix format
        assert_eq!(
            extract_version_from_tag("stable-1.2.3", "stable").unwrap(),
            "1.2.3"
        );
        
        // Test with v prefix
        assert_eq!(
            extract_version_from_tag("v1.2.3", "v").unwrap(),
            "1.2.3"
        );
        
        // Test with v prefix but no prefix expected
        assert_eq!(
            extract_version_from_tag("v1.2.3", "").unwrap(),
            "1.2.3"
        );
        
        // Test with complex tag
        assert_eq!(
            extract_version_from_tag("beta-1.2.3-rc.1", "beta").unwrap(),
            "1.2.3-rc.1"
        );
    }
    
    #[test]
    fn test_extract_sha256_from_body() {
        // Test with SHA256 label
        let body = "Release notes\n\nSHA256: 1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d\nMore text";
        assert_eq!(
            extract_sha256_from_body(body),
            Some("1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d".to_string())
        );
        
        // Test with checksum label
        let body = "Release notes\n\nChecksum: 1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d\nMore text";
        assert_eq!(
            extract_sha256_from_body(body),
            Some("1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7a8b9c0d".to_string())
        );
        
        // Test with no checksum
        let body = "Release notes\n\nNo checksum here\nMore text";
        assert_eq!(extract_sha256_from_body(body), None);
    }
} 