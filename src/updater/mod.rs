// src/updater/mod.rs
//
// Auto-update system for Node Controller
// This module manages checking for updates, downloading, verifying,
// backing up the current version, and applying updates safely.

mod github;
mod download;
mod backup;
mod health;
mod version;

pub use self::github::GithubReleaseInfo;
pub use self::version::Version;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use std::time::Duration;
use log::{info, error, debug};
use anyhow::{Result, Context};
use dirs;

/// Configuration for the update system
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// How often to check for updates (in minutes)
    pub check_interval_mins: u64,
    
    /// Which update channel to use (stable, beta, etc.)
    pub channel: UpdateChannel,
    
    /// Whether to apply updates automatically or just notify
    pub auto_update: bool,
    
    /// Repository owner/name on GitHub
    pub repository: String,
    
    /// Directory to store backups and downloaded updates
    pub update_dir: PathBuf,
    
    /// Maximum number of backups to keep
    pub max_backups: usize,
    
    /// Commands to run after successful update
    pub post_update_commands: Vec<String>,
    
    /// Timeout for health checks after an update
    pub health_check_timeout: Duration,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        // Use the user's Application Support directory by default
        let default_update_dir = dirs::home_dir()
            .map(|home| home.join("Library/Application Support/NodeController/updates"))
            .unwrap_or_else(|| PathBuf::from("./temp-updates"));
            
        Self {
            check_interval_mins: 60, // Check every hour by default
            channel: UpdateChannel::Stable,
            auto_update: false,      // Default to notify-only for safety
            repository: "a14a-org/node-controller-rust".to_string(),
            update_dir: default_update_dir,
            max_backups: 3,
            post_update_commands: vec![],
            health_check_timeout: Duration::from_secs(30),
        }
    }
}

/// Update channels that can be selected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateChannel {
    Stable,
    Beta,
    Nightly,
    Custom(String),
}

impl UpdateChannel {
    /// Convert the channel to a tag prefix for GitHub releases
    pub fn as_tag_prefix(&self) -> String {
        match self {
            Self::Stable => "stable".to_string(),
            Self::Beta => "beta".to_string(),
            Self::Nightly => "nightly".to_string(),
            Self::Custom(tag) => tag.clone(),
        }
    }
}

/// Status of the update process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpdateAvailable(GithubReleaseInfo),
    Downloading { version: String, progress: u8 },
    Verifying { version: String },
    BackingUp { version: String },
    Installing { version: String },
    VerifyingInstallation { version: String },
    UpdateSuccess { version: String, timestamp: chrono::DateTime<chrono::Utc> },
    UpdateFailed { version: String, error: String },
    RollingBack { version: String, reason: String },
    NoUpdateAvailable,
    Error(String),
}

/// The Update Manager handles the update workflow
pub struct UpdateManager {
    config: UpdateConfig,
    current_version: Version,
    status: Arc<Mutex<UpdateStatus>>,
    update_tx: mpsc::Sender<UpdateCommand>,
    update_rx: Option<mpsc::Receiver<UpdateCommand>>,
    /// Health check timeout duration
    /// Currently unused but part of the configuration
    #[allow(dead_code)]
    health_check_timeout: Duration,
}

/// Commands that can be sent to the update manager
#[derive(Debug)]
enum UpdateCommand {
    CheckForUpdates,
    ApplyUpdate(GithubReleaseInfo),
    CancelUpdate,
    Shutdown,
}

impl UpdateManager {
    /// Create a new update manager with the specified configuration
    pub fn new(config: UpdateConfig, current_version: Version) -> Self {
        let (tx, rx) = mpsc::channel(10);
        
        Self {
            config,
            current_version,
            status: Arc::new(Mutex::new(UpdateStatus::Idle)),
            update_tx: tx,
            update_rx: Some(rx),
            health_check_timeout: Duration::from_secs(30),
        }
    }
    
    /// Start the update manager background task
    pub async fn start(&mut self) -> Result<()> {
        let rx = self.update_rx.take()
            .context("UpdateManager has already been started")?;
            
        let status = self.status.clone();
        let config = self.config.clone();
        let current_version = self.current_version.clone();
        let tx = self.update_tx.clone();
        
        // Spawn the background update task
        tokio::spawn(async move {
            Self::update_loop(status, config, current_version, rx, tx).await;
        });
        
        // Trigger initial update check
        self.check_for_updates().await?;
        
        Ok(())
    }
    
    /// The main update loop that handles update commands
    async fn update_loop(
        status: Arc<Mutex<UpdateStatus>>,
        config: UpdateConfig,
        current_version: Version,
        mut rx: mpsc::Receiver<UpdateCommand>,
        _tx: mpsc::Sender<UpdateCommand>,
    ) {
        let mut update_interval = tokio::time::interval(
            Duration::from_secs(config.check_interval_mins * 60)
        );
        
        loop {
            tokio::select! {
                // Handle scheduled update checks
                _ = update_interval.tick() => {
                    debug!("Scheduled update check triggered");
                    if let Err(e) = Self::check_updates(&status, &config, &current_version).await {
                        error!("Scheduled update check failed: {}", e);
                        let mut s = status.lock().await;
                        *s = UpdateStatus::Error(format!("Update check failed: {}", e));
                    }
                }
                
                // Handle commands
                Some(cmd) = rx.recv() => {
                    match cmd {
                        UpdateCommand::CheckForUpdates => {
                            debug!("Manual update check triggered");
                            if let Err(e) = Self::check_updates(&status, &config, &current_version).await {
                                error!("Manual update check failed: {}", e);
                                let mut s = status.lock().await;
                                *s = UpdateStatus::Error(format!("Update check failed: {}", e));
                            }
                        }
                        
                        UpdateCommand::ApplyUpdate(release) => {
                            info!("Applying update to version {}", release.version);
                            let version_str = release.version.clone();
                            if let Err(e) = Self::apply_update(&status, &config, release).await {
                                error!("Update failed: {}", e);
                                let mut s = status.lock().await;
                                *s = UpdateStatus::UpdateFailed {
                                    version: version_str,
                                    error: e.to_string(),
                                };
                            }
                        }
                        
                        UpdateCommand::CancelUpdate => {
                            info!("Update cancelled by user");
                            let mut s = status.lock().await;
                            *s = UpdateStatus::Idle;
                        }
                        
                        UpdateCommand::Shutdown => {
                            info!("Update manager shutting down");
                            break;
                        }
                    }
                }
            }
        }
    }
    
    /// Check for available updates
    async fn check_updates(
        status: &Arc<Mutex<UpdateStatus>>,
        config: &UpdateConfig,
        current_version: &Version,
    ) -> Result<()> {
        {
            let mut s = status.lock().await;
            *s = UpdateStatus::Checking;
        }
        
        let release = github::check_for_updates(
            &config.repository, 
            &config.channel.as_tag_prefix(),
            current_version
        ).await?;
        
        let mut s = status.lock().await;
        if let Some(release) = release {
            info!("Update available: {} -> {}", current_version, release.version);
            *s = UpdateStatus::UpdateAvailable(release.clone());
            
            // Auto-apply the update if auto_update is enabled
            if config.auto_update {
                info!("Auto-update is enabled, applying update to version {}", release.version);
                // Drop the mutex lock before applying update
                drop(s);
                if let Err(e) = Self::apply_update(status, config, release).await {
                    error!("Automatic update failed: {}", e);
                }
            }
        } else {
            debug!("No updates available. Current version: {}", current_version);
            *s = UpdateStatus::NoUpdateAvailable;
        }
        
        Ok(())
    }
    
    /// Apply an update
    async fn apply_update(
        status: &Arc<Mutex<UpdateStatus>>,
        config: &UpdateConfig,
        release: GithubReleaseInfo,
    ) -> Result<()> {
        // 1. Download update
        {
            let mut s = status.lock().await;
            *s = UpdateStatus::Downloading {
                version: release.version.to_string(),
                progress: 0,
            };
        }
        
        let download_path = download::download_release(
            &release,
            &config.update_dir
        ).await?;
        
        // 2. Verify download
        {
            let mut s = status.lock().await;
            *s = UpdateStatus::Verifying {
                version: release.version.to_string(),
            };
        }
        
        download::verify_release(&download_path, &release).await?;
        
        // 3. Create backup
        {
            let mut s = status.lock().await;
            *s = UpdateStatus::BackingUp {
                version: release.version.to_string(),
            };
        }
        
        let backup_path = backup::create_backup(&config.update_dir).await?;
        
        // 4. Install update
        {
            let mut s = status.lock().await;
            *s = UpdateStatus::Installing {
                version: release.version.to_string(),
            };
        }
        
        backup::install_update(&download_path, config).await?;
        
        // 5. Verify installation
        {
            let mut s = status.lock().await;
            *s = UpdateStatus::VerifyingInstallation {
                version: release.version.to_string(),
            };
        }
        
        if let Err(e) = health::verify_installation(config.health_check_timeout).await {
            error!("Installation verification failed: {}", e);
            
            // Rollback to previous version
            {
                let mut s = status.lock().await;
                *s = UpdateStatus::RollingBack {
                    version: release.version.to_string(),
                    reason: e.to_string(),
                };
            }
            
            backup::restore_from_backup(&backup_path).await?;
            return Err(e.into());
        }
        
        // 6. Cleanup old backups
        backup::cleanup_old_backups(&config.update_dir, config.max_backups).await?;
        
        // 7. Update success
        {
            let mut s = status.lock().await;
            *s = UpdateStatus::UpdateSuccess {
                version: release.version.to_string(),
                timestamp: chrono::Utc::now(),
            };
        }
        
        info!("Successfully updated to version {}", release.version);
        Ok(())
    }
    
    /// Check for updates manually
    pub async fn check_for_updates(&self) -> Result<()> {
        self.update_tx.send(UpdateCommand::CheckForUpdates).await
            .context("Failed to send update check command")?;
        Ok(())
    }
    
    /// Gets the current update status
    /// This is currently unused but part of the public API
    #[allow(dead_code)]
    pub async fn status(&self) -> UpdateStatus {
        self.status.lock().await.clone()
    }
    
    /// Manually triggers an update process with the provided release info
    /// This is currently unused but part of the public API
    #[allow(dead_code)]
    pub async fn trigger_update(&self, release: GithubReleaseInfo) -> Result<()> {
        self.update_tx.send(UpdateCommand::ApplyUpdate(release)).await
            .context("Failed to send apply update command")?;
        Ok(())
    }
    
    /// Cancels an in-progress update
    /// This is currently unused but part of the public API
    #[allow(dead_code)]
    pub async fn cancel_update(&self) -> Result<()> {
        self.update_tx.send(UpdateCommand::CancelUpdate).await
            .context("Failed to send cancel update command")?;
        Ok(())
    }
    
    /// Gracefully shuts down the update manager
    /// This is currently unused but part of the public API
    #[allow(dead_code)]
    pub async fn shutdown(&self) -> Result<()> {
        self.update_tx.send(UpdateCommand::Shutdown).await
            .context("Failed to send shutdown command")?;
        Ok(())
    }
    
    /// Sets the timeout duration for health checks
    /// This is currently unused but part of the public API
    #[allow(dead_code)]
    pub fn set_health_check_timeout(&mut self, timeout: Duration) {
        self.health_check_timeout = timeout;
    }
} 