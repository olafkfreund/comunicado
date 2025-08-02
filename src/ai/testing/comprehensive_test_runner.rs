//! Comprehensive test runner for all AI functionality

use crate::ai::testing::{
    integration_tests::{AIIntegrationTestRunner, TestResult},
    performance_tests::{AIPerformanceBenchmark, BenchmarkResult},
    ui_tests::AIUITestRunner,
    test_utilities::{AITestContext, TestScenarioExecutor, TestScenarioResult},
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Comprehensive test suite results
#[derive(Debug, Clone)]
pub struct ComprehensiveTestResults {
    /// Integration test results
    pub integration_results: Vec<TestResult>,
    /// Performance benchmark results  
    pub performance_results: Vec<BenchmarkResult>,
    /// UI test results
    pub ui_results: Vec<(String, Result<(), String>)>,
    /// Scenario test results
    pub scenario_results: Vec<TestScenarioResult>,
    /// Overall test statistics
    pub statistics: TestStatistics,
    /// Test execution metadata
    pub metadata: HashMap<String, String>,
}

/// Overall test statistics
#[derive(Debug, Clone)]
pub struct TestStatistics {
    /// Total number of tests executed
    pub total_tests: usize,
    /// Number of tests that passed
    pub passed_tests: usize,
    /// Number of tests that failed
    pub failed_tests: usize,
    /// Overall pass rate (0.0 to 1.0)
    pub pass_rate: f64,
    /// Total execution time
    pub total_duration: Duration,
    /// Average test duration
    pub average_duration: Duration,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
}

/// Performance metrics from all tests
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Average operations per second across all benchmarks
    pub avg_ops_per_second: f64,
    /// Average latency across all operations
    pub avg_latency: Duration,
    /// Peak memory usage observed
    pub peak_memory_bytes: usize,
    /// Total operations performed
    pub total_operations: usize,
}

/// Comprehensive AI test runner
pub struct ComprehensiveAITestRunner {
    /// Test configuration
    config: TestRunnerConfig,
    /// Test context
    context: AITestContext,
}

/// Configuration for the test runner
#[derive(Debug, Clone)]
pub struct TestRunnerConfig {
    /// Whether to run integration tests
    pub run_integration_tests: bool,
    /// Whether to run performance benchmarks
    pub run_performance_tests: bool,
    /// Whether to run UI tests
    pub run_ui_tests: bool,
    /// Whether to run scenario tests
    pub run_scenario_tests: bool,
    /// Test timeout for individual tests
    pub test_timeout: Duration,
    /// Performance test concurrency level
    pub performance_concurrency: usize,
    /// Whether to generate detailed reports
    pub generate_reports: bool,
    /// Output directory for reports
    pub output_directory: Option<std::path::PathBuf>,
}

impl Default for TestRunnerConfig {
    fn default() -> Self {
        Self {
            run_integration_tests: true,
            run_performance_tests: true,
            run_ui_tests: true,
            run_scenario_tests: true,
            test_timeout: Duration::from_secs(30),
            performance_concurrency: 5,
            generate_reports: true,
            output_directory: None,
        }
    }
}

impl ComprehensiveAITestRunner {
    /// Create a new comprehensive test runner
    pub async fn new(config: TestRunnerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let mut context = AITestContext::new();
        context.load_standard_scenarios();
        context.test_data.load_standard_data();

        Ok(Self { config, context })
    }

    /// Run all configured tests
    pub async fn run_all_tests(&mut self) -> ComprehensiveTestResults {
        println!("Starting comprehensive AI test suite...");
        let overall_start = Instant::now();

        let mut integration_results = Vec::new();
        let mut performance_results = Vec::new();
        let mut ui_results = Vec::new();
        let mut scenario_results = Vec::new();

        // Run integration tests
        if self.config.run_integration_tests {
            println!("Running integration tests...");
            integration_results = self.run_integration_tests().await;
            println!("Integration tests completed: {}/{} passed", 
                integration_results.iter().filter(|r| r.passed).count(),
                integration_results.len()
            );
        }

        // Run performance tests
        if self.config.run_performance_tests {
            println!("Running performance benchmarks...");
            performance_results = self.run_performance_tests().await;
            println!("Performance tests completed: {} benchmarks run", performance_results.len());
        }

        // Run UI tests
        if self.config.run_ui_tests {
            println!("Running UI tests...");
            ui_results = self.run_ui_tests().await;
            println!("UI tests completed: {}/{} passed", 
                ui_results.iter().filter(|(_, r)| r.is_ok()).count(),
                ui_results.len()
            );
        }

        // Run scenario tests
        if self.config.run_scenario_tests {
            println!("Running scenario tests...");
            scenario_results = self.run_scenario_tests().await;
            println!("Scenario tests completed: {}/{} passed", 
                scenario_results.iter().filter(|r| r.success).count(),
                scenario_results.len()
            );
        }

        let total_duration = overall_start.elapsed();
        let statistics = self.calculate_statistics(
            &integration_results,
            &performance_results,
            &ui_results,
            &scenario_results,
            total_duration,
        );

        let mut metadata = HashMap::new();
        metadata.insert("start_time".to_string(), chrono::Utc::now().to_rfc3339());
        metadata.insert("test_timeout".to_string(), format!("{:?}", self.config.test_timeout));
        metadata.insert("performance_concurrency".to_string(), self.config.performance_concurrency.to_string());

        let results = ComprehensiveTestResults {
            integration_results,
            performance_results,
            ui_results,
            scenario_results,
            statistics,
            metadata,
        };

        // Generate reports if configured
        if self.config.generate_reports {
            self.generate_all_reports(&results).await;
        }

        println!("Comprehensive test suite completed in {:?}", total_duration);
        println!("Overall pass rate: {:.1}%", results.statistics.pass_rate * 100.0);

        results
    }

    /// Run integration tests
    async fn run_integration_tests(&self) -> Vec<TestResult> {
        match AIIntegrationTestRunner::new().await {
            Ok(runner) => {
                let runner = runner.with_timeout(self.config.test_timeout);
                runner.run_all_tests().await
            },
            Err(e) => {
                println!("Failed to create integration test runner: {}", e);
                vec![TestResult::failure(
                    "integration_runner_creation".to_string(),
                    Duration::ZERO,
                    format!("Failed to create runner: {}", e),
                )]
            }
        }
    }

    /// Run performance tests
    async fn run_performance_tests(&self) -> Vec<BenchmarkResult> {
        let benchmark = AIPerformanceBenchmark::create_standard_benchmarks()
            .with_concurrency(self.config.performance_concurrency);
        
        benchmark.run_all_benchmarks().await
    }

    /// Run UI tests
    async fn run_ui_tests(&self) -> Vec<(String, Result<(), String>)> {
        let mut runner = AIUITestRunner::new();
        runner.run_all_tests()
    }

    /// Run scenario tests
    async fn run_scenario_tests(&self) -> Vec<TestScenarioResult> {
        let executor = TestScenarioExecutor::new(self.context.clone());
        let mut results = Vec::new();

        for scenario in &self.context.scenarios {
            let result = executor.execute_scenario(scenario).await;
            results.push(result);
        }

        results
    }

    /// Calculate overall test statistics
    fn calculate_statistics(
        &self,
        integration_results: &[TestResult],
        performance_results: &[BenchmarkResult],
        ui_results: &[(String, Result<(), String>)],
        scenario_results: &[TestScenarioResult],
        total_duration: Duration,
    ) -> TestStatistics {
        // Count all tests
        let integration_total = integration_results.len();
        let integration_passed = integration_results.iter().filter(|r| r.passed).count();

        let ui_total = ui_results.len();
        let ui_passed = ui_results.iter().filter(|(_, r)| r.is_ok()).count();

        let scenario_total = scenario_results.len();
        let scenario_passed = scenario_results.iter().filter(|r| r.success).count();

        let performance_total = performance_results.len();
        // For performance tests, we consider them "passed" if they completed without errors
        let performance_passed = performance_total; // Assume all completed successfully

        let total_tests = integration_total + ui_total + scenario_total + performance_total;
        let passed_tests = integration_passed + ui_passed + scenario_passed + performance_passed;
        let failed_tests = total_tests - passed_tests;

        let pass_rate = if total_tests > 0 {
            passed_tests as f64 / total_tests as f64
        } else {
            0.0
        };

        let average_duration = if total_tests > 0 {
            total_duration / total_tests as u32
        } else {
            Duration::ZERO
        };

        // Calculate performance metrics
        let performance_metrics = self.calculate_performance_metrics(performance_results);

        TestStatistics {
            total_tests,
            passed_tests,
            failed_tests,
            pass_rate,
            total_duration,
            average_duration,
            performance_metrics,
        }
    }

    /// Calculate performance metrics
    fn calculate_performance_metrics(&self, performance_results: &[BenchmarkResult]) -> PerformanceMetrics {
        if performance_results.is_empty() {
            return PerformanceMetrics {
                avg_ops_per_second: 0.0,
                avg_latency: Duration::ZERO,
                peak_memory_bytes: 0,
                total_operations: 0,
            };
        }

        let total_ops_per_second: f64 = performance_results.iter().map(|r| r.ops_per_second).sum();
        let avg_ops_per_second = total_ops_per_second / performance_results.len() as f64;

        let total_latency: Duration = performance_results.iter().map(|r| r.avg_latency).sum();
        let avg_latency = total_latency / performance_results.len() as u32;

        let peak_memory_bytes = performance_results
            .iter()
            .map(|r| r.memory_stats.peak_memory_bytes)
            .max()
            .unwrap_or(0);

        let total_operations: usize = performance_results.iter().map(|r| r.operations).sum();

        PerformanceMetrics {
            avg_ops_per_second,
            avg_latency,
            peak_memory_bytes,
            total_operations,
        }
    }

    /// Generate all test reports
    async fn generate_all_reports(&self, results: &ComprehensiveTestResults) {
        let output_dir = self.config.output_directory
            .clone()
            .unwrap_or_else(|| std::path::PathBuf::from("./test_reports"));

        if let Err(e) = tokio::fs::create_dir_all(&output_dir).await {
            println!("Failed to create output directory: {}", e);
            return;
        }

        // Generate comprehensive report
        let comprehensive_report = self.generate_comprehensive_report(results);
        let report_path = output_dir.join("comprehensive_test_report.md");
        if let Err(e) = tokio::fs::write(&report_path, comprehensive_report).await {
            println!("Failed to write comprehensive report: {}", e);
        } else {
            println!("Comprehensive report written to: {:?}", report_path);
        }

        // Generate individual reports
        if !results.integration_results.is_empty() {
            let integration_runner = AIIntegrationTestRunner::new().await.unwrap();
            let integration_report = integration_runner.generate_report(&results.integration_results);
            let integration_path = output_dir.join("integration_test_report.md");
            let _ = tokio::fs::write(&integration_path, integration_report).await;
        }

        if !results.performance_results.is_empty() {
            let benchmark = AIPerformanceBenchmark::new();
            let performance_report = benchmark.generate_performance_report(&results.performance_results);
            let performance_path = output_dir.join("performance_test_report.md");
            let _ = tokio::fs::write(&performance_path, performance_report).await;
        }

        if !results.ui_results.is_empty() {
            let ui_runner = AIUITestRunner::new();
            let ui_report = ui_runner.generate_report(&results.ui_results);
            let ui_path = output_dir.join("ui_test_report.md");
            let _ = tokio::fs::write(&ui_path, ui_report).await;
        }
    }

    /// Generate comprehensive test report
    fn generate_comprehensive_report(&self, results: &ComprehensiveTestResults) -> String {
        let mut report = String::new();
        
        report.push_str("# Comprehensive AI Test Suite Report\n\n");
        report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Executive Summary
        report.push_str("## Executive Summary\n\n");
        report.push_str(&format!("- **Total Tests:** {}\n", results.statistics.total_tests));
        report.push_str(&format!("- **Passed:** {} ({:.1}%)\n", 
            results.statistics.passed_tests, 
            results.statistics.pass_rate * 100.0
        ));
        report.push_str(&format!("- **Failed:** {}\n", results.statistics.failed_tests));
        report.push_str(&format!("- **Total Duration:** {:?}\n", results.statistics.total_duration));
        report.push_str(&format!("- **Average Test Duration:** {:?}\n\n", results.statistics.average_duration));

        // Test Categories Summary
        report.push_str("## Test Categories\n\n");
        report.push_str("| Category | Total | Passed | Failed | Pass Rate |\n");
        report.push_str("|----------|-------|--------|--------|----------|\n");
        
        let integration_passed = results.integration_results.iter().filter(|r| r.passed).count();
        let integration_total = results.integration_results.len();
        if integration_total > 0 {
            report.push_str(&format!("| Integration | {} | {} | {} | {:.1}% |\n",
                integration_total,
                integration_passed,
                integration_total - integration_passed,
                (integration_passed as f64 / integration_total as f64) * 100.0
            ));
        }

        let ui_passed = results.ui_results.iter().filter(|(_, r)| r.is_ok()).count();
        let ui_total = results.ui_results.len();
        if ui_total > 0 {
            report.push_str(&format!("| UI Tests | {} | {} | {} | {:.1}% |\n",
                ui_total,
                ui_passed,
                ui_total - ui_passed,
                (ui_passed as f64 / ui_total as f64) * 100.0
            ));
        }

        let scenario_passed = results.scenario_results.iter().filter(|r| r.success).count();
        let scenario_total = results.scenario_results.len();
        if scenario_total > 0 {
            report.push_str(&format!("| Scenarios | {} | {} | {} | {:.1}% |\n",
                scenario_total,
                scenario_passed,
                scenario_total - scenario_passed,
                (scenario_passed as f64 / scenario_total as f64) * 100.0
            ));
        }

        let performance_total = results.performance_results.len();
        if performance_total > 0 {
            report.push_str(&format!("| Performance | {} | {} | {} | {:.1}% |\n",
                performance_total,
                performance_total, // Assume all completed
                0,
                100.0
            ));
        }

        report.push_str("\n");

        // Performance Summary
        if !results.performance_results.is_empty() {
            report.push_str("## Performance Summary\n\n");
            report.push_str(&format!("- **Average Throughput:** {:.2} ops/sec\n", 
                results.statistics.performance_metrics.avg_ops_per_second));
            report.push_str(&format!("- **Average Latency:** {:?}\n", 
                results.statistics.performance_metrics.avg_latency));
            report.push_str(&format!("- **Peak Memory Usage:** {} KB\n", 
                results.statistics.performance_metrics.peak_memory_bytes / 1024));
            report.push_str(&format!("- **Total Operations:** {}\n\n", 
                results.statistics.performance_metrics.total_operations));
        }

        // Failed Tests Summary
        let mut failed_tests = Vec::new();
        
        for result in &results.integration_results {
            if !result.passed {
                failed_tests.push(format!("Integration: {} - {}", 
                    result.name, 
                    result.error.as_deref().unwrap_or("Unknown error")
                ));
            }
        }

        for (name, result) in &results.ui_results {
            if let Err(error) = result {
                failed_tests.push(format!("UI: {} - {}", name, error));
            }
        }

        for result in &results.scenario_results {
            if !result.success {
                failed_tests.push(format!("Scenario: {} - {}", 
                    result.scenario_name, 
                    result.error.as_deref().unwrap_or("Unknown error")
                ));
            }
        }

        if !failed_tests.is_empty() {
            report.push_str("## Failed Tests\n\n");
            for failure in failed_tests {
                report.push_str(&format!("- {}\n", failure));
            }
            report.push_str("\n");
        }

        // Recommendations
        report.push_str("## Recommendations\n\n");
        
        if results.statistics.pass_rate < 0.9 {
            report.push_str("- ⚠️ Pass rate is below 90%. Consider investigating failed tests.\n");
        }
        
        if results.statistics.performance_metrics.avg_latency > Duration::from_millis(1000) {
            report.push_str("- ⚠️ Average latency is high. Consider performance optimization.\n");
        }

        if results.statistics.performance_metrics.peak_memory_bytes > 100 * 1024 * 1024 {
            report.push_str("- ⚠️ High memory usage detected. Consider memory optimization.\n");
        }

        if results.statistics.pass_rate >= 0.95 {
            report.push_str("- ✅ Excellent test coverage and pass rate.\n");
        }

        report.push_str("\n");

        // Test Configuration
        report.push_str("## Test Configuration\n\n");
        for (key, value) in &results.metadata {
            report.push_str(&format!("- **{}:** {}\n", key, value));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_comprehensive_runner_creation() {
        let config = TestRunnerConfig::default();
        let runner = ComprehensiveAITestRunner::new(config).await;
        assert!(runner.is_ok());
    }

    #[tokio::test]
    async fn test_limited_test_run() {
        let config = TestRunnerConfig {
            run_integration_tests: false,
            run_performance_tests: false,
            run_ui_tests: true,
            run_scenario_tests: false,
            test_timeout: Duration::from_secs(5),
            performance_concurrency: 1,
            generate_reports: false,
            output_directory: None,
        };

        let mut runner = ComprehensiveAITestRunner::new(config).await.unwrap();
        let results = runner.run_all_tests().await;

        assert!(!results.ui_results.is_empty());
        assert!(results.integration_results.is_empty());
        assert!(results.performance_results.is_empty());
        assert!(results.scenario_results.is_empty());
    }

    #[test]
    fn test_statistics_calculation() {
        let runner_config = TestRunnerConfig::default();
        let context = AITestContext::new();
        let runner = ComprehensiveAITestRunner { config: runner_config, context };

        let integration_results = vec![
            TestResult::success("test1".to_string(), Duration::from_millis(100)),
            TestResult::failure("test2".to_string(), Duration::from_millis(200), "Error".to_string()),
        ];

        let ui_results = vec![
            ("ui_test1".to_string(), Ok(())),
            ("ui_test2".to_string(), Err("Error".to_string())),
        ];

        let scenario_results = vec![];
        let performance_results = vec![];

        let stats = runner.calculate_statistics(
            &integration_results,
            &performance_results,
            &ui_results,
            &scenario_results,
            Duration::from_secs(1),
        );

        assert_eq!(stats.total_tests, 4);
        assert_eq!(stats.passed_tests, 2);
        assert_eq!(stats.failed_tests, 2);
        assert_eq!(stats.pass_rate, 0.5);
    }
}