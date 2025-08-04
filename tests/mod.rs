//! Test module declarations for Comunicado TUI testing

pub mod tui_test_framework;

// Re-export key testing utilities
pub use tui_test_framework::{
    TuiTestRunner, TestResult, KeySequence,
    run_comprehensive_tui_tests,
};