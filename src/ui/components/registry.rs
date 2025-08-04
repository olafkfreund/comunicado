//! Component Registry
//!
//! Manages the lifecycle and coordination of UI components.

use super::{
    ComponentId, ComponentState, UIComponent, UIEvent, EventResult, ComponentError, ComponentResult,
    RenderContext, ComponentMetrics,
};
use std::collections::HashMap;
use std::any::TypeId;
use std::time::{Duration, Instant};

/// Handle to a registered component
#[derive(Debug)]
pub struct ComponentHandle {
    id: ComponentId,
    component: Box<dyn UIComponent>,
    state: ComponentState,
    metrics: ComponentMetrics,
    last_event_time: Instant,
}

impl ComponentHandle {
    /// Create a new component handle
    pub fn new(mut component: Box<dyn UIComponent>) -> ComponentResult<Self> {
        let id = component.component_id();
        
        // Initialize the component
        component.initialize()?;
        
        Ok(Self {
            id,
            component,
            state: ComponentState::Ready,
            metrics: ComponentMetrics::default(),
            last_event_time: Instant::now(),
        })
    }
    
    /// Get the component ID
    pub fn id(&self) -> ComponentId {
        self.id
    }
    
    /// Get the component state
    pub fn state(&self) -> ComponentState {
        self.state
    }
    
    /// Set the component state
    pub fn set_state(&mut self, new_state: ComponentState) -> ComponentResult<()> {
        let old_state = self.state;
        
        // Validate state transition
        if !Self::is_valid_transition(old_state, new_state) {
            return Err(ComponentError::InvalidStateTransition {
                from: old_state,
                to: new_state,
            });
        }
        
        self.state = new_state;
        self.component.set_state(new_state)?;
        
        Ok(())
    }
    
    /// Check if a state transition is valid
    fn is_valid_transition(from: ComponentState, to: ComponentState) -> bool {
        use ComponentState::*;
        
        match (from, to) {
            // Can always go to error or destroying states
            (_, Error) | (_, Destroying) => true,
            
            // From uninitialized
            (Uninitialized, Ready) => true,
            
            // Between active states
            (Ready, Focused) | (Focused, Ready) => true,
            (Ready, Hidden) | (Hidden, Ready) => true,
            (Focused, Hidden) | (Hidden, Focused) => true,
            
            // Can recover from error
            (Error, Ready) | (Error, Hidden) => true,
            
            // Invalid transitions
            _ => false,
        }
    }
    
    /// Render the component if it should be rendered
    pub fn render(&mut self, mut context: RenderContext<'_>) -> ComponentResult<()> {
        if !self.state.should_render() {
            return Ok(());
        }
        
        let start_time = Instant::now();
        
        // Render the component
        self.component.render(&mut context)?;
        
        // Update metrics
        let render_time = start_time.elapsed();
        self.metrics.last_render_time = render_time;
        self.metrics.render_calls += 1;
        
        // Update average render time (simple moving average over last 10 frames)
        let weight = 0.1;
        self.metrics.avg_render_time = Duration::from_nanos(
            (self.metrics.avg_render_time.as_nanos() as f64 * (1.0 - weight) +
             render_time.as_nanos() as f64 * weight) as u64
        );
        
        self.metrics.last_updated = Instant::now();
        
        Ok(())
    }
    
    /// Handle an event if the component can handle events
    pub fn handle_event(&mut self, event: &UIEvent) -> ComponentResult<EventResult> {
        if !self.state.can_handle_events() {
            return Ok(EventResult::Ignored);
        }
        
        self.last_event_time = Instant::now();
        self.metrics.events_processed += 1;
        
        self.component.handle_event(event)
    }
    
    /// Get component metrics
    pub fn metrics(&self) -> &ComponentMetrics {
        &self.metrics
    }
    
    /// Get component name
    pub fn name(&self) -> &str {
        self.component.component_name()
    }
    
    /// Check if the component can accept focus
    pub fn can_focus(&self) -> bool {
        self.state.can_handle_events() && self.component.can_focus()
    }
    
    /// Cleanup the component
    pub fn cleanup(&mut self) -> ComponentResult<()> {
        self.set_state(ComponentState::Destroying)?;
        self.component.cleanup()
    }
}

/// Registry for managing UI components
pub struct ComponentRegistry {
    /// Registered components
    components: HashMap<ComponentId, ComponentHandle>,
    
    /// Component lookup by type
    type_registry: HashMap<TypeId, Vec<ComponentId>>,
    
    /// Focus management
    focused_component: Option<ComponentId>,
    focus_history: Vec<ComponentId>,
    
    /// Event routing
    event_handlers: HashMap<TypeId, Vec<ComponentId>>,
    
    /// Performance tracking
    total_render_time: std::time::Duration,
    total_events_processed: u64,
}

impl ComponentRegistry {
    /// Create a new component registry
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            type_registry: HashMap::new(),
            focused_component: None,
            focus_history: Vec::new(),
            event_handlers: HashMap::new(),
            total_render_time: std::time::Duration::ZERO,
            total_events_processed: 0,
        }
    }
    
    /// Register a new component
    pub fn register<T: UIComponent + 'static>(&mut self, component: T) -> ComponentResult<ComponentId> {
        let component_box = Box::new(component);
        let id = component_box.component_id();
        let type_id = TypeId::of::<T>();
        
        // Create component handle
        let handle = ComponentHandle::new(component_box)?;
        
        // Register in various indices
        self.components.insert(id, handle);
        self.type_registry.entry(type_id).or_insert_with(Vec::new).push(id);
        
        Ok(id)
    }
    
    /// Unregister a component
    pub fn unregister(&mut self, component_id: ComponentId) -> ComponentResult<()> {
        if let Some(mut handle) = self.components.remove(&component_id) {
            // Cleanup the component
            handle.cleanup()?;
            
            // Remove from type registry
            let type_id = component_id.type_id();
            if let Some(components) = self.type_registry.get_mut(&type_id) {
                components.retain(|&id| id != component_id);
                if components.is_empty() {
                    self.type_registry.remove(&type_id);
                }
            }
            
            // Remove from focus management
            if self.focused_component == Some(component_id) {
                self.focused_component = None;
            }
            self.focus_history.retain(|&id| id != component_id);
            
            // Remove from event handlers
            for handlers in self.event_handlers.values_mut() {
                handlers.retain(|&id| id != component_id);
            }
        }
        
        Ok(())
    }
    
    /// Get a component by ID
    pub fn get(&self, component_id: ComponentId) -> Option<&ComponentHandle> {
        self.components.get(&component_id)
    }
    
    /// Get a mutable component by ID
    pub fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ComponentHandle> {
        self.components.get_mut(&component_id)
    }
    
    /// Get all components of a specific type
    pub fn get_by_type<T: 'static>(&self) -> Vec<&ComponentHandle> {
        let type_id = TypeId::of::<T>();
        if let Some(component_ids) = self.type_registry.get(&type_id) {
            component_ids.iter()
                .filter_map(|&id| self.components.get(&id))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Render a specific component by ID
    pub fn render_component(&mut self, component_id: ComponentId, mut context: RenderContext<'_>) -> ComponentResult<()> {
        if let Some(handle) = self.components.get_mut(&component_id) {
            if handle.state().should_render() {
                // Update context focus state
                let is_focused = self.focused_component == Some(component_id);
                context.is_focused = is_focused;
                context.state = handle.state();
                
                handle.render(context)?;
            }
        }
        Ok(())
    }
    
    /// Get a list of visible component IDs in render order  
    pub fn get_visible_components(&self) -> Vec<ComponentId> {
        self.components.keys()
            .filter(|id| {
                self.components.get(id)
                    .map(|h| h.state().should_render())
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }
    
    /// Send an event to all components that can handle it
    pub fn broadcast_event(&mut self, event: &UIEvent) -> ComponentResult<Vec<EventResult>> {
        let mut results = Vec::new();
        
        // Get list of components to avoid borrowing issues
        let component_ids: Vec<ComponentId> = self.components.keys().cloned().collect();
        
        for component_id in component_ids {
            if let Some(handle) = self.components.get_mut(&component_id) {
                match handle.handle_event(event) {
                    Ok(result) => {
                        results.push(result);
                        self.total_events_processed += 1;
                    }
                    Err(e) => {
                        // Set component to error state and continue
                        let _ = handle.set_state(ComponentState::Error);
                        return Err(e);
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Send an event to the focused component first, then others
    pub fn handle_event(&mut self, event: &UIEvent) -> ComponentResult<EventResult> {
        // Try focused component first
        if let Some(focused_id) = self.focused_component {
            if let Some(handle) = self.components.get_mut(&focused_id) {
                match handle.handle_event(event)? {
                    EventResult::Consumed => return Ok(EventResult::Consumed),
                    EventResult::RequestFocus(new_focus) => {
                        self.set_focus(Some(new_focus))?;
                        return Ok(EventResult::Handled);
                    }
                    EventResult::RequestStateChange(new_state) => {
                        handle.set_state(new_state)?;
                        return Ok(EventResult::Handled);
                    }
                    other => {
                        if other != EventResult::Ignored {
                            return Ok(other);
                        }
                    }
                }
            }
        }
        
        // Try other components
        let component_ids: Vec<ComponentId> = self.components.keys()
            .filter(|&&id| Some(id) != self.focused_component)
            .cloned()
            .collect();
            
        for component_id in component_ids {
            if let Some(handle) = self.components.get_mut(&component_id) {
                match handle.handle_event(event)? {
                    EventResult::Consumed => return Ok(EventResult::Consumed),
                    EventResult::RequestFocus(new_focus) => {
                        self.set_focus(Some(new_focus))?;
                        return Ok(EventResult::Handled);
                    }
                    EventResult::RequestStateChange(new_state) => {
                        handle.set_state(new_state)?;
                        return Ok(EventResult::Handled);
                    }
                    EventResult::Ignored => continue,
                    other => return Ok(other),
                }
            }
        }
        
        Ok(EventResult::Ignored)
    }
    
    /// Set focus to a component
    pub fn set_focus(&mut self, component_id: Option<ComponentId>) -> ComponentResult<()> {
        // Remove focus from current component
        if let Some(current_focus) = self.focused_component {
            if let Some(handle) = self.components.get_mut(&current_focus) {
                handle.handle_event(&UIEvent::FocusLost)?;
                handle.set_state(ComponentState::Ready)?;
            }
        }
        
        // Set focus to new component
        if let Some(new_focus) = component_id {
            if let Some(handle) = self.components.get_mut(&new_focus) {
                if handle.can_focus() {
                    handle.handle_event(&UIEvent::FocusGained)?;
                    handle.set_state(ComponentState::Focused)?;
                    
                    // Update focus history
                    if let Some(old_focus) = self.focused_component {
                        self.focus_history.push(old_focus);
                    }
                    
                    self.focused_component = Some(new_focus);
                } else {
                    return Err(ComponentError::NotReady { state: handle.state() });
                }
            } else {
                return Err(ComponentError::Unknown(
                    format!("Component {:?} not found", new_focus)
                ));
            }
        } else {
            self.focused_component = None;
        }
        
        Ok(())
    }
    
    /// Get the currently focused component
    pub fn focused_component(&self) -> Option<ComponentId> {
        self.focused_component
    }
    
    /// Cycle focus to the next focusable component
    pub fn cycle_focus(&mut self) -> ComponentResult<()> {
        let focusable_components: Vec<ComponentId> = self.components.iter()
            .filter(|(_, handle)| handle.can_focus())
            .map(|(&id, _)| id)
            .collect();
            
        if focusable_components.is_empty() {
            return Ok(());
        }
        
        let next_focus = if let Some(current) = self.focused_component {
            if let Some(current_index) = focusable_components.iter().position(|&id| id == current) {
                let next_index = (current_index + 1) % focusable_components.len();
                focusable_components[next_index]
            } else {
                focusable_components[0]
            }
        } else {
            focusable_components[0]
        };
        
        self.set_focus(Some(next_focus))
    }
    
    /// Get overall performance metrics
    pub fn performance_metrics(&self) -> RegistryMetrics {
        RegistryMetrics {
            total_components: self.components.len(),
            total_render_time: self.total_render_time,
            total_events_processed: self.total_events_processed,
            focused_component: self.focused_component,
            component_metrics: self.components.iter()
                .map(|(&id, handle)| (id, handle.metrics().clone()))
                .collect(),
        }
    }
    
    /// Cleanup all components
    pub fn cleanup_all(&mut self) -> ComponentResult<()> {
        let component_ids: Vec<ComponentId> = self.components.keys().cloned().collect();
        
        for component_id in component_ids {
            self.unregister(component_id)?;
        }
        
        Ok(())
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics for the entire registry
#[derive(Debug, Clone)]
pub struct RegistryMetrics {
    pub total_components: usize,
    pub total_render_time: std::time::Duration,
    pub total_events_processed: u64,
    pub focused_component: Option<ComponentId>,
    pub component_metrics: HashMap<ComponentId, ComponentMetrics>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::components::traits::{UIComponent, RenderContext, UIEvent, EventResult};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    // Test component implementation
    #[derive(Debug)]
    struct TestComponent {
        id: ComponentId,
        name: String,
        state: ComponentState,
        metrics: ComponentMetrics,
        render_count: AtomicUsize,
    }
    
    impl TestComponent {
        fn new(name: String) -> Self {
            Self {
                id: ComponentId::new::<Self>(),
                name,
                state: ComponentState::Uninitialized,
                metrics: ComponentMetrics::default(),
                render_count: AtomicUsize::new(0),
            }
        }
    }
    
    impl UIComponent for TestComponent {
        fn component_id(&self) -> ComponentId {
            self.id
        }
        
        fn component_name(&self) -> &str {
            &self.name
        }
        
        fn state(&self) -> ComponentState {
            self.state
        }
        
        fn render(&mut self, _context: &mut RenderContext<'_>) -> ComponentResult<()> {
            self.render_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        
        fn metrics(&self) -> &ComponentMetrics {
            &self.metrics
        }
        
        fn set_state(&mut self, new_state: ComponentState) -> ComponentResult<()> {
            self.state = new_state;
            Ok(())
        }
    }
    
    #[test]
    fn test_component_registration() {
        let mut registry = ComponentRegistry::new();
        
        let component = TestComponent::new("test".to_string());
        let id = registry.register(component).unwrap();
        
        assert!(registry.get(id).is_some());
        assert_eq!(registry.get(id).unwrap().name(), "test");
    }
    
    #[test]
    fn test_focus_management() {
        let mut registry = ComponentRegistry::new();
        
        let component1 = TestComponent::new("component1".to_string());
        let component2 = TestComponent::new("component2".to_string());
        
        let id1 = registry.register(component1).unwrap();
        let id2 = registry.register(component2).unwrap();
        
        // Set focus to first component
        registry.set_focus(Some(id1)).unwrap();
        assert_eq!(registry.focused_component(), Some(id1));
        assert_eq!(registry.get(id1).unwrap().state(), ComponentState::Focused);
        
        // Set focus to second component
        registry.set_focus(Some(id2)).unwrap();
        assert_eq!(registry.focused_component(), Some(id2));
        assert_eq!(registry.get(id1).unwrap().state(), ComponentState::Ready);
        assert_eq!(registry.get(id2).unwrap().state(), ComponentState::Focused);
    }
    
    #[test]
    fn test_event_handling() {
        let mut registry = ComponentRegistry::new();
        
        let component = TestComponent::new("test".to_string());
        let id = registry.register(component).unwrap();
        registry.set_focus(Some(id)).unwrap();
        
        let event = UIEvent::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        ));
        
        let result = registry.handle_event(&event).unwrap();
        assert_eq!(result, EventResult::Ignored); // TestComponent ignores key events
    }
}