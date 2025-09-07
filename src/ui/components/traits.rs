//! Core UI Component Traits
//!
//! Defines the fundamental interfaces for UI components in the modular architecture.

use super::{ComponentId, ComponentState};
use crate::theme::Theme;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{layout::Rect, Frame};
use std::any::Any;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Result type for component operations
pub type ComponentResult<T> = Result<T, ComponentError>;

/// Component-specific error types
#[derive(Debug, Error)]
pub enum ComponentError {
    #[error("Component initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Component render failed: {0}")]
    RenderFailed(String),

    #[error("Component event handling failed: {0}")]
    EventHandlingFailed(String),

    #[error("Component state transition invalid: from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: ComponentState,
        to: ComponentState,
    },

    #[error("Component not ready: current state is {state:?}")]
    NotReady { state: ComponentState },

    #[error("Component dependency missing: {dependency}")]
    DependencyMissing { dependency: String },

    #[error("Layout error: {0}")]
    LayoutError(String),

    #[error("Layout calculation error")]
    LayoutCalculationError(#[from] super::layout::LayoutError),

    #[error("Unknown component error: {0}")]
    Unknown(String),
}

/// Context provided to components during rendering
#[derive(Debug)]
pub struct RenderContext<'a> {
    /// The frame to render to
    pub frame: &'a mut Frame<'a>,
    /// The area allocated for this component
    pub area: Rect,
    /// Current theme
    pub theme: &'a Theme,
    /// Component's current state
    pub state: ComponentState,
    /// Whether this component is currently focused
    pub is_focused: bool,
    /// Render timestamp for animations
    pub timestamp: Instant,
}

impl<'a> RenderContext<'a> {
    /// Create a new render context
    pub fn new(
        frame: &'a mut Frame<'a>,
        area: Rect,
        theme: &'a Theme,
        state: ComponentState,
        is_focused: bool,
    ) -> Self {
        Self {
            frame,
            area,
            theme,
            state,
            is_focused,
            timestamp: Instant::now(),
        }
    }

    /// Update the area for this context
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
    }
}

/// UI events that components can handle
#[derive(Debug)]
pub enum UIEvent {
    /// Key press event
    Key(KeyEvent),
    /// Mouse event  
    Mouse(MouseEvent),
    /// Component gained focus
    FocusGained,
    /// Component lost focus
    FocusLost,
    /// Component became visible
    Show,
    /// Component became hidden
    Hide,
    /// Theme changed
    ThemeChanged,
    /// Window/terminal resized
    Resize { width: u16, height: u16 },
    /// Custom component-specific event
    Custom {
        event_type: String,
        data: Box<dyn Any + Send>,
    },
}

impl Clone for UIEvent {
    fn clone(&self) -> Self {
        match self {
            UIEvent::Key(k) => UIEvent::Key(*k),
            UIEvent::Mouse(m) => UIEvent::Mouse(*m),
            UIEvent::FocusGained => UIEvent::FocusGained,
            UIEvent::FocusLost => UIEvent::FocusLost,
            UIEvent::Show => UIEvent::Show,
            UIEvent::Hide => UIEvent::Hide,
            UIEvent::ThemeChanged => UIEvent::ThemeChanged,
            UIEvent::Resize { width, height } => UIEvent::Resize {
                width: *width,
                height: *height,
            },
            UIEvent::Custom { event_type, .. } => {
                // Custom events with Any data cannot be cloned generically
                // Create a new custom event with just the type
                UIEvent::Custom {
                    event_type: event_type.clone(),
                    data: Box::new(()),
                }
            }
        }
    }
}

impl UIEvent {
    /// Create a custom event with typed data
    pub fn custom<T: Any + Send>(event_type: &str, data: T) -> Self {
        Self::Custom {
            event_type: event_type.to_string(),
            data: Box::new(data),
        }
    }

    /// Try to extract custom event data
    pub fn custom_data<T: Any + Send>(&self) -> Option<&T> {
        if let UIEvent::Custom { data, .. } = self {
            data.downcast_ref::<T>()
        } else {
            None
        }
    }
}

/// Result of event handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventResult {
    /// Event was handled and consumed
    Consumed,
    /// Event was handled but should continue propagating
    Handled,
    /// Event was ignored/not handled
    Ignored,
    /// Request focus change to another component
    RequestFocus(ComponentId),
    /// Request mode change (placeholder - actual mode type may vary)
    RequestModeChange(String),
    /// Request component state change
    RequestStateChange(ComponentState),
    /// Request application quit
    RequestQuit,
    /// Request data refresh
    RequestRefresh,
}

/// Performance metrics for components
#[derive(Debug, Clone)]
pub struct ComponentMetrics {
    /// Last render time
    pub last_render_time: Duration,
    /// Average render time over last 10 frames
    pub avg_render_time: Duration,
    /// Number of events processed
    pub events_processed: u64,
    /// Number of render calls
    pub render_calls: u64,
    /// Memory usage estimate
    pub memory_usage_bytes: usize,
    /// Last update timestamp
    pub last_updated: Instant,
}

impl Default for ComponentMetrics {
    fn default() -> Self {
        Self {
            last_render_time: Duration::ZERO,
            avg_render_time: Duration::ZERO,
            events_processed: 0,
            render_calls: 0,
            memory_usage_bytes: 0,
            last_updated: Instant::now(),
        }
    }
}

/// Core trait that all UI components must implement
pub trait UIComponent: Send + Sync + std::fmt::Debug {
    /// Get the unique identifier for this component
    fn component_id(&self) -> ComponentId;

    /// Get a human-readable name for this component
    fn component_name(&self) -> &str;

    /// Get the current state of the component
    fn state(&self) -> ComponentState;

    /// Initialize the component
    fn initialize(&mut self) -> ComponentResult<()> {
        Ok(())
    }

    /// Render the component
    fn render(&mut self, context: &mut RenderContext<'_>) -> ComponentResult<()>;

    /// Handle UI events
    fn handle_event(&mut self, event: &UIEvent) -> ComponentResult<EventResult> {
        match event {
            UIEvent::FocusGained => self.on_focus_gained(),
            UIEvent::FocusLost => self.on_focus_lost(),
            UIEvent::Show => self.on_show(),
            UIEvent::Hide => self.on_hide(),
            UIEvent::ThemeChanged => self.on_theme_changed(),
            UIEvent::Resize { width, height } => self.on_resize(*width, *height),
            _ => Ok(EventResult::Ignored),
        }
    }

    /// Called when component gains focus
    fn on_focus_gained(&mut self) -> ComponentResult<EventResult> {
        Ok(EventResult::Handled)
    }

    /// Called when component loses focus
    fn on_focus_lost(&mut self) -> ComponentResult<EventResult> {
        Ok(EventResult::Handled)
    }

    /// Called when component becomes visible
    fn on_show(&mut self) -> ComponentResult<EventResult> {
        Ok(EventResult::Handled)
    }

    /// Called when component becomes hidden
    fn on_hide(&mut self) -> ComponentResult<EventResult> {
        Ok(EventResult::Handled)
    }

    /// Called when theme changes
    fn on_theme_changed(&mut self) -> ComponentResult<EventResult> {
        Ok(EventResult::Handled)
    }

    /// Called when terminal is resized
    fn on_resize(&mut self, _width: u16, _height: u16) -> ComponentResult<EventResult> {
        Ok(EventResult::Handled)
    }

    /// Get performance metrics
    fn metrics(&self) -> &ComponentMetrics;

    /// Update the component state
    fn set_state(&mut self, new_state: ComponentState) -> ComponentResult<()>;

    /// Check if the component can accept focus
    fn can_focus(&self) -> bool {
        self.state().can_handle_events()
    }

    /// Get component-specific configuration
    fn config(&self) -> ComponentConfig {
        ComponentConfig::default()
    }

    /// Cleanup component resources
    fn cleanup(&mut self) -> ComponentResult<()> {
        Ok(())
    }
}

/// Configuration for component behavior
#[derive(Debug, Clone)]
pub struct ComponentConfig {
    /// Whether the component can receive focus
    pub focusable: bool,
    /// Whether the component should cache render results
    pub cache_renders: bool,
    /// Minimum update interval for performance
    pub min_update_interval: Duration,
    /// Whether the component handles mouse events
    pub handle_mouse: bool,
    /// Priority for event handling (higher = first)
    pub event_priority: i32,
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self {
            focusable: true,
            cache_renders: false,
            min_update_interval: Duration::from_millis(16), // ~60fps
            handle_mouse: false,
            event_priority: 0,
        }
    }
}

/// Trait for components that manage child components
pub trait ContainerComponent: UIComponent {
    /// Get child components
    fn children(&self) -> Vec<ComponentId>;

    /// Add a child component
    fn add_child(&mut self, child_id: ComponentId) -> ComponentResult<()>;

    /// Remove a child component
    fn remove_child(&mut self, child_id: ComponentId) -> ComponentResult<()>;

    /// Get the currently focused child (if any)
    fn focused_child(&self) -> Option<ComponentId>;

    /// Set focus to a specific child
    fn focus_child(&mut self, child_id: ComponentId) -> ComponentResult<()>;
}

/// Trait for components that can be serialized/deserialized for state persistence
pub trait StatefulComponent: UIComponent {
    type State: serde::Serialize + serde::de::DeserializeOwned;

    /// Save component state
    fn save_state(&self) -> ComponentResult<Self::State>;

    /// Restore component state
    fn restore_state(&mut self, state: Self::State) -> ComponentResult<()>;
}

/// Trait for components that provide keyboard shortcuts
pub trait ShortcutProvider: UIComponent {
    /// Get keyboard shortcuts provided by this component
    fn shortcuts(&self) -> Vec<crate::keyboard::KeyboardShortcut>;

    /// Handle a shortcut activation
    fn handle_shortcut(&mut self, shortcut_id: &str) -> ComponentResult<EventResult>;
}
