//! Production deployment and packaging system
//!
//! This module provides comprehensive deployment and packaging capabilities including:
//! - Multi-platform package generation (DEB, RPM, AppImage, Flatpak)
//! - Container deployment (Docker, Podman)
//! - Distribution-specific packaging (AUR, Nix, Homebrew)
//! - Automated CI/CD integration
//! - Version management and release automation
//! - Configuration management for different environments
//! - Health checks and monitoring setup
//! - Auto-update mechanisms

pub mod auto_update;
pub mod ci_cd;
pub mod configuration;
pub mod containers;
pub mod distributions;
pub mod health_checks;
pub mod packaging;
pub mod version_manager;

pub use auto_update::{
    AutoUpdateConfig, AutoUpdateError, AutoUpdateManager, AutoUpdateResult, UpdateChannel,
    UpdatePolicy, UpdateStrategy,
};
pub use ci_cd::{
    CiCdConfig, CiCdError, CiCdManager, CiCdProvider, CiCdResult, GitHubActions, GitLabCi,
    JenkinsConfig, PipelineStage,
};
pub use configuration::{
    ConfigManager, DeploymentConfig, DevelopmentConfig, Environment, ProductionConfig,
    StagingConfig,
};
pub use containers::{
    ContainerConfig, ContainerError, ContainerManager, ContainerResult, ContainerRuntime,
    DockerConfig, PodmanConfig,
};
pub use distributions::{
    AurPackage, DistributionConfig, DistributionError, DistributionManager, DistributionResult,
    HomebrewPackage, NixPackage, SnapPackage,
};
pub use health_checks::{
    AlertConfig, HealthCheck, HealthCheckConfig, HealthCheckError, HealthCheckManager,
    HealthCheckResult, MonitoringConfig,
};
pub use packaging::{
    AppImagePackage, DebianPackage, FlatpakPackage, PackageConfig, PackageError, PackageManager,
    PackageResult, PackageType, RpmPackage,
};
pub use version_manager::{
    ChangelogEntry, ReleaseType, SemanticVersion, VersionBump, VersionConfig, VersionError,
    VersionManager, VersionResult,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

// Manager imports are already handled by the existing pub use statements above

/// Deployment target environments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeploymentTarget {
    Production,
    Staging,
    Development,
    Testing,
    Custom(String),
}

/// Deployment strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeploymentStrategy {
    /// Replace all instances at once
    Recreate,
    /// Gradual replacement with zero downtime
    RollingUpdate,
    /// Deploy alongside current version, then switch
    BlueGreen,
    /// Route percentage of traffic to new version
    Canary { percentage: u8 },
    /// Custom deployment strategy
    Custom(String),
}

/// Platform architectures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum Architecture {
    X86_64,
    Aarch64,
    Armv7,
    I686,
    Riscv64,
    Custom(String),
}

impl Architecture {
    pub fn as_str(&self) -> &str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
            Architecture::Armv7 => "armv7",
            Architecture::I686 => "i686",
            Architecture::Riscv64 => "riscv64",
            Architecture::Custom(name) => name,
        }
    }

    pub fn rust_target(&self) -> &str {
        match self {
            Architecture::X86_64 => "x86_64-unknown-linux-gnu",
            Architecture::Aarch64 => "aarch64-unknown-linux-gnu",
            Architecture::Armv7 => "armv7-unknown-linux-gnueabihf",
            Architecture::I686 => "i686-unknown-linux-gnu",
            Architecture::Riscv64 => "riscv64gc-unknown-linux-gnu",
            Architecture::Custom(target) => target,
        }
    }
}

/// Operating systems
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum OperatingSystem {
    Linux,
    MacOS,
    Windows,
    FreeBSD,
    OpenBSD,
    NetBSD,
    Custom(String),
}

impl OperatingSystem {
    pub fn as_str(&self) -> &str {
        match self {
            OperatingSystem::Linux => "linux",
            OperatingSystem::MacOS => "macos",
            OperatingSystem::Windows => "windows",
            OperatingSystem::FreeBSD => "freebsd",
            OperatingSystem::OpenBSD => "openbsd",
            OperatingSystem::NetBSD => "netbsd",
            OperatingSystem::Custom(name) => name,
        }
    }
}

/// Platform specification combining OS and architecture
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub struct Platform {
    pub os: OperatingSystem,
    pub arch: Architecture,
}

impl Platform {
    pub fn new(os: OperatingSystem, arch: Architecture) -> Self {
        Self { os, arch }
    }

    pub fn triple(&self) -> String {
        format!("{}-{}", self.arch.as_str(), self.os.as_str())
    }

    /// Common platform presets
    pub fn linux_x86_64() -> Self {
        Self::new(OperatingSystem::Linux, Architecture::X86_64)
    }

    pub fn linux_aarch64() -> Self {
        Self::new(OperatingSystem::Linux, Architecture::Aarch64)
    }

    pub fn macos_x86_64() -> Self {
        Self::new(OperatingSystem::MacOS, Architecture::X86_64)
    }

    pub fn macos_aarch64() -> Self {
        Self::new(OperatingSystem::MacOS, Architecture::Aarch64)
    }

    pub fn windows_x86_64() -> Self {
        Self::new(OperatingSystem::Windows, Architecture::X86_64)
    }
}

/// Deployment artifact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentArtifact {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub platform: Platform,
    pub artifact_type: ArtifactType,
    pub file_path: PathBuf,
    pub checksum: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Types of deployment artifacts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArtifactType {
    /// Executable binary
    Binary,
    /// Debian package (.deb)
    Deb,
    /// RPM package (.rpm)  
    Rpm,
    /// AppImage portable application
    AppImage,
    /// Flatpak package
    Flatpak,
    /// Snap package
    Snap,
    /// Container image
    Container,
    /// Archive (tar.gz, zip)
    Archive,
    /// Source package
    Source,
    /// Custom artifact type
    Custom(String),
}

/// Deployment status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStatus {
    pub id: Uuid,
    pub target: DeploymentTarget,
    pub version: String,
    pub strategy: DeploymentStrategy,
    pub status: Status,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub artifacts: Vec<Uuid>,
    pub logs: Vec<String>,
    pub health_checks: Vec<HealthCheckStatus>,
}

/// Deployment status states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Status {
    Pending,
    InProgress { stage: String, progress: f32 },
    Completed,
    Failed { error: String },
    RolledBack,
    Cancelled,
}

/// Health check status for deployments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckStatus {
    pub check_name: String,
    pub status: HealthStatus,
    pub last_check: DateTime<Utc>,
    pub message: Option<String>,
    pub response_time_ms: Option<u64>,
}

/// Health check result status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Main deployment orchestrator
pub struct DeploymentOrchestrator {
    package_manager: PackageManager,
    container_manager: ContainerManager,
    distribution_manager: DistributionManager,
    ci_cd_manager: CiCdManager,
    version_manager: VersionManager,
    config_manager: ConfigManager,
    health_check_manager: HealthCheckManager,
    auto_update_manager: AutoUpdateManager,
    active_deployments: HashMap<Uuid, DeploymentStatus>,
}

impl DeploymentOrchestrator {
    pub fn new() -> DeploymentResult<Self> {
        Ok(Self {
            package_manager: PackageManager::new()?,
            container_manager: ContainerManager::new()?,
            distribution_manager: DistributionManager::new()?,
            ci_cd_manager: CiCdManager::new()?,
            version_manager: VersionManager::new()?,
            config_manager: ConfigManager::new()?,
            health_check_manager: HealthCheckManager::new()?,
            auto_update_manager: AutoUpdateManager::new()?,
            active_deployments: HashMap::new(),
        })
    }

    /// Deploy to a specific target environment
    pub async fn deploy(
        &mut self,
        target: DeploymentTarget,
        version: String,
        strategy: DeploymentStrategy,
        artifacts: Vec<DeploymentArtifact>,
    ) -> DeploymentResult<Uuid> {
        let deployment_id = Uuid::new_v4();

        let deployment_status = DeploymentStatus {
            id: deployment_id,
            target: target.clone(),
            version: version.clone(),
            strategy: strategy.clone(),
            status: Status::Pending,
            started_at: Utc::now(),
            completed_at: None,
            artifacts: artifacts.iter().map(|a| a.id).collect(),
            logs: Vec::new(),
            health_checks: Vec::new(),
        };

        self.active_deployments
            .insert(deployment_id, deployment_status);

        // Pre-deployment checks with CI/CD manager
        self.ci_cd_manager
            .validate_deployment_readiness(&artifacts)
            .await?;

        // Version management
        self.version_manager
            .prepare_version_rollback(&version)
            .await?;

        // Execute deployment strategy
        match strategy {
            DeploymentStrategy::Recreate => {
                self.execute_recreate_deployment(deployment_id, &artifacts)
                    .await?;
            }
            DeploymentStrategy::RollingUpdate => {
                self.execute_rolling_update(deployment_id, &artifacts)
                    .await?;
            }
            DeploymentStrategy::BlueGreen => {
                self.execute_blue_green_deployment(deployment_id, &artifacts)
                    .await?;
            }
            DeploymentStrategy::Canary { percentage } => {
                self.execute_canary_deployment(deployment_id, &artifacts, percentage)
                    .await?;
            }
            DeploymentStrategy::Custom(strategy_name) => {
                self.execute_custom_deployment(deployment_id, &artifacts, &strategy_name)
                    .await?;
            }
        }

        // Run health checks
        self.run_post_deployment_health_checks(deployment_id)
            .await?;

        // Post-deployment tasks
        self.distribution_manager
            .register_deployed_version(&version, &artifacts)
            .await?;
        self.auto_update_manager
            .schedule_update_checks(deployment_id)
            .await?;

        // CI/CD post-deployment notifications
        self.ci_cd_manager
            .notify_deployment_complete(deployment_id)
            .await?;

        Ok(deployment_id)
    }

    /// Get status of a deployment
    pub fn get_deployment_status(&self, deployment_id: Uuid) -> Option<&DeploymentStatus> {
        self.active_deployments.get(&deployment_id)
    }

    /// List all active deployments
    pub fn list_deployments(&self) -> Vec<&DeploymentStatus> {
        self.active_deployments.values().collect()
    }

    /// Private deployment strategy implementations
    async fn execute_recreate_deployment(
        &mut self,
        deployment_id: Uuid,
        artifacts: &[DeploymentArtifact],
    ) -> DeploymentResult<()> {
        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::InProgress {
                stage: "Deploying".to_string(),
                progress: 0.0,
            };
        }

        // Stop existing deployment
        self.container_manager.stop_all_containers().await?;

        // Deploy artifacts using package manager
        for artifact in artifacts {
            self.package_manager.install_artifact(artifact).await?;
        }

        // Start containers
        self.container_manager
            .start_deployment(deployment_id, artifacts)
            .await?;

        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::Completed;
            deployment.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    async fn execute_rolling_update(
        &mut self,
        deployment_id: Uuid,
        artifacts: &[DeploymentArtifact],
    ) -> DeploymentResult<()> {
        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::InProgress {
                stage: "Deploying".to_string(),
                progress: 0.0,
            };
        }

        // Install new version artifacts
        for artifact in artifacts {
            self.package_manager.install_artifact(artifact).await?;
        }

        // Rolling update with container manager
        self.container_manager
            .rolling_update(deployment_id, artifacts)
            .await?;

        // Run health checks after each update step
        self.health_check_manager
            .run_health_checks(deployment_id)
            .await?;

        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::Completed;
            deployment.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    async fn execute_blue_green_deployment(
        &mut self,
        deployment_id: Uuid,
        artifacts: &[DeploymentArtifact],
    ) -> DeploymentResult<()> {
        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::InProgress {
                stage: "Deploying".to_string(),
                progress: 0.0,
            };
        }

        // Deploy to green environment
        self.container_manager
            .deploy_to_green_environment(deployment_id, artifacts)
            .await?;

        // Run comprehensive health checks on green environment
        self.health_check_manager
            .run_health_checks(deployment_id)
            .await?;

        // Switch traffic from blue to green
        self.container_manager
            .switch_blue_green_traffic(deployment_id)
            .await?;

        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::Completed;
            deployment.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    async fn execute_canary_deployment(
        &mut self,
        deployment_id: Uuid,
        artifacts: &[DeploymentArtifact],
        percentage: u8,
    ) -> DeploymentResult<()> {
        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::InProgress {
                stage: "Deploying".to_string(),
                progress: 0.0,
            };
        }

        // Deploy canary version to percentage of traffic
        self.container_manager
            .deploy_canary(deployment_id, artifacts, percentage)
            .await?;

        // Monitor canary deployment
        self.health_check_manager
            .monitor_canary_deployment(deployment_id, percentage)
            .await?;

        // Gradually increase traffic if health checks pass
        for traffic_percentage in (percentage..100).step_by(10) {
            self.container_manager
                .update_canary_traffic(deployment_id, traffic_percentage)
                .await?;
            self.health_check_manager
                .run_health_checks(deployment_id)
                .await?;
        }

        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::Completed;
            deployment.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    async fn execute_custom_deployment(
        &mut self,
        deployment_id: Uuid,
        artifacts: &[DeploymentArtifact],
        strategy_name: &str,
    ) -> DeploymentResult<()> {
        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::InProgress {
                stage: "Deploying".to_string(),
                progress: 0.0,
            };
        }

        // Load and execute custom deployment strategy
        let custom_config = self
            .config_manager
            .load_custom_strategy(strategy_name)
            .await?;

        // Execute custom deployment steps
        for artifact in artifacts {
            self.package_manager
                .install_artifact_with_config(artifact, &custom_config)
                .await?;
        }

        self.container_manager
            .deploy_with_custom_strategy(deployment_id, artifacts, &custom_config)
            .await?;
        self.health_check_manager
            .run_custom_health_checks(deployment_id, &custom_config)
            .await?;

        // Update deployment status
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.status = Status::Completed;
            deployment.completed_at = Some(Utc::now());
        }

        Ok(())
    }

    async fn run_post_deployment_health_checks(
        &mut self,
        deployment_id: Uuid,
    ) -> DeploymentResult<()> {
        // Run comprehensive post-deployment health checks
        let health_results = self
            .health_check_manager
            .run_full_health_suite(deployment_id)
            .await?;

        // Update deployment with health check results
        if let Some(deployment) = self.active_deployments.get_mut(&deployment_id) {
            deployment.health_checks = health_results;

            // Set status based on health check results
            let all_healthy = deployment
                .health_checks
                .iter()
                .all(|check| check.status == HealthStatus::Healthy);
            if !all_healthy {
                deployment.status = Status::Failed {
                    error: "Health checks failed".to_string(),
                };
                return Err(DeploymentError::DeploymentFailed(
                    "Health checks failed".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for DeploymentOrchestrator {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// Common result and error types for the deployment system
pub type DeploymentResult<T> = Result<T, DeploymentError>;

/// Deployment system errors
#[derive(thiserror::Error, Debug)]
pub enum DeploymentError {
    #[error("Package error: {0}")]
    Package(#[from] PackageError),

    #[error("Container error: {0}")]
    Container(#[from] ContainerError),

    #[error("Distribution error: {0}")]
    Distribution(#[from] DistributionError),

    #[error("CI/CD error: {0}")]
    CiCd(#[from] CiCdError),

    #[error("Version error: {0}")]
    Version(#[from] VersionError),

    #[error("Configuration error: {0}")]
    Configuration(#[from] configuration::DeploymentConfigError),

    #[error("Health check error: {0}")]
    HealthCheck(#[from] HealthCheckError),

    #[error("Auto-update error: {0}")]
    AutoUpdate(#[from] AutoUpdateError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Deployment not found: {0}")]
    DeploymentNotFound(Uuid),

    #[error("Deployment failed: {0}")]
    DeploymentFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}
