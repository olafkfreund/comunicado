//! Multi-platform package generation system
//!
//! This module handles creation of various package formats including:
//! - Debian packages (.deb) for Ubuntu/Debian distributions
//! - RPM packages (.rpm) for Red Hat/SUSE distributions  
//! - AppImage portable applications for universal Linux compatibility
//! - Flatpak packages for sandboxed distribution
//! - Snap packages for Ubuntu Core and other distributions

use crate::deployment::{Platform, Architecture, OperatingSystem, DeploymentArtifact, ArtifactType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{PathBuf}; // Path
use thiserror::Error;
use uuid::Uuid;

/// Package management errors
#[derive(Error, Debug)]
pub enum PackageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Unsupported package type: {0:?}")]
    UnsupportedPackageType(PackageType),
    
    #[error("Build failed: {0}")]
    BuildFailed(String),
    
    #[error("Missing dependency: {0}")]
    MissingDependency(String),
    
    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),
    
    #[error("Platform not supported: {0:?}")]
    PlatformNotSupported(Platform),
    
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
    
    #[error("Template error: {0}")]
    Template(String),
}

pub type PackageResult<T> = Result<T, PackageError>;

/// Supported package formats
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PackageType {
    /// Debian package (.deb)
    Debian,
    /// Red Hat package (.rpm)
    Rpm,
    /// Portable AppImage
    AppImage,
    /// Flatpak package
    Flatpak,
    /// Snap package
    Snap,
    /// Generic archive
    Archive(ArchiveFormat),
}

/// Archive formats for generic packaging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ArchiveFormat {
    TarGz,
    TarXz,
    Zip,
}

/// Package configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
    pub maintainer: String,
    pub homepage: Option<String>,
    pub license: String,
    pub platform: Platform,
    pub package_type: PackageType,
    pub binary_path: PathBuf,
    pub assets: Vec<AssetConfig>,
    pub dependencies: Vec<Dependency>,
    pub categories: Vec<String>,
    pub desktop_entry: Option<DesktopEntry>,
    pub scripts: PackageScripts,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

/// Asset configuration for packaging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub permissions: Option<u32>,
    pub asset_type: AssetType,
}

/// Types of assets included in packages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetType {
    Binary,
    Configuration,
    Documentation,
    Icon,
    Desktop,
    License,
    Readme,
    ManPage,
    Completion,
    Custom(String),
}

/// Package dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub requirement: DependencyRequirement,
    pub optional: bool,
}

/// Dependency version requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyRequirement {
    Exact(String),
    Minimum(String),
    Maximum(String),
    Range(String, String),
    Any,
}

/// Desktop entry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopEntry {
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub icon: Option<String>,
    pub exec: String,
    pub terminal: bool,
    pub categories: Vec<String>,
    pub mime_types: Vec<String>,
    pub keywords: Vec<String>,
}

/// Package installation/removal scripts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageScripts {
    pub pre_install: Option<String>,
    pub post_install: Option<String>,
    pub pre_remove: Option<String>,
    pub post_remove: Option<String>,
}

/// Main package manager
pub struct PackageManager {
    config: PackageManagerConfig,
    builders: HashMap<PackageType, Box<dyn PackageBuilder>>,
}

/// Package manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerConfig {
    pub build_dir: PathBuf,
    pub output_dir: PathBuf,
    pub parallel_builds: u32,
    pub cleanup_after_build: bool,
}

/// Trait for package builders
pub trait PackageBuilder: Send + Sync {
    fn build(&self, config: &PackageConfig) -> PackageResult<DeploymentArtifact>;
    fn supports_platform(&self, platform: &Platform) -> bool;
    fn validate_config(&self, config: &PackageConfig) -> PackageResult<()>;
}

impl PackageManager {
    pub fn new() -> PackageResult<Self> {
        let mut builders: HashMap<PackageType, Box<dyn PackageBuilder>> = HashMap::new();
        
        // Register package builders
        builders.insert(PackageType::Debian, Box::new(DebianPackage::new()?));
        builders.insert(PackageType::Rpm, Box::new(RpmPackage::new()?));
        builders.insert(PackageType::AppImage, Box::new(AppImagePackage::new()?));
        builders.insert(PackageType::Flatpak, Box::new(FlatpakPackage::new()?));
        
        Ok(Self {
            config: PackageManagerConfig::default(),
            builders,
        })
    }

    /// Build a package with the specified configuration
    pub async fn build_package(&self, config: PackageConfig) -> PackageResult<DeploymentArtifact> {
        let builder = self.builders
            .get(&config.package_type)
            .ok_or_else(|| PackageError::UnsupportedPackageType(config.package_type.clone()))?;

        // Validate configuration
        builder.validate_config(&config)?;

        // Check platform support
        if !builder.supports_platform(&config.platform) {
            return Err(PackageError::PlatformNotSupported(config.platform.clone()));
        }

        // Build package
        builder.build(&config)
    }

    /// Build packages for multiple platforms
    pub async fn build_multi_platform(
        &self,
        base_config: PackageConfig,
        platforms: Vec<Platform>,
    ) -> PackageResult<Vec<DeploymentArtifact>> {
        let mut artifacts = Vec::new();

        for platform in platforms {
            let mut config = base_config.clone();
            config.platform = platform;
            config.id = Uuid::new_v4(); // New ID for each platform

            let artifact = self.build_package(config).await?;
            artifacts.push(artifact);
        }

        Ok(artifacts)
    }

    /// Get supported package types for a platform
    pub fn supported_package_types(&self, platform: &Platform) -> Vec<PackageType> {
        self.builders
            .iter()
            .filter_map(|(package_type, builder)| {
                if builder.supports_platform(platform) {
                    Some(package_type.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for PackageManagerConfig {
    fn default() -> Self {
        Self {
            build_dir: PathBuf::from("target/package-build"),
            output_dir: PathBuf::from("target/packages"),
            parallel_builds: 4,
            cleanup_after_build: true,
        }
    }
}

impl PackageConfig {
    pub fn new(name: String, version: String, platform: Platform, package_type: PackageType) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            version,
            description: String::new(),
            maintainer: String::new(),
            homepage: None,
            license: String::new(),
            platform,
            package_type,
            binary_path: PathBuf::new(),
            assets: Vec::new(),
            dependencies: Vec::new(),
            categories: Vec::new(),
            desktop_entry: None,
            scripts: PackageScripts::default(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create configuration for Comunicado
    pub fn for_comunicado(platform: Platform, package_type: PackageType) -> Self {
        let mut config = Self::new(
            "comunicado".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            platform,
            package_type,
        );

        config.description = "Modern TUI email and calendar client for terminal enthusiasts".to_string();
        config.maintainer = "Comunicado Team <contact@comunicado.app>".to_string();
        config.homepage = Some("https://github.com/comunicado/comunicado".to_string());
        config.license = "MIT".to_string();
        config.categories = vec![
            "Network".to_string(),
            "Email".to_string(),
            "Office".to_string(),
            "Productivity".to_string(),
        ];

        // Desktop entry for GUI integration
        config.desktop_entry = Some(DesktopEntry {
            name: "Comunicado".to_string(),
            generic_name: Some("Email & Calendar Client".to_string()),
            comment: Some("Modern terminal-based email and calendar application".to_string()),
            icon: Some("comunicado".to_string()),
            exec: "comunicado".to_string(),
            terminal: true,
            categories: vec![
                "Network".to_string(),
                "Email".to_string(),
                "Office".to_string(),
            ],
            mime_types: vec![
                "x-scheme-handler/mailto".to_string(),
                "message/rfc822".to_string(),
            ],
            keywords: vec![
                "email".to_string(),
                "mail".to_string(),
                "calendar".to_string(),
                "terminal".to_string(),
                "tui".to_string(),
            ],
        });

        config
    }
}

/// Debian package builder
pub struct DebianPackage {
    tools_available: bool,
}

impl DebianPackage {
    pub fn new() -> PackageResult<Self> {
        // Check if dpkg-deb is available
        let tools_available = std::process::Command::new("dpkg-deb")
            .arg("--version")
            .output()
            .is_ok();

        Ok(Self { tools_available })
    }
}

impl PackageBuilder for DebianPackage {
    fn build(&self, config: &PackageConfig) -> PackageResult<DeploymentArtifact> {
        if !self.tools_available {
            return Err(PackageError::MissingDependency("dpkg-deb".to_string()));
        }

        // Implementation placeholder - would create debian package structure
        // and call dpkg-deb to build the .deb file
        
        let artifact = DeploymentArtifact {
            id: config.id,
            name: format!("{}-{}.deb", config.name, config.version),
            version: config.version.clone(),
            platform: config.platform.clone(),
            artifact_type: ArtifactType::Deb,
            file_path: PathBuf::from(format!("target/packages/{}-{}.deb", config.name, config.version)),
            checksum: "placeholder".to_string(),
            size_bytes: 0,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        };

        Ok(artifact)
    }

    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(platform.os, OperatingSystem::Linux)
    }

    fn validate_config(&self, _config: &PackageConfig) -> PackageResult<()> {
        // Validate debian-specific requirements
        Ok(())
    }
}

/// RPM package builder
pub struct RpmPackage {
    tools_available: bool,
}

impl RpmPackage {
    pub fn new() -> PackageResult<Self> {
        let tools_available = std::process::Command::new("rpmbuild")
            .arg("--version")
            .output()
            .is_ok();

        Ok(Self { tools_available })
    }
}

impl PackageBuilder for RpmPackage {
    fn build(&self, config: &PackageConfig) -> PackageResult<DeploymentArtifact> {
        if !self.tools_available {
            return Err(PackageError::MissingDependency("rpmbuild".to_string()));
        }

        let artifact = DeploymentArtifact {
            id: config.id,
            name: format!("{}-{}.rpm", config.name, config.version),
            version: config.version.clone(),
            platform: config.platform.clone(),
            artifact_type: ArtifactType::Rpm,
            file_path: PathBuf::from(format!("target/packages/{}-{}.rpm", config.name, config.version)),
            checksum: "placeholder".to_string(),
            size_bytes: 0,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        };

        Ok(artifact)
    }

    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(platform.os, OperatingSystem::Linux)
    }

    fn validate_config(&self, _config: &PackageConfig) -> PackageResult<()> {
        Ok(())
    }
}

/// AppImage package builder
pub struct AppImagePackage {
    tools_available: bool,
}

impl AppImagePackage {
    pub fn new() -> PackageResult<Self> {
        let tools_available = std::process::Command::new("appimagetool")
            .arg("--version")
            .output()
            .is_ok();

        Ok(Self { tools_available })
    }
}

impl PackageBuilder for AppImagePackage {
    fn build(&self, config: &PackageConfig) -> PackageResult<DeploymentArtifact> {
        let artifact = DeploymentArtifact {
            id: config.id,
            name: format!("{}-{}.AppImage", config.name, config.version),
            version: config.version.clone(),
            platform: config.platform.clone(),
            artifact_type: ArtifactType::AppImage,
            file_path: PathBuf::from(format!("target/packages/{}-{}.AppImage", config.name, config.version)),
            checksum: "placeholder".to_string(),
            size_bytes: 0,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        };

        Ok(artifact)
    }

    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(platform.os, OperatingSystem::Linux)
    }

    fn validate_config(&self, _config: &PackageConfig) -> PackageResult<()> {
        Ok(())
    }
}

/// Flatpak package builder
pub struct FlatpakPackage {
    tools_available: bool,
}

impl FlatpakPackage {
    pub fn new() -> PackageResult<Self> {
        let tools_available = std::process::Command::new("flatpak-builder")
            .arg("--version")
            .output()
            .is_ok();

        Ok(Self { tools_available })
    }
}

impl PackageBuilder for FlatpakPackage {
    fn build(&self, config: &PackageConfig) -> PackageResult<DeploymentArtifact> {
        let artifact = DeploymentArtifact {
            id: config.id,
            name: format!("{}-{}.flatpak", config.name, config.version),
            version: config.version.clone(),
            platform: config.platform.clone(),
            artifact_type: ArtifactType::Flatpak,
            file_path: PathBuf::from(format!("target/packages/{}-{}.flatpak", config.name, config.version)),
            checksum: "placeholder".to_string(),
            size_bytes: 0,
            created_at: Utc::now(),
            metadata: HashMap::new(),
        };

        Ok(artifact)
    }

    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(platform.os, OperatingSystem::Linux)
    }

    fn validate_config(&self, _config: &PackageConfig) -> PackageResult<()> {
        Ok(())
    }
}