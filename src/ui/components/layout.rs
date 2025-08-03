//! Layout Management System
//!
//! Provides declarative layout definitions and responsive layout calculations.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::collections::HashMap;
use thiserror::Error;

/// Result type for layout operations
pub type LayoutResult<T> = Result<T, LayoutError>;

/// Layout system errors
#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("Layout template not found: {template_id}")]
    TemplateNotFound { template_id: String },
    
    #[error("Invalid layout constraints: {reason}")]
    InvalidConstraints { reason: String },
    
    #[error("Responsive rule error: {rule_id}")]
    ResponsiveRuleError { rule_id: String },
    
    #[error("Layout calculation failed: {0}")]
    CalculationFailed(String),
}

/// Layout specification with responsive rules
#[derive(Debug, Clone)]
pub struct LayoutSpec {
    /// Unique identifier for this layout
    pub id: String,
    /// Base layout template
    pub template: LayoutTemplate,
    /// Responsive rules for different screen sizes
    pub responsive_rules: Vec<ResponsiveRule>,
    /// Minimum size requirements
    pub min_size: Option<(u16, u16)>,
    /// Whether this layout can be cached
    pub cacheable: bool,
}

impl LayoutSpec {
    /// Create a new layout specification
    pub fn new(id: String, template: LayoutTemplate) -> Self {
        Self {
            id,
            template,
            responsive_rules: Vec::new(),
            min_size: None,
            cacheable: true,
        }
    }
    
    /// Add a responsive rule
    pub fn with_responsive_rule(mut self, rule: ResponsiveRule) -> Self {
        self.responsive_rules.push(rule);
        self
    }
    
    /// Set minimum size requirements
    pub fn with_min_size(mut self, width: u16, height: u16) -> Self {
        self.min_size = Some((width, height));
        self
    }
    
    /// Set whether this layout can be cached
    pub fn with_caching(mut self, cacheable: bool) -> Self {
        self.cacheable = cacheable;
        self
    }
}

/// Layout template definitions
#[derive(Debug, Clone)]
pub enum LayoutTemplate {
    /// Three-pane horizontal layout (left, center, right)
    ThreePane {
        left: f32,
        center: f32,
        right: f32,
    },
    /// Two-pane horizontal layout (left, right)
    TwoPane {
        left: f32,
        right: f32,
    },
    /// Two-pane vertical layout (top, bottom)
    TwoPaneVertical {
        top: f32,
        bottom: f32,
    },
    /// Full screen layout
    FullScreen,
    /// Grid layout
    Grid {
        cols: usize,
        rows: usize,
        col_constraints: Vec<Constraint>,
        row_constraints: Vec<Constraint>,
    },
    /// Sidebar layout (sidebar + main content)
    Sidebar {
        sidebar_width: f32,
        sidebar_position: SidebarPosition,
    },
    /// Header/footer layout with main content
    HeaderFooter {
        header_height: u16,
        footer_height: u16,
    },
    /// Complex layout with multiple sections
    Complex {
        sections: Vec<LayoutSection>,
        direction: Direction,
    },
}

impl LayoutTemplate {
    /// Calculate layout areas for the given template
    pub fn calculate(&self, area: Rect) -> LayoutResult<Vec<Rect>> {
        match self {
            LayoutTemplate::ThreePane { left, center, right } => {
                self.validate_percentages(&[*left, *center, *right])?;
                
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage((*left * 100.0) as u16),
                        Constraint::Percentage((*center * 100.0) as u16),
                        Constraint::Percentage((*right * 100.0) as u16),
                    ])
                    .split(area);
                    
                Ok(chunks.to_vec())
            },
            
            LayoutTemplate::TwoPane { left, right } => {
                self.validate_percentages(&[*left, *right])?;
                
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage((*left * 100.0) as u16),
                        Constraint::Percentage((*right * 100.0) as u16),
                    ])
                    .split(area);
                    
                Ok(chunks.to_vec())
            },
            
            LayoutTemplate::TwoPaneVertical { top, bottom } => {
                self.validate_percentages(&[*top, *bottom])?;
                
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage((*top * 100.0) as u16),
                        Constraint::Percentage((*bottom * 100.0) as u16),
                    ])
                    .split(area);
                    
                Ok(chunks.to_vec())
            },
            
            LayoutTemplate::FullScreen => {
                Ok(vec![area])
            },
            
            LayoutTemplate::Grid { cols, rows, col_constraints, row_constraints } => {
                if col_constraints.len() != *cols || row_constraints.len() != *rows {
                    return Err(LayoutError::InvalidConstraints {
                        reason: "Grid constraints don't match grid dimensions".to_string(),
                    });
                }
                
                // First split into columns
                let col_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(col_constraints.clone())
                    .split(area);
                    
                // Then split each column into rows
                let mut result = Vec::new();
                for col_chunk in col_chunks.iter() {
                    let row_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(row_constraints.clone())
                        .split(*col_chunk);
                    result.extend(row_chunks.iter().cloned());
                }
                
                Ok(result)
            },
            
            LayoutTemplate::Sidebar { sidebar_width, sidebar_position } => {
                let sidebar_constraint = Constraint::Percentage((*sidebar_width * 100.0) as u16);
                let main_constraint = Constraint::Percentage(((1.0 - sidebar_width) * 100.0) as u16);
                
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(match sidebar_position {
                        SidebarPosition::Left => [sidebar_constraint, main_constraint],
                        SidebarPosition::Right => [main_constraint, sidebar_constraint],
                    })
                    .split(area);
                    
                Ok(chunks.to_vec())
            },
            
            LayoutTemplate::HeaderFooter { header_height, footer_height } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(*header_height),
                        Constraint::Min(1),
                        Constraint::Length(*footer_height),
                    ])
                    .split(area);
                    
                Ok(chunks.to_vec())
            },
            
            LayoutTemplate::Complex { sections, direction } => {
                let constraints: Vec<Constraint> = sections.iter()
                    .map(|section| section.constraint.clone())
                    .collect();
                    
                let chunks = Layout::default()
                    .direction(*direction)
                    .constraints(constraints)
                    .split(area);
                    
                Ok(chunks.to_vec())
            },
        }
    }
    
    /// Validate that percentages sum to approximately 1.0
    fn validate_percentages(&self, percentages: &[f32]) -> LayoutResult<()> {
        let sum: f32 = percentages.iter().sum();
        if (sum - 1.0).abs() > 0.01 {
            return Err(LayoutError::InvalidConstraints {
                reason: format!("Percentages sum to {:.2}, expected 1.0", sum),
            });
        }
        Ok(())
    }
}

/// Sidebar position options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPosition {
    Left,
    Right,
}

/// Layout section for complex layouts
#[derive(Debug, Clone)]
pub struct LayoutSection {
    pub id: String,
    pub constraint: Constraint,
    pub min_size: Option<(u16, u16)>,
}

impl LayoutSection {
    pub fn new(id: String, constraint: Constraint) -> Self {
        Self {
            id,
            constraint,
            min_size: None,
        }
    }
    
    pub fn with_min_size(mut self, width: u16, height: u16) -> Self {
        self.min_size = Some((width, height));
        self
    }
}

/// Responsive rule for adapting layouts to different screen sizes
#[derive(Debug, Clone)]
pub struct ResponsiveRule {
    /// Condition for when this rule applies
    pub condition: ResponsiveCondition,
    /// Template to use when condition is met
    pub template: LayoutTemplate,
    /// Priority (higher priority rules are checked first)
    pub priority: i32,
}

impl ResponsiveRule {
    /// Create a rule for when width is less than threshold
    pub fn when_width_lt(threshold: u16) -> ResponsiveRuleBuilder {
        ResponsiveRuleBuilder::new(ResponsiveCondition::WidthLessThan(threshold))
    }
    
    /// Create a rule for when height is less than threshold
    pub fn when_height_lt(threshold: u16) -> ResponsiveRuleBuilder {
        ResponsiveRuleBuilder::new(ResponsiveCondition::HeightLessThan(threshold))
    }
    
    /// Create a rule for when area is less than threshold
    pub fn when_area_lt(threshold: u32) -> ResponsiveRuleBuilder {
        ResponsiveRuleBuilder::new(ResponsiveCondition::AreaLessThan(threshold))
    }
    
    /// Check if this rule applies to the given area
    pub fn applies_to(&self, area: Rect) -> bool {
        self.condition.evaluate(area)
    }
}

/// Builder for responsive rules
pub struct ResponsiveRuleBuilder {
    condition: ResponsiveCondition,
    priority: i32,
}

impl ResponsiveRuleBuilder {
    fn new(condition: ResponsiveCondition) -> Self {
        Self {
            condition,
            priority: 0,
        }
    }
    
    /// Set rule priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// Complete the rule with a template
    pub fn use_template(self, template: LayoutTemplate) -> ResponsiveRule {
        ResponsiveRule {
            condition: self.condition,
            template,
            priority: self.priority,
        }
    }
    
    /// Use two-pane layout
    pub fn use_two_pane(self, left: f32, right: f32) -> ResponsiveRule {
        self.use_template(LayoutTemplate::TwoPane { left, right })
    }
    
    /// Use full screen layout
    pub fn use_full_screen(self) -> ResponsiveRule {
        self.use_template(LayoutTemplate::FullScreen)
    }
}

/// Conditions for responsive rules
#[derive(Debug, Clone)]
pub enum ResponsiveCondition {
    WidthLessThan(u16),
    WidthGreaterThan(u16),
    HeightLessThan(u16),
    HeightGreaterThan(u16),
    AreaLessThan(u32),
    AreaGreaterThan(u32),
    AspectRatioLessThan(f32),
    AspectRatioGreaterThan(f32),
    And(Box<ResponsiveCondition>, Box<ResponsiveCondition>),
    Or(Box<ResponsiveCondition>, Box<ResponsiveCondition>),
}

impl ResponsiveCondition {
    /// Evaluate condition against area
    pub fn evaluate(&self, area: Rect) -> bool {
        match self {
            ResponsiveCondition::WidthLessThan(threshold) => area.width < *threshold,
            ResponsiveCondition::WidthGreaterThan(threshold) => area.width > *threshold,
            ResponsiveCondition::HeightLessThan(threshold) => area.height < *threshold,
            ResponsiveCondition::HeightGreaterThan(threshold) => area.height > *threshold,
            ResponsiveCondition::AreaLessThan(threshold) => {
                (area.width as u32 * area.height as u32) < *threshold
            },
            ResponsiveCondition::AreaGreaterThan(threshold) => {
                (area.width as u32 * area.height as u32) > *threshold
            },
            ResponsiveCondition::AspectRatioLessThan(ratio) => {
                (area.width as f32 / area.height as f32) < *ratio
            },
            ResponsiveCondition::AspectRatioGreaterThan(ratio) => {
                (area.width as f32 / area.height as f32) > *ratio
            },
            ResponsiveCondition::And(left, right) => {
                left.evaluate(area) && right.evaluate(area)
            },
            ResponsiveCondition::Or(left, right) => {
                left.evaluate(area) || right.evaluate(area)
            },
        }
    }
}

/// Layout manager for calculating and caching layouts
pub struct LayoutManager {
    /// Registered layout specifications
    layouts: HashMap<String, LayoutSpec>,
    /// Layout calculation cache
    cache: HashMap<LayoutCacheKey, Vec<Rect>>,
    /// Cache hit statistics
    cache_hits: u64,
    cache_misses: u64,
}

impl LayoutManager {
    /// Create a new layout manager
    pub fn new() -> Self {
        Self {
            layouts: HashMap::new(),
            cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }
    
    /// Register a layout specification
    pub fn register_layout(&mut self, spec: LayoutSpec) {
        self.layouts.insert(spec.id.clone(), spec);
    }
    
    /// Calculate layout for the given specification and area
    pub fn calculate_layout(&mut self, layout_id: &str, area: Rect) -> LayoutResult<Vec<Rect>> {
        let spec = self.layouts.get(layout_id)
            .ok_or_else(|| LayoutError::TemplateNotFound {
                template_id: layout_id.to_string(),
            })?;
        
        // Check minimum size requirements
        if let Some((min_width, min_height)) = spec.min_size {
            if area.width < min_width || area.height < min_height {
                return Err(LayoutError::CalculationFailed(
                    format!("Area too small: {}x{}, minimum: {}x{}", 
                           area.width, area.height, min_width, min_height)
                ));
            }
        }
        
        // Check cache if layout is cacheable
        if spec.cacheable {
            let cache_key = LayoutCacheKey {
                layout_id: layout_id.to_string(),
                area,
            };
            
            if let Some(cached_result) = self.cache.get(&cache_key) {
                self.cache_hits += 1;
                return Ok(cached_result.clone());
            }
        }
        
        // Find applicable template (check responsive rules)
        let template = self.find_applicable_template(spec, area)?;
        
        // Calculate layout
        let result = template.calculate(area)?;
        
        // Cache result if cacheable
        if spec.cacheable {
            let cache_key = LayoutCacheKey {
                layout_id: layout_id.to_string(),
                area,
            };
            self.cache.insert(cache_key, result.clone());
        }
        
        self.cache_misses += 1;
        Ok(result)
    }
    
    /// Find the applicable template based on responsive rules
    fn find_applicable_template(&self, spec: &LayoutSpec, area: Rect) -> LayoutResult<LayoutTemplate> {
        // Sort responsive rules by priority (highest first)
        let mut rules = spec.responsive_rules.clone();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Find first matching rule
        for rule in &rules {
            if rule.applies_to(area) {
                return Ok(rule.template.clone());
            }
        }
        
        // Use base template if no rules match
        Ok(spec.template.clone())
    }
    
    /// Clear layout cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (u64, u64, f64) {
        let total_requests = self.cache_hits + self.cache_misses;
        let hit_rate = if total_requests > 0 {
            self.cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };
        
        (self.cache_hits, self.cache_misses, hit_rate)
    }
    
    /// Get all registered layout IDs
    pub fn layout_ids(&self) -> Vec<&str> {
        self.layouts.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        let mut manager = Self::new();
        
        // Register common layouts
        manager.register_common_layouts();
        
        manager
    }
}

impl LayoutManager {
    /// Register commonly used layouts
    fn register_common_layouts(&mut self) {
        // Email three-pane layout
        let email_layout = LayoutSpec::new(
            "email_main".to_string(),
            LayoutTemplate::ThreePane {
                left: 0.25,
                center: 0.35,
                right: 0.40,
            },
        )
        .with_responsive_rule(
            ResponsiveRule::when_width_lt(120)
                .use_two_pane(0.4, 0.6)
        )
        .with_responsive_rule(
            ResponsiveRule::when_width_lt(80)
                .use_full_screen()
        )
        .with_min_size(60, 20);
        
        self.register_layout(email_layout);
        
        // Calendar layout
        let calendar_layout = LayoutSpec::new(
            "calendar_main".to_string(),
            LayoutTemplate::HeaderFooter {
                header_height: 3,
                footer_height: 1,
            },
        )
        .with_responsive_rule(
            ResponsiveRule::when_height_lt(15)
                .use_full_screen()
        );
        
        self.register_layout(calendar_layout);
        
        // Full screen layout
        let fullscreen_layout = LayoutSpec::new(
            "fullscreen".to_string(),
            LayoutTemplate::FullScreen,
        );
        
        self.register_layout(fullscreen_layout);
        
        // Sidebar layout
        let sidebar_layout = LayoutSpec::new(
            "sidebar_main".to_string(),
            LayoutTemplate::Sidebar {
                sidebar_width: 0.25,
                sidebar_position: SidebarPosition::Left,
            },
        )
        .with_responsive_rule(
            ResponsiveRule::when_width_lt(100)
                .use_full_screen()
        );
        
        self.register_layout(sidebar_layout);
    }
}

/// Cache key for layout calculations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LayoutCacheKey {
    layout_id: String,
    area: Rect,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_three_pane_layout() {
        let template = LayoutTemplate::ThreePane {
            left: 0.25,
            center: 0.35,
            right: 0.40,
        };
        
        let area = Rect::new(0, 0, 100, 50);
        let result = template.calculate(area).unwrap();
        
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].width, 25); // 25% of 100
        assert_eq!(result[1].width, 35); // 35% of 100
        assert_eq!(result[2].width, 40); // 40% of 100
    }
    
    #[test]
    fn test_responsive_rule() {
        let rule = ResponsiveRule::when_width_lt(80)
            .use_two_pane(0.5, 0.5);
        
        let small_area = Rect::new(0, 0, 70, 30);
        let large_area = Rect::new(0, 0, 120, 30);
        
        assert!(rule.applies_to(small_area));
        assert!(!rule.applies_to(large_area));
    }
    
    #[test]
    fn test_layout_manager() {
        let mut manager = LayoutManager::new();
        
        let spec = LayoutSpec::new(
            "test".to_string(),
            LayoutTemplate::TwoPane { left: 0.6, right: 0.4 },
        );
        
        manager.register_layout(spec);
        
        let area = Rect::new(0, 0, 100, 50);
        let result = manager.calculate_layout("test", area).unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].width, 60);
        assert_eq!(result[1].width, 40);
    }
    
    #[test]
    fn test_layout_cache() {
        let mut manager = LayoutManager::new();
        
        let spec = LayoutSpec::new(
            "cached_test".to_string(),
            LayoutTemplate::TwoPane { left: 0.5, right: 0.5 },
        );
        
        manager.register_layout(spec);
        
        let area = Rect::new(0, 0, 100, 50);
        
        // First calculation (cache miss)
        let _result1 = manager.calculate_layout("cached_test", area).unwrap();
        let (hits1, misses1, _) = manager.cache_stats();
        
        // Second calculation (cache hit)
        let _result2 = manager.calculate_layout("cached_test", area).unwrap();
        let (hits2, misses2, _) = manager.cache_stats();
        
        assert_eq!(hits1, 0);
        assert_eq!(misses1, 1);
        assert_eq!(hits2, 1);
        assert_eq!(misses2, 1);
    }
}