//! Mouse event handling system for the TUI
//!
//! This module provides comprehensive mouse event handling including:
//! - Coordinate mapping to UI components
//! - Click, drag, and scroll event processing
//! - Context menu integration
//! - Component-specific mouse interactions

use anyhow::Result;
use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
use ratatui::layout::Rect;
use tracing;

/// UI components that can receive mouse events
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UIComponent {
    /// Message list pane
    MessageList,
    /// Folder tree pane
    FolderTree,
    /// Email viewer pane
    EmailViewer,
    /// Calendar view
    Calendar,
    /// Status bar
    StatusBar,
    /// Command palette
    CommandPalette,
    /// Context menu
    ContextMenu,
    /// Modal dialog
    Modal,
    /// Empty/unknown area
    None,
}

/// Mouse event processor that handles coordinate mapping and event routing
pub struct MouseEventProcessor {
    /// Last known terminal size
    terminal_size: (u16, u16),
    /// UI component layout areas (updated by UI rendering)
    component_areas: std::collections::HashMap<UIComponent, Rect>,
    /// Currently hovered component
    hovered_component: Option<UIComponent>,
    /// Currently pressed mouse button and location
    pressed_button: Option<(MouseButton, u16, u16)>,
}

impl MouseEventProcessor {
    /// Create a new mouse event processor
    pub fn new() -> Self {
        Self {
            terminal_size: (80, 24), // Default fallback
            component_areas: std::collections::HashMap::new(),
            hovered_component: None,
            pressed_button: None,
        }
    }
    
    /// Update terminal size (called when terminal is resized)
    pub fn update_terminal_size(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
        tracing::debug!("Updated terminal size to {}x{}", width, height);
    }
    
    /// Update component areas (called during UI layout)
    pub fn update_component_areas(&mut self, areas: std::collections::HashMap<UIComponent, Rect>) {
        self.component_areas = areas;
        tracing::trace!("Updated component areas: {} components", self.component_areas.len());
    }
    
    /// Get the UI component at the given coordinates
    pub fn get_component_at_coordinates(&self, x: u16, y: u16) -> UIComponent {
        // Check each component area to see if coordinates fall within it
        for (component, area) in &self.component_areas {
            if x >= area.x && x < area.x + area.width 
               && y >= area.y && y < area.y + area.height {
                tracing::trace!("Coordinates ({}, {}) map to component: {:?}", x, y, component);
                return component.clone();
            }
        }
        
        tracing::trace!("Coordinates ({}, {}) don't map to any component", x, y);
        UIComponent::None
    }
    
    /// Get the relative coordinates within a component
    pub fn get_relative_coordinates(&self, component: &UIComponent, x: u16, y: u16) -> Option<(u16, u16)> {
        if let Some(area) = self.component_areas.get(component) {
            if x >= area.x && x < area.x + area.width 
               && y >= area.y && y < area.y + area.height {
                return Some((x - area.x, y - area.y));
            }
        }
        None
    }
    
    /// Process a mouse event and return the appropriate action
    pub async fn process_mouse_event(&mut self, mouse_event: MouseEvent) -> Result<MouseAction> {
        // Validate coordinates
        if mouse_event.column >= self.terminal_size.0 || mouse_event.row >= self.terminal_size.1 {
            tracing::debug!("Mouse coordinates out of bounds: ({}, {}) vs terminal ({}, {})",
                           mouse_event.column, mouse_event.row, self.terminal_size.0, self.terminal_size.1);
            return Ok(MouseAction::None);
        }
        
        let component = self.get_component_at_coordinates(mouse_event.column, mouse_event.row);
        let relative_coords = self.get_relative_coordinates(&component, mouse_event.column, mouse_event.row);
        
        // Process different mouse event types
        let action = match mouse_event.kind {
            MouseEventKind::Down(button) => {
                self.pressed_button = Some((button, mouse_event.column, mouse_event.row));
                self.handle_mouse_down(button, component, mouse_event.column, mouse_event.row, relative_coords)
            }
            MouseEventKind::Up(button) => {
                let action = self.handle_mouse_up(button, component, mouse_event.column, mouse_event.row, relative_coords);
                self.pressed_button = None;
                action
            }
            MouseEventKind::Drag(button) => {
                self.handle_mouse_drag(button, component, mouse_event.column, mouse_event.row, relative_coords)
            }
            MouseEventKind::Moved => {
                let old_hovered = self.hovered_component.clone();
                self.hovered_component = Some(component.clone());
                self.handle_mouse_move(component, mouse_event.column, mouse_event.row, relative_coords, old_hovered)
            }
            MouseEventKind::ScrollDown => {
                self.handle_mouse_scroll(component, mouse_event.column, mouse_event.row, relative_coords, false)
            }
            MouseEventKind::ScrollUp => {
                self.handle_mouse_scroll(component, mouse_event.column, mouse_event.row, relative_coords, true)
            }
            MouseEventKind::ScrollLeft => {
                // Horizontal scrolling - not commonly used in our TUI
                MouseAction::None
            }
            MouseEventKind::ScrollRight => {
                // Horizontal scrolling - not commonly used in our TUI
                MouseAction::None
            }
        };
        
        tracing::debug!("Mouse event {:?} at ({}, {}) -> {:?}", 
                        mouse_event.kind, mouse_event.column, mouse_event.row, action);
        
        Ok(action)
    }
    
    /// Handle mouse button down events
    fn handle_mouse_down(&self, button: MouseButton, component: UIComponent, x: u16, y: u16, relative_coords: Option<(u16, u16)>) -> MouseAction {
        match button {
            MouseButton::Left => {
                match component {
                    UIComponent::MessageList => {
                        if let Some((rel_x, rel_y)) = relative_coords {
                            MouseAction::SelectMessage { row: rel_y, column: rel_x }
                        } else {
                            MouseAction::None
                        }
                    }
                    UIComponent::FolderTree => {
                        if let Some((rel_x, rel_y)) = relative_coords {
                            MouseAction::SelectFolder { row: rel_y, column: rel_x }
                        } else {
                            MouseAction::None
                        }
                    }
                    UIComponent::EmailViewer => {
                        MouseAction::FocusEmailViewer
                    }
                    UIComponent::Calendar => {
                        if let Some((rel_x, rel_y)) = relative_coords {
                            MouseAction::SelectCalendarDate { row: rel_y, column: rel_x }
                        } else {
                            MouseAction::None
                        }
                    }
                    UIComponent::StatusBar => {
                        if let Some((rel_x, rel_y)) = relative_coords {
                            MouseAction::ClickStatusBar { row: rel_y, column: rel_x }
                        } else {
                            MouseAction::None
                        }
                    }
                    _ => MouseAction::ClearSelection
                }
            }
            MouseButton::Right => {
                match component {
                    UIComponent::MessageList => {
                        if let Some((rel_x, rel_y)) = relative_coords {
                            MouseAction::ShowEmailContextMenu { x, y, row: rel_y, column: rel_x }
                        } else {
                            MouseAction::None
                        }
                    }
                    UIComponent::FolderTree => {
                        if let Some((rel_x, rel_y)) = relative_coords {
                            MouseAction::ShowFolderContextMenu { x, y, row: rel_y, column: rel_x }
                        } else {
                            MouseAction::None
                        }
                    }
                    UIComponent::EmailViewer => {
                        MouseAction::ShowEmailContentContextMenu { x, y }
                    }
                    _ => MouseAction::ShowGeneralContextMenu { x, y }
                }
            }
            MouseButton::Middle => {
                match component {
                    UIComponent::MessageList => MouseAction::MiddleClickMessage,
                    UIComponent::FolderTree => MouseAction::MiddleClickFolder,
                    _ => MouseAction::None
                }
            }
        }
    }
    
    /// Handle mouse button up events
    fn handle_mouse_up(&self, _button: MouseButton, _component: UIComponent, _x: u16, _y: u16, _relative_coords: Option<(u16, u16)>) -> MouseAction {
        // Most click handling is done on mouse down for responsiveness
        // Mouse up is mainly used for drag end operations
        MouseAction::None
    }
    
    /// Handle mouse drag events
    fn handle_mouse_drag(&self, _button: MouseButton, component: UIComponent, _x: u16, _y: u16, relative_coords: Option<(u16, u16)>) -> MouseAction {
        match component {
            UIComponent::EmailViewer => {
                if let Some((rel_x, rel_y)) = relative_coords {
                    MouseAction::DragText { start_x: rel_x, start_y: rel_y }
                } else {
                    MouseAction::None
                }
            }
            _ => MouseAction::None
        }
    }
    
    /// Handle mouse move events (hover)
    fn handle_mouse_move(&self, component: UIComponent, _x: u16, _y: u16, relative_coords: Option<(u16, u16)>, _old_hovered: Option<UIComponent>) -> MouseAction {
        match component {
            UIComponent::MessageList => {
                if let Some((rel_x, rel_y)) = relative_coords {
                    MouseAction::HoverMessage { row: rel_y, column: rel_x }
                } else {
                    MouseAction::None
                }
            }
            UIComponent::FolderTree => {
                if let Some((rel_x, rel_y)) = relative_coords {
                    MouseAction::HoverFolder { row: rel_y, column: rel_x }
                } else {
                    MouseAction::None
                }
            }
            _ => MouseAction::None
        }
    }
    
    /// Handle mouse scroll events
    fn handle_mouse_scroll(&self, component: UIComponent, _x: u16, _y: u16, _relative_coords: Option<(u16, u16)>, scroll_up: bool) -> MouseAction {
        match component {
            UIComponent::MessageList => {
                if scroll_up {
                    MouseAction::ScrollMessageListUp
                } else {
                    MouseAction::ScrollMessageListDown
                }
            }
            UIComponent::EmailViewer => {
                if scroll_up {
                    MouseAction::ScrollEmailViewerUp
                } else {
                    MouseAction::ScrollEmailViewerDown
                }
            }
            UIComponent::FolderTree => {
                if scroll_up {
                    MouseAction::ScrollFolderTreeUp
                } else {
                    MouseAction::ScrollFolderTreeDown
                }
            }
            UIComponent::Calendar => {
                if scroll_up {
                    MouseAction::ScrollCalendarUp
                } else {
                    MouseAction::ScrollCalendarDown
                }
            }
            _ => MouseAction::None
        }
    }
}

impl Default for MouseEventProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions that can result from mouse events
#[derive(Debug, Clone)]
pub enum MouseAction {
    /// No action needed
    None,
    
    // Message List Actions
    /// Select a message at the given row
    SelectMessage { row: u16, column: u16 },
    /// Hover over a message
    HoverMessage { row: u16, column: u16 },
    /// Middle click on message (future: open in new tab)
    MiddleClickMessage,
    /// Scroll message list up
    ScrollMessageListUp,
    /// Scroll message list down
    ScrollMessageListDown,
    
    // Folder Tree Actions
    /// Select/expand a folder at the given row
    SelectFolder { row: u16, column: u16 },
    /// Hover over a folder
    HoverFolder { row: u16, column: u16 },
    /// Middle click on folder (future: open in new view)
    MiddleClickFolder,
    /// Scroll folder tree up
    ScrollFolderTreeUp,
    /// Scroll folder tree down
    ScrollFolderTreeDown,
    
    // Email Viewer Actions
    /// Focus the email viewer
    FocusEmailViewer,
    /// Scroll email viewer up
    ScrollEmailViewerUp,
    /// Scroll email viewer down
    ScrollEmailViewerDown,
    /// Start text drag selection
    DragText { start_x: u16, start_y: u16 },
    
    // Calendar Actions
    /// Select a calendar date
    SelectCalendarDate { row: u16, column: u16 },
    /// Scroll calendar up
    ScrollCalendarUp,
    /// Scroll calendar down
    ScrollCalendarDown,
    
    // Status Bar Actions
    /// Click on status bar element
    ClickStatusBar { row: u16, column: u16 },
    
    // Context Menu Actions
    /// Show email context menu
    ShowEmailContextMenu { x: u16, y: u16, row: u16, column: u16 },
    /// Show folder context menu
    ShowFolderContextMenu { x: u16, y: u16, row: u16, column: u16 },
    /// Show email content context menu
    ShowEmailContentContextMenu { x: u16, y: u16 },
    /// Show general context menu
    ShowGeneralContextMenu { x: u16, y: u16 },
    
    // General Actions
    /// Clear current selection
    ClearSelection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_mapping() {
        let mut processor = MouseEventProcessor::new();
        
        // Set up component areas
        let mut areas = std::collections::HashMap::new();
        areas.insert(UIComponent::MessageList, Rect::new(0, 0, 50, 20));
        areas.insert(UIComponent::FolderTree, Rect::new(50, 0, 30, 20));
        processor.update_component_areas(areas);
        
        // Test coordinate mapping
        assert_eq!(processor.get_component_at_coordinates(25, 10), UIComponent::MessageList);
        assert_eq!(processor.get_component_at_coordinates(60, 10), UIComponent::FolderTree);
        assert_eq!(processor.get_component_at_coordinates(90, 10), UIComponent::None);
    }
    
    #[test]
    fn test_relative_coordinates() {
        let mut processor = MouseEventProcessor::new();
        
        // Set up component area with offset
        let mut areas = std::collections::HashMap::new();
        areas.insert(UIComponent::MessageList, Rect::new(10, 5, 50, 20));
        processor.update_component_areas(areas);
        
        // Test relative coordinate calculation
        assert_eq!(processor.get_relative_coordinates(&UIComponent::MessageList, 15, 10), Some((5, 5)));
        assert_eq!(processor.get_relative_coordinates(&UIComponent::MessageList, 5, 5), None);
    }
}