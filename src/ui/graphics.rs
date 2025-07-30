//! Terminal graphics protocol support for image and animation rendering
//!
//! This module provides support for various terminal graphics protocols including:
//! - Kitty graphics protocol
//! - Sixel protocol
//! - iTerm2 inline images
//! - Fallback text representations

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use image::{DynamicImage, ImageFormat, RgbaImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::io::{Cursor, Write};
use thiserror::Error;

/// Graphics rendering errors
#[derive(Error, Debug)]
pub enum GraphicsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Image processing error: {0}")]
    ImageProcessing(String),
    
    #[error("Protocol not supported: {0:?}")]
    ProtocolNotSupported(GraphicsProtocol),
    
    #[error("Terminal detection failed")]
    TerminalDetectionFailed,
    
    #[error("Encoding error: {0}")]
    Encoding(String),
    
    #[error("Size validation failed: {width}x{height} exceeds limit")]
    SizeLimitExceeded { width: u32, height: u32 },
}

pub type GraphicsResult<T> = Result<T, GraphicsError>;

/// Supported terminal graphics protocols
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GraphicsProtocol {
    /// Kitty graphics protocol (most advanced)
    Kitty,
    /// Sixel graphics protocol (widely supported)
    Sixel,
    /// iTerm2 inline images
    ITerm2,
    /// WezTerm image protocol
    WezTerm,
    /// Fallback to Unicode block elements
    Unicode,
    /// No graphics support
    None,
}

impl GraphicsProtocol {
    /// Detect supported graphics protocols
    pub fn detect_supported() -> Vec<GraphicsProtocol> {
        let mut supported = Vec::new();
        
        // Check environment variables and terminal capabilities
        if let Ok(term) = env::var("TERM") {
            match term.as_str() {
                t if t.contains("kitty") => supported.push(GraphicsProtocol::Kitty),
                t if t.contains("xterm") => {
                    // Check for Sixel support
                    if supports_sixel() {
                        supported.push(GraphicsProtocol::Sixel);
                    }
                },
                _ => {}
            }
        }
        
        // Check for specific terminal programs
        if let Ok(term_program) = env::var("TERM_PROGRAM") {
            match term_program.as_str() {
                "iTerm.app" => supported.push(GraphicsProtocol::ITerm2),
                "WezTerm" => supported.push(GraphicsProtocol::WezTerm),
                _ => {}
            }
        }
        
        // Check for Kitty terminal
        if env::var("KITTY_WINDOW_ID").is_ok() {
            supported.push(GraphicsProtocol::Kitty);
        }
        
        // Always support Unicode fallback
        supported.push(GraphicsProtocol::Unicode);
        
        if supported.is_empty() {
            supported.push(GraphicsProtocol::None);
        }
        
        supported
    }
    
    /// Get the best available protocol
    pub fn best_available() -> GraphicsProtocol {
        let supported = Self::detect_supported();
        
        // Priority order: Kitty > Sixel > iTerm2 > WezTerm > Unicode > None
        for protocol in &[
            GraphicsProtocol::Kitty,
            GraphicsProtocol::Sixel,
            GraphicsProtocol::ITerm2,
            GraphicsProtocol::WezTerm,
            GraphicsProtocol::Unicode,
        ] {
            if supported.contains(protocol) {
                return *protocol;
            }
        }
        
        GraphicsProtocol::None
    }
    
    /// Check if protocol supports animation
    pub fn supports_animation(&self) -> bool {
        matches!(self, 
            GraphicsProtocol::Kitty | 
            GraphicsProtocol::Sixel |
            GraphicsProtocol::WezTerm
        )
    }
    
    /// Get maximum supported image dimensions
    pub fn max_dimensions(&self) -> (u32, u32) {
        match self {
            GraphicsProtocol::Kitty => (4096, 4096),
            GraphicsProtocol::Sixel => (1000, 1000),
            GraphicsProtocol::ITerm2 => (2048, 2048),
            GraphicsProtocol::WezTerm => (2048, 2048),
            GraphicsProtocol::Unicode => (200, 100),
            GraphicsProtocol::None => (0, 0),
        }
    }
}

/// Image rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    pub protocol: GraphicsProtocol,
    pub max_width: u32,
    pub max_height: u32,
    pub preserve_aspect_ratio: bool,
    pub quality: u8, // 1-100
    pub dithering: bool,
    pub transparency: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        let protocol = GraphicsProtocol::best_available();
        let (max_width, max_height) = protocol.max_dimensions();
        
        Self {
            protocol,
            max_width,
            max_height,
            preserve_aspect_ratio: true,
            quality: 85,
            dithering: true,
            transparency: true,
        }
    }
}

/// Terminal image renderer
pub struct ImageRenderer {
    config: RenderConfig,
    #[allow(dead_code)]
    supported_protocols: Vec<GraphicsProtocol>,
}

impl ImageRenderer {
    /// Create a new image renderer
    pub fn new(config: RenderConfig) -> Self {
        Self {
            supported_protocols: GraphicsProtocol::detect_supported(),
            config,
        }
    }
    
    /// Create with automatic protocol detection
    pub fn auto() -> Self {
        Self::new(RenderConfig::default())
    }
    
    /// Update rendering configuration
    pub fn update_config(&mut self, config: RenderConfig) {
        self.config = config;
    }
    
    /// Get current protocol
    pub fn protocol(&self) -> GraphicsProtocol {
        self.config.protocol
    }
    
    /// Check if current protocol supports animation
    pub fn supports_animation(&self) -> bool {
        self.config.protocol.supports_animation()
    }
    
    /// Render image to terminal escape sequences
    pub async fn render_image(
        &self,
        image: &DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> GraphicsResult<Vec<u8>> {
        // Validate input size
        let (img_width, img_height) = image.dimensions();
        let (max_width, max_height) = self.config.protocol.max_dimensions();
        
        if img_width > max_width || img_height > max_height {
            return Err(GraphicsError::SizeLimitExceeded { 
                width: img_width, 
                height: img_height 
            });
        }
        
        // Resize image if needed
        let resized_image = self.resize_image(image, target_width, target_height)?;
        
        // Render based on protocol
        match self.config.protocol {
            GraphicsProtocol::Kitty => self.render_kitty(&resized_image).await,
            GraphicsProtocol::Sixel => self.render_sixel(&resized_image).await,
            GraphicsProtocol::ITerm2 => self.render_iterm2(&resized_image).await,
            GraphicsProtocol::WezTerm => self.render_wezterm(&resized_image).await,
            GraphicsProtocol::Unicode => self.render_unicode(&resized_image).await,
            GraphicsProtocol::None => Ok(vec![]),
        }
    }
    
    /// Resize image maintaining aspect ratio if configured
    fn resize_image(
        &self,
        image: &DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> GraphicsResult<DynamicImage> {
        let (img_width, img_height) = image.dimensions();
        
        // Skip if no resize needed
        if img_width <= target_width && img_height <= target_height {
            return Ok(image.clone());
        }
        
        let (new_width, new_height) = if self.config.preserve_aspect_ratio {
            let width_ratio = target_width as f32 / img_width as f32;
            let height_ratio = target_height as f32 / img_height as f32;
            let ratio = width_ratio.min(height_ratio);
            
            ((img_width as f32 * ratio) as u32, (img_height as f32 * ratio) as u32)
        } else {
            (target_width, target_height)
        };
        
        let filter = if self.config.quality > 80 {
            image::imageops::FilterType::Lanczos3
        } else if self.config.quality > 50 {
            image::imageops::FilterType::CatmullRom
        } else {
            image::imageops::FilterType::Triangle
        };
        
        Ok(image.resize(new_width, new_height, filter))
    }
    
    /// Render using Kitty graphics protocol
    async fn render_kitty(&self, image: &DynamicImage) -> GraphicsResult<Vec<u8>> {
        let rgba_image = image.to_rgba8();
        let (width, height) = rgba_image.dimensions();
        
        // Encode as PNG for best quality
        let mut png_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut png_data);
            image.write_to(&mut cursor, ImageFormat::Png)
                .map_err(|e| GraphicsError::ImageProcessing(e.to_string()))?;
        }
        
        // Encode as base64
        let base64_data = BASE64_STANDARD.encode(&png_data);
        
        // Create Kitty graphics command
        let mut output = Vec::new();
        
        // Start transmission with format and size information
        write!(output, "\x1b_Gf=100,s={},v={},m=1;", width, height)
            .map_err(|e| GraphicsError::Io(e))?;
        
        // Split base64 data into chunks (max 4096 bytes per chunk)
        const CHUNK_SIZE: usize = 4096;
        let chunks: Vec<&str> = base64_data.as_bytes()
            .chunks(CHUNK_SIZE)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect();
        
        for (i, chunk) in chunks.iter().enumerate() {
            if i == chunks.len() - 1 {
                // Last chunk
                write!(output, "{}\x1b\\", chunk)
                    .map_err(|e| GraphicsError::Io(e))?;
            } else {
                // Intermediate chunk
                write!(output, "\x1b_Gm=1;{}\x1b\\", chunk)
                    .map_err(|e| GraphicsError::Io(e))?;
            }
        }
        
        Ok(output)
    }
    
    /// Render using Sixel protocol
    async fn render_sixel(&self, image: &DynamicImage) -> GraphicsResult<Vec<u8>> {
        // Convert to RGB (Sixel doesn't support alpha)
        let rgb_image = image.to_rgb8();
        let (width, height) = rgb_image.dimensions();
        
        let mut output = Vec::new();
        
        // Start Sixel sequence
        write!(output, "\x1bPq").map_err(|e| GraphicsError::Io(e))?;
        
        // Simple Sixel encoding (this is a basic implementation)
        // For production, consider using a dedicated Sixel library
        
        // Define color palette (simplified to 256 colors)
        let mut colors = HashMap::new();
        let mut color_index = 0;
        
        // Process image in 6-pixel high bands (Sixel limitation)
        for y in (0..height).step_by(6) {
            for x in 0..width {
                let mut sixel_data = 0u8;
                
                // Process 6 pixels vertically
                for dy in 0..6 {
                    if y + dy < height {
                        let pixel = rgb_image.get_pixel(x, y + dy);
                        let rgb = (pixel[0], pixel[1], pixel[2]);
                        
                        // Get or assign color index
                        let color_idx = *colors.entry(rgb).or_insert_with(|| {
                            let idx = color_index;
                            color_index += 1;
                            idx
                        });
                        
                        // Set bit in sixel data
                        if color_idx < 256 { // Limit to 256 colors
                            sixel_data |= 1 << dy;
                        }
                    }
                }
                
                // Encode sixel character (add 63 to make printable)
                output.push(sixel_data + 63);
            }
            
            // End of line
            write!(output, "$").map_err(|e| GraphicsError::Io(e))?;
        }
        
        // End Sixel sequence
        write!(output, "\x1b\\").map_err(|e| GraphicsError::Io(e))?;
        
        Ok(output)
    }
    
    /// Render using iTerm2 inline images
    async fn render_iterm2(&self, image: &DynamicImage) -> GraphicsResult<Vec<u8>> {
        let (width, height) = image.dimensions();
        
        // Encode as PNG
        let mut png_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut png_data);
            image.write_to(&mut cursor, ImageFormat::Png)
                .map_err(|e| GraphicsError::ImageProcessing(e.to_string()))?;
        }
        
        // Encode as base64
        let base64_data = BASE64_STANDARD.encode(&png_data);
        
        // Create iTerm2 escape sequence
        let output = format!(
            "\x1b]1337;File=inline=1;width={}px;height={}px:{}\x07",
            width, height, base64_data
        );
        
        Ok(output.into_bytes())
    }
    
    /// Render using WezTerm image protocol
    async fn render_wezterm(&self, image: &DynamicImage) -> GraphicsResult<Vec<u8>> {
        // WezTerm uses a similar protocol to iTerm2 but with different escape codes
        let (width, height) = image.dimensions();
        
        // Encode as PNG
        let mut png_data = Vec::new();
        {
            let mut cursor = Cursor::new(&mut png_data);
            image.write_to(&mut cursor, ImageFormat::Png)
                .map_err(|e| GraphicsError::ImageProcessing(e.to_string()))?;
        }
        
        // Encode as base64
        let base64_data = BASE64_STANDARD.encode(&png_data);
        
        // Create WezTerm escape sequence
        let output = format!(
            "\x1b]1337;File=inline=1;size={};width={}px;height={}px:{}\x07",
            png_data.len(), width, height, base64_data
        );
        
        Ok(output.into_bytes())
    }
    
    /// Render using Unicode block elements (fallback)
    async fn render_unicode(&self, image: &DynamicImage) -> GraphicsResult<Vec<u8>> {
        let rgba_image = image.to_rgba8();
        let (width, height) = rgba_image.dimensions();
        
        let mut output = Vec::new();
        
        // Use Unicode block elements to approximate the image
        // Process in 2x4 pixel blocks using Unicode characters like ▀▄█
        for y in (0..height).step_by(4) {
            for x in (0..width).step_by(2) {
                let block_char = self.get_unicode_block(&rgba_image, x, y);
                output.extend_from_slice(block_char.as_bytes());
            }
            output.push(b'\n');
        }
        
        Ok(output)
    }
    
    /// Get appropriate Unicode block character for a 2x4 pixel area
    fn get_unicode_block(&self, image: &RgbaImage, x: u32, y: u32) -> &'static str {
        // Sample the 2x4 area and determine which Unicode block character best represents it
        let mut brightness_sum = 0.0;
        let mut pixel_count = 0;
        
        for dy in 0..4 {
            for dx in 0..2 {
                if x + dx < image.width() && y + dy < image.height() {
                    let pixel = image.get_pixel(x + dx, y + dy);
                    // Calculate brightness using standard RGB to grayscale formula
                    let brightness = (0.299 * pixel[0] as f32 + 
                                    0.587 * pixel[1] as f32 + 
                                    0.114 * pixel[2] as f32) / 255.0;
                    brightness_sum += brightness;
                    pixel_count += 1;
                }
            }
        }
        
        if pixel_count == 0 {
            return " ";
        }
        
        let avg_brightness = brightness_sum / pixel_count as f32;
        
        // Map brightness to Unicode block characters
        match avg_brightness {
            b if b > 0.875 => "█", // Full block
            b if b > 0.750 => "▉", // 7/8 block
            b if b > 0.625 => "▊", // 3/4 block
            b if b > 0.500 => "▋", // 5/8 block
            b if b > 0.375 => "▌", // 1/2 block
            b if b > 0.250 => "▍", // 3/8 block
            b if b > 0.125 => "▎", // 1/4 block
            b if b > 0.000 => "▏", // 1/8 block
            _ => " ",              // Empty
        }
    }
}

/// Check if terminal supports Sixel graphics
fn supports_sixel() -> bool {
    // Check for Sixel support through terminfo or environment variables
    if let Ok(colorterm) = env::var("COLORTERM") {
        if colorterm.contains("sixel") {
            return true;
        }
    }
    
    // Check TERM variable
    if let Ok(term) = env::var("TERM") {
        // Some terminals that support Sixel
        if term.contains("xterm") || term.contains("mlterm") || term.contains("mintty") {
            // This is a simplification - in reality you'd query terminal capabilities
            return true;
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{RgbaImage, Rgba};

    #[test]
    fn test_graphics_protocol_detection() {
        let protocols = GraphicsProtocol::detect_supported();
        assert!(!protocols.is_empty());
        assert!(protocols.contains(&GraphicsProtocol::Unicode) || 
                protocols.contains(&GraphicsProtocol::None));
    }

    #[test]
    fn test_graphics_protocol_best_available() {
        let best = GraphicsProtocol::best_available();
        assert!(matches!(best, 
            GraphicsProtocol::Kitty | 
            GraphicsProtocol::Sixel | 
            GraphicsProtocol::ITerm2 | 
            GraphicsProtocol::WezTerm | 
            GraphicsProtocol::Unicode | 
            GraphicsProtocol::None
        ));
    }

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();
        assert!(config.preserve_aspect_ratio);
        assert_eq!(config.quality, 85);
        assert!(config.dithering);
    }

    #[tokio::test]
    async fn test_image_renderer_creation() {
        let renderer = ImageRenderer::auto();
        assert!(matches!(renderer.protocol(), 
            GraphicsProtocol::Kitty | 
            GraphicsProtocol::Sixel | 
            GraphicsProtocol::ITerm2 | 
            GraphicsProtocol::WezTerm | 
            GraphicsProtocol::Unicode | 
            GraphicsProtocol::None
        ));
    }

    #[tokio::test]
    async fn test_unicode_rendering() {
        let mut image = RgbaImage::new(4, 4);
        
        // Create a simple test pattern
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let brightness = if (x + y) % 2 == 0 { 255 } else { 0 };
            *pixel = Rgba([brightness, brightness, brightness, 255]);
        }
        
        let dynamic_image = DynamicImage::ImageRgba8(image);
        let renderer = ImageRenderer::new(RenderConfig {
            protocol: GraphicsProtocol::Unicode,
            ..Default::default()
        });
        
        let result = renderer.render_image(&dynamic_image, 4, 4).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert!(!output.is_empty());
        
        // Should contain Unicode block characters
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.chars().any(|c| "█▉▊▋▌▍▎▏ ".contains(c)));
    }

    #[test]
    fn test_protocol_max_dimensions() {
        assert_eq!(GraphicsProtocol::Kitty.max_dimensions(), (4096, 4096));
        assert_eq!(GraphicsProtocol::Sixel.max_dimensions(), (1000, 1000));
        assert_eq!(GraphicsProtocol::Unicode.max_dimensions(), (200, 100));
    }

    #[test]
    fn test_protocol_animation_support() {
        assert!(GraphicsProtocol::Kitty.supports_animation());
        assert!(GraphicsProtocol::Sixel.supports_animation());
        assert!(!GraphicsProtocol::ITerm2.supports_animation());
        assert!(!GraphicsProtocol::Unicode.supports_animation());
    }
}