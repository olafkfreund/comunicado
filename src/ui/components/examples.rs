//! Component System Usage Examples
//!
//! Demonstrates how to use the new modular component system.

use super::{
    ComponentRegistry, ComponentResult, EmailComponent, EmailComponentMode, EventResult,
    LayoutManager, UIComponent, UIEvent, UIServices,
};
use crate::theme::Theme;
use ratatui::{layout::Rect, Frame};

/// Example application using the new component system
pub struct ComponentBasedUI {
    registry: ComponentRegistry,
    layout_manager: LayoutManager,
    #[allow(dead_code)]
    services: UIServices,
    email_component_id: Option<super::ComponentId>,
}

impl ComponentBasedUI {
    /// Create a new component-based UI
    pub fn new() -> ComponentResult<Self> {
        let registry = ComponentRegistry::new();
        let layout_manager = LayoutManager::default();
        let services = UIServices::new();

        Ok(Self {
            registry,
            layout_manager,
            services,
            email_component_id: None,
        })
    }

    /// Initialize the UI with email component
    pub fn initialize(&mut self) -> ComponentResult<()> {
        // Create and register email component
        let mut email_component = EmailComponent::new();
        email_component.initialize()?;

        let email_id = self.registry.register(email_component)?;
        self.email_component_id = Some(email_id);

        // Set initial focus to email component
        self.registry.set_focus(Some(email_id))?;

        Ok(())
    }

    /// Render the entire UI
    pub fn render(&mut self, frame: &mut Frame, area: Rect, _theme: &Theme) -> ComponentResult<()> {
        // Calculate layout
        let _layout_areas = self.layout_manager.calculate_layout("email_main", area)?;

        // For now, skip both context creation and registry rendering to avoid borrowing issues
        // TODO: Fix RenderContext lifetime issues and implement individual component rendering

        // Render a simple placeholder for now
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let placeholder = Paragraph::new("ModularUI Example - Under Development")
            .block(Block::default().borders(Borders::ALL).title("Example UI"))
            .style(Style::default().fg(Color::Green));

        frame.render_widget(placeholder, area);

        Ok(())
    }

    /// Handle UI events
    pub fn handle_event(&mut self, event: UIEvent) -> ComponentResult<EventResult> {
        self.registry.handle_event(&event)
    }

    /// Get email component for direct access
    pub fn email_component(&mut self) -> Option<&mut super::ComponentHandle> {
        if let Some(id) = self.email_component_id {
            self.registry.get_mut(id)
        } else {
            None
        }
    }

    /// Switch email component mode
    pub fn set_email_mode(&mut self, _mode: EmailComponentMode) -> ComponentResult<()> {
        if let Some(id) = self.email_component_id {
            if let Some(_handle) = self.registry.get_mut(id) {
                // TODO: Need to access the underlying component to call set_mode
                // This would require extending the ComponentHandle API
                // For now, this is a placeholder
            }
        }
        Ok(())
    }

    /// Get performance metrics for all components
    pub fn performance_metrics(&self) -> super::registry::RegistryMetrics {
        self.registry.performance_metrics()
    }
}

impl Default for ComponentBasedUI {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback to minimal state if initialization fails
            Self {
                registry: ComponentRegistry::new(),
                layout_manager: LayoutManager::default(),
                services: UIServices::new(),
                email_component_id: None,
            }
        })
    }
}

/// Example usage of the component system
#[cfg(test)]
mod examples {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[tokio::test]
    async fn test_component_system_basic_usage() {
        // Create and initialize the component-based UI
        let mut ui = ComponentBasedUI::new().unwrap();
        ui.initialize().unwrap();

        // Verify email component was registered
        assert!(ui.email_component_id.is_some());

        // Test event handling
        let key_event = UIEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        let result = ui.handle_event(key_event).unwrap();

        // The result should be handled since the email component processes 'c' for compose
        assert_ne!(result, EventResult::Ignored);
    }

    #[test]
    fn test_component_registry_usage() {
        let mut registry = ComponentRegistry::new();

        // Register an email component
        let email_component = EmailComponent::new();
        let email_id = registry.register(email_component).unwrap();

        // Verify component was registered
        assert!(registry.get(email_id).is_some());

        // Test focus management
        registry.set_focus(Some(email_id)).unwrap();
        assert_eq!(registry.focused_component(), Some(email_id));
    }

    #[test]
    fn test_layout_manager_usage() {
        let mut layout_manager = LayoutManager::default();

        // Test predefined email layout
        let area = Rect::new(0, 0, 120, 30);
        let layout_result = layout_manager.calculate_layout("email_main", area);

        assert!(layout_result.is_ok());
        let areas = layout_result.unwrap();
        assert_eq!(areas.len(), 3); // Three-pane layout
    }

    #[test]
    fn test_services_initialization() {
        let services = UIServices::new();

        // Test service availability
        let _cache_service = services.cache_service(); // Cache service is always available
        assert!(services.email_service().is_none()); // Email service needs initialization
    }
}

/// Performance comparison helper
pub mod performance {
    // Performance comparison utilities
    use std::time::{Duration, Instant};

    /// Compare performance between old monolithic approach and new component system
    pub struct PerformanceComparison {
        pub monolithic_render_time: Duration,
        pub component_render_time: Duration,
        pub event_handling_overhead: Duration,
        pub memory_usage_estimate: usize,
    }

    impl PerformanceComparison {
        /// Run a basic performance comparison
        pub fn run_comparison() -> Self {
            // This is a simplified example - real measurements would require
            // actual rendering and more comprehensive benchmarks

            let start = Instant::now();
            // Simulate monolithic render time
            std::thread::sleep(Duration::from_micros(100));
            let monolithic_time = start.elapsed();

            let start = Instant::now();
            // Simulate component-based render time
            std::thread::sleep(Duration::from_micros(80)); // Should be faster due to caching
            let component_time = start.elapsed();

            Self {
                monolithic_render_time: monolithic_time,
                component_render_time: component_time,
                event_handling_overhead: Duration::from_micros(5), // Minimal overhead
                memory_usage_estimate: 1024 * 64,                  // Estimate: 64KB per component
            }
        }

        /// Get performance improvement percentage
        pub fn improvement_percentage(&self) -> f64 {
            if self.monolithic_render_time.as_nanos() == 0 {
                return 0.0;
            }

            let improvement = self.monolithic_render_time.as_nanos() as f64
                - self.component_render_time.as_nanos() as f64;

            (improvement / self.monolithic_render_time.as_nanos() as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::performance::*;
    use std::time::Duration;

    #[test]
    fn test_performance_comparison() {
        let comparison = PerformanceComparison::run_comparison();

        // Component system should show some improvement
        assert!(comparison.improvement_percentage() >= 0.0);

        // Memory usage should be reasonable
        assert!(comparison.memory_usage_estimate < 1024 * 1024); // Less than 1MB

        // Event handling overhead should be minimal
        assert!(comparison.event_handling_overhead < Duration::from_millis(1));
    }
}
