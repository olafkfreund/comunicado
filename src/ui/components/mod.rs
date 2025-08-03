//! Modular UI Component System
//!
//! This module provides a component-based architecture for Comunicado's UI,
//! replacing the monolithic UI structure with focused, reusable components.

pub mod traits;
pub mod registry;
pub mod email;
pub mod calendar;
pub mod contacts;
pub mod layout;
pub mod services;
pub mod examples;
pub mod modular_ui;
pub mod integration_example;

// Re-export core types
pub use traits::{
    UIComponent, ComponentError, ComponentResult,
    RenderContext, UIEvent, EventResult, ComponentMetrics,
};
pub use registry::{ComponentRegistry, ComponentHandle};
pub use services::{UIServices, ServiceProvider};
pub use layout::{LayoutManager, LayoutSpec, LayoutTemplate, ResponsiveRule};
pub use email::{EmailComponent, EmailComponentMode, EmailSection};
pub use calendar::{CalendarComponent, CalendarViewMode, CalendarPane, CalendarAction};
pub use contacts::{ContactsComponent, ContactsViewMode, ContactTab, ContactsPane, ContactAction};
pub use modular_ui::{ModularUI, AppMode, ModularUIMetrics};

// These types are defined in this module and exported directly

use std::any::TypeId;
use uuid::Uuid;
use ratatui::{layout::Rect, Frame};
use crate::theme::Theme;

/// Unique identifier for UI components
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(pub TypeId, pub Uuid);

impl ComponentId {
    /// Create a new component ID for a specific component type
    pub fn new<T: UIComponent + 'static>() -> Self {
        Self(TypeId::of::<T>(), Uuid::new_v4())
    }
    
    /// Create a component ID from a type and instance ID
    pub fn from_type<T: 'static>(instance_id: Uuid) -> Self {
        Self(TypeId::of::<T>(), instance_id)
    }
    
    /// Get the type ID
    pub fn type_id(&self) -> TypeId {
        self.0
    }
    
    /// Get the instance ID
    pub fn instance_id(&self) -> Uuid {
        self.1
    }
}

/// Component state management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentState {
    /// Component is not initialized
    Uninitialized,
    /// Component is ready for use
    Ready,
    /// Component is currently focused
    Focused,
    /// Component is hidden/inactive
    Hidden,
    /// Component encountered an error
    Error,
    /// Component is being destroyed
    Destroying,
}

impl ComponentState {
    /// Check if the component can handle events
    pub fn can_handle_events(&self) -> bool {
        matches!(self, ComponentState::Ready | ComponentState::Focused)
    }
    
    /// Check if the component should be rendered
    pub fn should_render(&self) -> bool {
        !matches!(self, ComponentState::Hidden | ComponentState::Destroying)
    }
    
    /// Check if the component is focused
    pub fn is_focused(&self) -> bool {
        matches!(self, ComponentState::Focused)
    }
}