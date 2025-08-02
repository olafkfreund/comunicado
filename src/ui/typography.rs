/// Modern typography system for enhanced visual hierarchy
/// 
/// Provides consistent text styling, spacing, and information density
/// management across the TUI interface for better readability and professional appearance.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use crate::theme::Theme;

/// Typography scale following a modular approach
#[derive(Debug, Clone, Copy)]
pub enum TypographyLevel {
    /// Large headings for primary sections (H1)
    Heading1,
    /// Medium headings for subsections (H2)  
    Heading2,
    /// Small headings for sub-subsections (H3)
    Heading3,
    /// Regular body text
    Body,
    /// Smaller text for secondary information
    Caption,
    /// Tiny text for metadata and helper text
    Metadata,
    /// Monospace text for code, IDs, technical data
    Monospace,
    /// Labels for form fields and UI elements
    Label,
}

/// Information density levels for different UI modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InformationDensity {
    /// Compact - minimal spacing, more content visible
    Compact,
    /// Comfortable - balanced spacing (default)
    Comfortable, 
    /// Relaxed - maximum spacing, easier reading
    Relaxed,
}

/// Spacing system using consistent units
#[derive(Debug, Clone, Copy)]
pub struct Spacing {
    /// Base spacing unit (1 character width)
    pub unit: u16,
}

impl Spacing {
    /// Create new spacing system
    pub fn new() -> Self {
        Self { unit: 1 }
    }
    
    /// Get spacing for different densities
    pub fn get_spacing(&self, density: InformationDensity) -> SpacingValues {
        match density {
            InformationDensity::Compact => SpacingValues {
                none: 0,
                xs: self.unit,
                sm: self.unit,
                md: self.unit * 2,
                lg: self.unit * 2,
                xl: self.unit * 3,
            },
            InformationDensity::Comfortable => SpacingValues {
                none: 0,
                xs: self.unit,
                sm: self.unit * 2,
                md: self.unit * 3,
                lg: self.unit * 4,
                xl: self.unit * 6,
            },
            InformationDensity::Relaxed => SpacingValues {
                none: 0,
                xs: self.unit * 2,
                sm: self.unit * 3,
                md: self.unit * 4,
                lg: self.unit * 6,
                xl: self.unit * 8,
            },
        }
    }
}

/// Spacing values for different sizes
#[derive(Debug, Clone, Copy)]
pub struct SpacingValues {
    pub none: u16,
    pub xs: u16,
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
    pub xl: u16,
}

/// Typography system manager
#[derive(Clone)]
pub struct TypographySystem {
    spacing: Spacing,
    density: InformationDensity,
}

impl Default for TypographySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl TypographySystem {
    /// Create new typography system with default settings
    pub fn new() -> Self {
        Self {
            spacing: Spacing::new(),
            density: InformationDensity::Comfortable,
        }
    }
    
    /// Set information density
    pub fn with_density(mut self, density: InformationDensity) -> Self {
        self.density = density;
        self
    }
    
    /// Get current density
    pub fn density(&self) -> InformationDensity {
        self.density
    }
    
    /// Get spacing values for current density
    pub fn spacing(&self) -> SpacingValues {
        self.spacing.get_spacing(self.density)
    }
    
    /// Create styled text with proper typography level
    pub fn create_text<'a>(&self, content: &str, level: TypographyLevel, theme: &Theme) -> Text<'a> {
        let style = self.get_typography_style(level, theme);
        Text::from(vec![Line::from(vec![Span::styled(content.to_string(), style)])])
    }
    
    /// Create styled span with typography level
    pub fn create_span<'a>(&self, content: String, level: TypographyLevel, theme: &Theme) -> Span<'a> {
        let style = self.get_typography_style(level, theme);
        Span::styled(content, style)
    }
    
    /// Get style for typography level
    pub fn get_typography_style(&self, level: TypographyLevel, theme: &Theme) -> Style {
        match level {
            TypographyLevel::Heading1 => Style::default()
                .fg(theme.colors.palette.text_primary)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
                
            TypographyLevel::Heading2 => Style::default()
                .fg(theme.colors.palette.text_primary)
                .add_modifier(Modifier::BOLD),
                
            TypographyLevel::Heading3 => Style::default()
                .fg(theme.colors.palette.text_primary)
                .add_modifier(Modifier::BOLD),
                
            TypographyLevel::Body => Style::default()
                .fg(theme.colors.palette.text_primary),
                
            TypographyLevel::Caption => Style::default()
                .fg(theme.colors.palette.text_secondary),
                
            TypographyLevel::Metadata => Style::default()
                .fg(theme.colors.palette.text_muted)
                .add_modifier(Modifier::DIM),
                
            TypographyLevel::Monospace => Style::default()
                .fg(theme.colors.palette.text_primary)
                .bg(theme.colors.palette.surface),
                
            TypographyLevel::Label => Style::default()
                .fg(theme.colors.palette.text_secondary)
                .add_modifier(Modifier::DIM),
        }
    }
    
    /// Create a formatted header with consistent styling
    pub fn create_header<'a>(&self, title: &str, subtitle: Option<&str>, theme: &Theme) -> Vec<Line<'a>> {
        let mut lines = Vec::new();
        
        // Main title
        lines.push(Line::from(vec![
            self.create_span(title.to_string(), TypographyLevel::Heading2, theme)
        ]));
        
        // Optional subtitle
        if let Some(sub) = subtitle {
            lines.push(Line::from(vec![
                self.create_span(sub.to_string(), TypographyLevel::Caption, theme)
            ]));
        }
        
        // Add spacing line based on density
        match self.density {
            InformationDensity::Compact => {}, // No extra spacing
            InformationDensity::Comfortable => lines.push(Line::from("")),
            InformationDensity::Relaxed => {
                lines.push(Line::from(""));
                lines.push(Line::from(""));
            }
        }
        
        lines
    }
    
    /// Create a data row with consistent formatting
    pub fn create_data_row<'a>(&self, label: &str, value: &str, theme: &Theme) -> Line<'a> {
        let spacing = self.spacing();
        
        Line::from(vec![
            self.create_span(label.to_string(), TypographyLevel::Label, theme),
            Span::raw(" ".repeat(spacing.sm as usize)),
            self.create_span(value.to_string(), TypographyLevel::Body, theme),
        ])
    }
    
    /// Create a metadata line with proper styling
    pub fn create_metadata<'a>(&self, content: &str, theme: &Theme) -> Line<'a> {
        Line::from(vec![
            self.create_span(content.to_string(), TypographyLevel::Metadata, theme)
        ])
    }
    
    /// Create emphasized text
    pub fn create_emphasis<'a>(&self, content: &str, theme: &Theme) -> Span<'a> {
        Span::styled(
            content.to_string(),
            Style::default()
                .fg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD)
        )
    }
    
    /// Create monospace identifier text (for IDs, hashes, etc.)
    pub fn create_identifier<'a>(&self, content: &str, theme: &Theme) -> Span<'a> {
        self.create_span(content.to_string(), TypographyLevel::Monospace, theme)
    }
    
    /// Calculate optimal line height for current density
    pub fn line_height(&self) -> u16 {
        match self.density {
            InformationDensity::Compact => 1,
            InformationDensity::Comfortable => 1,
            InformationDensity::Relaxed => 2,
        }
    }
    
    /// Calculate padding for containers
    pub fn container_padding(&self) -> u16 {
        let spacing = self.spacing();
        spacing.sm
    }
    
    /// Calculate margin between sections
    pub fn section_margin(&self) -> u16 {
        let spacing = self.spacing();
        spacing.md
    }
}

/// Visual hierarchy helpers for common UI patterns
pub struct VisualHierarchy;

impl VisualHierarchy {
    /// Create a section divider
    pub fn section_divider<'a>(theme: &Theme) -> Line<'a> {
        Line::from(vec![
            Span::styled(
                "─".repeat(50),
                Style::default().fg(theme.colors.palette.border)
            )
        ])
    }
    
    /// Create a subtle section divider
    pub fn subtle_divider<'a>(theme: &Theme) -> Line<'a> {
        Line::from(vec![
            Span::styled(
                "·".repeat(20),
                Style::default().fg(theme.colors.palette.text_muted)
            )
        ])
    }
    
    /// Create a status indicator with color
    pub fn status_indicator<'a>(status: &str, color: Color) -> Span<'a> {
        Span::styled(
            format!("● {}", status),
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        )
    }
    
    /// Create a badge-style indicator
    pub fn badge<'a>(text: &str, theme: &Theme) -> Span<'a> {
        Span::styled(
            format!(" {} ", text),
            Style::default()
                .fg(theme.colors.palette.text_inverse)
                .bg(theme.colors.palette.accent)
                .add_modifier(Modifier::BOLD)
        )
    }
    
    /// Create a count indicator (like unread count)
    pub fn count_indicator<'a>(count: usize, theme: &Theme) -> Option<Span<'a>> {
        if count == 0 {
            None
        } else {
            Some(Span::styled(
                format!("({})", count),
                Style::default()
                    .fg(theme.colors.palette.accent)
                    .add_modifier(Modifier::BOLD)
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    #[test]
    fn test_typography_system_creation() {
        let typography = TypographySystem::new();
        assert_eq!(typography.density(), InformationDensity::Comfortable);
    }
    
    #[test]
    fn test_spacing_values() {
        let spacing = Spacing::new();
        let compact = spacing.get_spacing(InformationDensity::Compact);
        let comfortable = spacing.get_spacing(InformationDensity::Comfortable);
        let relaxed = spacing.get_spacing(InformationDensity::Relaxed);
        
        // Relaxed should have more spacing than compact
        assert!(relaxed.md > comfortable.md);
        assert!(comfortable.md > compact.md);
    }
    
    #[test]
    fn test_density_change() {
        let typography = TypographySystem::new()
            .with_density(InformationDensity::Compact);
        assert_eq!(typography.density(), InformationDensity::Compact);
    }
}