use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
};

pub struct AppLayout {
    folder_width: u16,
    message_width_ratio: u16,
}

impl AppLayout {
    pub fn new() -> Self {
        Self {
            folder_width: 25,
            message_width_ratio: 40,
        }
    }

    pub fn calculate_layout(&self, area: Rect) -> Vec<Rect> {
        // First, split vertically to reserve space for status bar
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),      // Main content area
                Constraint::Length(3),   // Status bar (fixed height)
            ])
            .split(area);

        let main_area = vertical_chunks[0];
        
        // Split the main area horizontally: [Left Panel | Messages | Content]
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.folder_width),           // Fixed width for left panel
                Constraint::Percentage(self.message_width_ratio), // Percentage for messages
                Constraint::Min(30),                             // Remaining space for content
            ])
            .split(main_area);

        // Split the left panel vertically: [Account Switcher | Folders]
        let left_panel_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),   // Fixed height for account switcher
                Constraint::Min(5),      // Remaining space for folders
            ])
            .split(horizontal_chunks[0]);

        // Return all chunks: [account_switcher, folder, message, content, status_bar]
        vec![
            left_panel_chunks[0],  // Account switcher
            left_panel_chunks[1],  // Folder tree
            horizontal_chunks[1],  // Message list
            horizontal_chunks[2],  // Content preview
            vertical_chunks[1],    // Status bar
        ]
    }

    pub fn set_folder_width(&mut self, width: u16) {
        self.folder_width = width;
    }

    pub fn set_message_width_ratio(&mut self, ratio: u16) {
        self.message_width_ratio = ratio;
    }
}

impl Default for AppLayout {
    fn default() -> Self {
        Self::new()
    }
}