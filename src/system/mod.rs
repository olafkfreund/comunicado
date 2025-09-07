//! System-level coordination and integration
//!
//! This module provides the highest level of system coordination,
//! integrating all major components and providing unified system management.

pub mod integration;
pub mod performance;

pub use integration::{
    PerformanceMetric, SystemConfig, SystemEvent, SystemHealth, SystemIntegrationService,
    SystemResult, SystemStatistics,
};
pub use performance::{
    ComponentPerformance, PerformanceAlert, PerformanceMonitor, PerformanceReport,
    PerformanceThresholds, SystemResourceUsage,
};
