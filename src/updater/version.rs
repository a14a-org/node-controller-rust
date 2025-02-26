// src/updater/version.rs
//
// Version handling for the auto-update system
// Implements semantic versioning parsing and comparison

use std::fmt;
use std::str::FromStr;
use anyhow::{Result, anyhow};
use std::cmp::Ordering;

/// A semantic version (SemVer) representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// Major version number (incompatible API changes)
    pub major: u32,
    
    /// Minor version number (backwards-compatible functionality)
    pub minor: u32,
    
    /// Patch version number (backwards-compatible bug fixes)
    pub patch: u32,
    
    /// Pre-release identifier (e.g., "beta.1")
    pub pre_release: Option<String>,
    
    /// Build metadata (e.g., "20230101")
    pub build: Option<String>,
}

impl Version {
    /// Create a new version with the given components
    pub fn new(
        major: u32,
        minor: u32,
        patch: u32,
        pre_release: Option<String>,
        build: Option<String>,
    ) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release,
            build,
        }
    }
    
    /// Check if this version is a pre-release
    pub fn is_pre_release(&self) -> bool {
        self.pre_release.is_some()
    }
    
    /// Check if this version has build metadata
    pub fn has_build_metadata(&self) -> bool {
        self.build.is_some()
    }
    
    /// Extract the version from Cargo.toml file
    pub fn from_cargo_toml() -> Result<Self> {
        let cargo_toml = include_str!("../../Cargo.toml");
        
        // Extract version string using a simple parsing approach
        for line in cargo_toml.lines() {
            let line = line.trim();
            if line.starts_with("version") {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let version_str = parts[1].trim().trim_matches('"');
                    return Version::from_str(version_str);
                }
            }
        }
        
        Err(anyhow!("Could not find version in Cargo.toml"))
    }
}

impl FromStr for Version {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split into version, pre-release, and build parts
        let mut parts = s.split('+');
        let version_and_pre = parts.next().ok_or_else(|| anyhow!("Empty version string"))?;
        let build = parts.next().map(|s| s.to_string());
        
        // Handle too many + characters
        if parts.next().is_some() {
            return Err(anyhow!("Invalid version format: too many '+' characters"));
        }
        
        // Split version and pre-release parts
        let mut parts = version_and_pre.split('-');
        let version = parts.next().ok_or_else(|| anyhow!("Empty version string"))?;
        let pre_release = parts.next().map(|s| s.to_string());
        
        // Handle too many - characters
        if parts.next().is_some() {
            return Err(anyhow!("Invalid version format: too many '-' characters"));
        }
        
        // Parse the version numbers
        let version_parts: Vec<&str> = version.split('.').collect();
        if version_parts.len() != 3 {
            return Err(anyhow!(
                "Invalid version format: expected 3 version numbers, got {}",
                version_parts.len()
            ));
        }
        
        let major = version_parts[0].parse::<u32>()
            .map_err(|_| anyhow!("Invalid major version number"))?;
        
        let minor = version_parts[1].parse::<u32>()
            .map_err(|_| anyhow!("Invalid minor version number"))?;
        
        let patch = version_parts[2].parse::<u32>()
            .map_err(|_| anyhow!("Invalid patch version number"))?;
        
        Ok(Version {
            major,
            minor,
            patch,
            pre_release,
            build,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        
        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }
        
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        
        Ok(())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare major.minor.patch numbers
        match self.major.cmp(&other.major) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {},
            ordering => return ordering,
        }
        
        // Compare pre-release identifiers (according to SemVer spec)
        match (&self.pre_release, &other.pre_release) {
            (None, Some(_)) => return Ordering::Greater, // Release > pre-release
            (Some(_), None) => return Ordering::Less,    // Pre-release < release
            (None, None) => {},                          // Both are releases
            (Some(a), Some(b)) => {                      // Compare pre-releases
                return a.cmp(b);
                // In a more complete implementation, we would split by .
                // and compare each identifier numerically if it's a number
            }
        }
        
        // Build metadata does not affect precedence
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_parsing() {
        let v = Version::from_str("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, None);
        assert_eq!(v.build, None);
        
        let v = Version::from_str("1.2.3-beta.1").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, Some("beta.1".to_string()));
        assert_eq!(v.build, None);
        
        let v = Version::from_str("1.2.3+20230101").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, None);
        assert_eq!(v.build, Some("20230101".to_string()));
        
        let v = Version::from_str("1.2.3-alpha.1+20230101").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, Some("alpha.1".to_string()));
        assert_eq!(v.build, Some("20230101".to_string()));
    }
    
    #[test]
    fn test_version_comparison() {
        // Major version comparison
        assert!(Version::from_str("2.0.0").unwrap() > Version::from_str("1.0.0").unwrap());
        
        // Minor version comparison
        assert!(Version::from_str("1.2.0").unwrap() > Version::from_str("1.1.0").unwrap());
        
        // Patch version comparison
        assert!(Version::from_str("1.0.2").unwrap() > Version::from_str("1.0.1").unwrap());
        
        // Pre-release vs release
        assert!(Version::from_str("1.0.0").unwrap() > Version::from_str("1.0.0-beta").unwrap());
        
        // Same version
        assert!(Version::from_str("1.0.0").unwrap() == Version::from_str("1.0.0").unwrap());
        
        // Build metadata doesn't affect comparison
        assert!(Version::from_str("1.0.0+build.1").unwrap() == Version::from_str("1.0.0+build.2").unwrap());
    }
    
    #[test]
    fn test_version_display() {
        let v = Version::new(1, 2, 3, None, None);
        assert_eq!(v.to_string(), "1.2.3");
        
        let v = Version::new(1, 2, 3, Some("beta.1".to_string()), None);
        assert_eq!(v.to_string(), "1.2.3-beta.1");
        
        let v = Version::new(1, 2, 3, None, Some("20230101".to_string()));
        assert_eq!(v.to_string(), "1.2.3+20230101");
        
        let v = Version::new(1, 2, 3, Some("alpha.1".to_string()), Some("20230101".to_string()));
        assert_eq!(v.to_string(), "1.2.3-alpha.1+20230101");
    }
} 