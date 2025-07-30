//! System-level coordination and integration
//!
//! This module provides the highest level of system coordination,
//! integrating all major components and providing unified system management.

pub mod integration;
pub mod performance;

pub use integration::{
    SystemIntegrationService, SystemConfig, SystemResult, SystemEvent,
    SystemHealth, SystemStatistics, PerformanceMetric,
};
pub use performance::{
    PerformanceMonitor, PerformanceReport, SystemResourceUsage,
    ComponentPerformance, PerformanceAlert, PerformanceThresholds,
};