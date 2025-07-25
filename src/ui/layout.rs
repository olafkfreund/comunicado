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
        // Create horizontal layout: [Folders | Messages | Content]
        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(self.folder_width),           // Fixed width for folders
                Constraint::Percentage(self.message_width_ratio), // Percentage for messages
                Constraint::Min(30),                             // Remaining space for content
            ])
            .split(area);

        horizontal_chunks.to_vec()
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