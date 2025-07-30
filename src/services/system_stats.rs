use chrono::Local;
use std::fs;
use std::time::Duration;
use sysinfo::System;

use crate::ui::start_page::SystemStats;

/// System statistics service
pub struct SystemStatsService {
    system: System,
    last_update: Option<chrono::DateTime<chrono::Local>>,
    cached_stats: Option<SystemStats>,
    update_interval: Duration,
}

impl SystemStatsService {
    /// Create a new system stats service
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system,
            last_update: None,
            cached_stats: None,
            update_interval: Duration::from_secs(5), // Update every 5 seconds
        }
    }

    /// Get current system statistics
    pub fn get_stats(&mut self) -> SystemStats {
        // Check if we should refresh
        let should_refresh = if let Some(last_update) = self.last_update {
            Local::now().signed_duration_since(last_update)
                >= chrono::Duration::from_std(self.update_interval).unwrap_or_default()
        } else {
            true
        };

        if should_refresh {
            self.refresh_stats();
        }

        self.cached_stats.clone().unwrap_or_else(|| {
            // Fallback if refresh failed
            SystemStats {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                disk_usage: 0.0,
                network_rx: 0.0,
                network_tx: 0.0,
                uptime: chrono::Duration::zero(),
            }
        })
    }

    /// Refresh system statistics
    fn refresh_stats(&mut self) {
        // Refresh system information
        self.system.refresh_cpu();
        self.system.refresh_memory();

        // Calculate CPU usage (average across all cores)
        let cpu_usage = self
            .system
            .cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .sum::<f32>()
            / self.system.cpus().len() as f32;

        // Calculate memory usage
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let memory_usage = if total_memory > 0 {
            (used_memory as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        // Calculate disk usage (mock data for now - sysinfo API changed)
        let disk_usage = 45.0;

        // Calculate network usage (simplified - mock data for now)
        let (network_rx, network_tx) = (0u64, 0u64);

        // Convert bytes to KB/s (approximate)
        let network_rx_kbps = network_rx as f32 / 1024.0;
        let network_tx_kbps = network_tx as f32 / 1024.0;

        // Get uptime
        let uptime = self.get_uptime();

        let stats = SystemStats {
            cpu_usage,
            memory_usage,
            disk_usage,
            network_rx: network_rx_kbps,
            network_tx: network_tx_kbps,
            uptime,
        };

        self.cached_stats = Some(stats);
        self.last_update = Some(Local::now());
    }

    /// Get system uptime
    fn get_uptime(&self) -> chrono::Duration {
        // Try to read uptime from /proc/uptime (Linux)
        if let Ok(uptime_str) = fs::read_to_string("/proc/uptime") {
            if let Some(uptime_seconds_str) = uptime_str.split_whitespace().next() {
                if let Ok(uptime_seconds) = uptime_seconds_str.parse::<f64>() {
                    return chrono::Duration::seconds(uptime_seconds as i64);
                }
            }
        }

        // Fallback: use sysinfo boot_time if available
        let boot_time = System::boot_time();
        let current_time = chrono::Utc::now().timestamp() as u64;

        if current_time > boot_time {
            chrono::Duration::seconds((current_time - boot_time) as i64)
        } else {
            chrono::Duration::zero()
        }
    }

    /// Get detailed CPU information
    pub fn get_cpu_info(&mut self) -> Vec<(String, f32)> {
        self.system.refresh_cpu();

        self.system
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| (format!("CPU {}", i), cpu.cpu_usage()))
            .collect()
    }

    /// Get memory information in bytes
    pub fn get_memory_info(&mut self) -> (u64, u64, u64) {
        self.system.refresh_memory();
        (
            self.system.total_memory(),
            self.system.used_memory(),
            self.system.available_memory(),
        )
    }

    /// Get disk information
    pub fn get_disk_info(&mut self) -> Vec<(String, u64, u64, f32)> {
        // Mock disk data for now - sysinfo API changed
        vec![("/".to_string(), 100_000_000_000, 55_000_000_000, 45.0)]
    }

    /// Get network interfaces information
    pub fn get_network_info(&mut self) -> Vec<(String, u64, u64)> {
        // Mock network data for now
        vec![("eth0".to_string(), 0, 0)]
    }

    /// Get load average (Linux only)
    pub fn get_load_average(&self) -> Option<(f64, f64, f64)> {
        if let Ok(loadavg_str) = fs::read_to_string("/proc/loadavg") {
            let parts: Vec<&str> = loadavg_str.split_whitespace().collect();
            if parts.len() >= 3 {
                if let (Ok(load1), Ok(load5), Ok(load15)) = (
                    parts[0].parse::<f64>(),
                    parts[1].parse::<f64>(),
                    parts[2].parse::<f64>(),
                ) {
                    return Some((load1, load5, load15));
                }
            }
        }
        None
    }

    /// Get system temperature (if available)
    pub fn get_temperature(&mut self) -> Vec<(String, f32)> {
        // Try to read from thermal zones (Linux)
        let mut temperatures = Vec::new();

        if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("thermal_zone") {
                        let temp_path = path.join("temp");
                        if let Ok(temp_str) = fs::read_to_string(&temp_path) {
                            if let Ok(temp_millicelsius) = temp_str.trim().parse::<i32>() {
                                let temp_celsius = temp_millicelsius as f32 / 1000.0;
                                temperatures.push((name.to_string(), temp_celsius));
                            }
                        }
                    }
                }
            }
        }

        temperatures
    }

    /// Force refresh all stats
    pub fn refresh(&mut self) {
        self.last_update = None; // Force refresh
        self.refresh_stats();
    }
}

impl Default for SystemStatsService {
    fn default() -> Self {
        Self::new()
    }
}
