use anyhow::{anyhow, Result};
use image::{DynamicImage, ImageFormat, ImageOutputFormat};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

/// Supported terminal graphics protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalProtocol {
    /// Kitty graphics protocol
    Kitty,
    /// Sixel graphics protocol  
    Sixel,
    /// No graphics support
    None,
}

/// Image cache entry
#[derive(Debug, Clone)]
struct CachedImage {
    data: Vec<u8>,
    #[allow(dead_code)] // Used for metadata and potential future features
    format: ImageFormat,
    #[allow(dead_code)] // Used for metadata and potential future features
    width: u32,
    #[allow(dead_code)] // Used for metadata and potential future features
    height: u32,
    encoded_data: Option<String>, // Pre-encoded for terminal display
}

/// Image display manager for terminal graphics
#[derive(Clone)]
pub struct ImageManager {
    cache: Arc<RwLock<HashMap<String, CachedImage>>>,
    protocol: TerminalProtocol,
    max_width: u32,
    max_height: u32,
    #[allow(dead_code)] // Directory for persistent cache storage
    cache_dir: PathBuf,
}

impl ImageManager {
    /// Create a new image manager
    pub fn new() -> Result<Self> {
        let protocol = Self::detect_terminal_protocol();
        let cache_dir = Self::get_cache_directory()?;

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            protocol,
            max_width: 80,  // Default terminal width in characters
            max_height: 24, // Default terminal height in characters
            cache_dir,
        })
    }

    /// Set maximum display dimensions (in terminal characters)
    pub fn set_max_dimensions(&mut self, width: u32, height: u32) {
        self.max_width = width;
        self.max_height = height;
    }

    /// Get the supported terminal protocol
    pub fn protocol(&self) -> TerminalProtocol {
        self.protocol
    }

    /// Check if terminal supports image display
    pub fn supports_images(&self) -> bool {
        !matches!(self.protocol, TerminalProtocol::None)
    }

    /// Download and cache an image from URL
    pub async fn load_image_from_url(&self, url: &str) -> Result<String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(url) {
                if let Some(ref encoded) = cached.encoded_data {
                    return Ok(encoded.clone());
                }
            }
        }

        // Download image
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;

        // Load and process image
        let img = image::load_from_memory(&bytes)?;
        let format = image::guess_format(&bytes)?;

        // Resize image to fit terminal
        let resized = self.resize_for_terminal(&img);

        // Encode for terminal display
        let encoded = self.encode_for_terminal(&resized, format)?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                url.to_string(),
                CachedImage {
                    data: bytes.to_vec(),
                    format,
                    width: resized.width(),
                    height: resized.height(),
                    encoded_data: Some(encoded.clone()),
                },
            );
        }

        Ok(encoded)
    }

    /// Load an image from embedded base64 data
    pub async fn load_image_from_base64(
        &self,
        data: &str,
        mime_type: Option<&str>,
    ) -> Result<String> {
        // Create cache key from hash of data
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let cache_key = format!("base64_{:x}", hasher.finalize());

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if let Some(ref encoded) = cached.encoded_data {
                    return Ok(encoded.clone());
                }
            }
        }

        // Decode base64 data
        use base64::{engine::general_purpose, Engine as _};
        let bytes = general_purpose::STANDARD.decode(data)?;

        // Determine format from MIME type or guess from data
        let format = if let Some(mime) = mime_type {
            Self::format_from_mime_type(mime)?
        } else {
            image::guess_format(&bytes)?
        };

        // Load and process image
        let img = image::load_from_memory(&bytes)?;

        // Resize image to fit terminal
        let resized = self.resize_for_terminal(&img);

        // Encode for terminal display
        let encoded = self.encode_for_terminal(&resized, format)?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                CachedImage {
                    data: bytes,
                    format,
                    width: resized.width(),
                    height: resized.height(),
                    encoded_data: Some(encoded.clone()),
                },
            );
        }

        Ok(encoded)
    }

    /// Load an image from raw bytes data
    pub async fn load_image_from_bytes(
        &self,
        data: &[u8],
        mime_type: Option<&str>,
    ) -> Result<String> {
        // Create cache key from hash of data
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let cache_key = format!("bytes_{:x}", hasher.finalize());

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if let Some(ref encoded) = cached.encoded_data {
                    return Ok(encoded.clone());
                }
            }
        }

        // Determine format from MIME type or guess from data
        let format = if let Some(mime) = mime_type {
            Self::format_from_mime_type(mime)?
        } else {
            image::guess_format(data)?
        };

        // Load and process image
        let img = image::load_from_memory(data)?;

        // Resize image to fit terminal
        let resized = self.resize_for_terminal(&img);

        // Encode for terminal display
        let encoded = self.encode_for_terminal(&resized, format)?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(
                cache_key,
                CachedImage {
                    data: data.to_vec(),
                    format,
                    width: resized.width(),
                    height: resized.height(),
                    encoded_data: Some(encoded.clone()),
                },
            );
        }

        Ok(encoded)
    }

    /// Generate a placeholder for unsupported images
    pub fn generate_placeholder(
        &self,
        alt_text: Option<&str>,
        width: Option<u32>,
        height: Option<u32>,
    ) -> String {
        let text = alt_text.unwrap_or("Image");
        let w = width.unwrap_or(20).min(self.max_width);
        let h = height.unwrap_or(5).min(self.max_height);

        // Create a simple ASCII art placeholder
        let mut placeholder = String::new();

        // Top border
        placeholder.push('┌');
        for _ in 0..(w - 2) {
            placeholder.push('─');
        }
        placeholder.push_str("┐\n");

        // Middle rows with text
        for row in 0..h.saturating_sub(2) {
            placeholder.push('│');
            if row == h / 2 - 1 && text.len() <= (w - 2) as usize {
                // Center the text
                let padding = ((w - 2) as usize).saturating_sub(text.len()) / 2;
                for _ in 0..padding {
                    placeholder.push(' ');
                }
                placeholder.push_str(text);
                for _ in 0..((w - 2) as usize - padding - text.len()) {
                    placeholder.push(' ');
                }
            } else {
                for _ in 0..(w - 2) {
                    placeholder.push(' ');
                }
            }
            placeholder.push_str("│\n");
        }

        // Bottom border
        placeholder.push('└');
        for _ in 0..(w - 2) {
            placeholder.push('─');
        }
        placeholder.push('┘');

        placeholder
    }

    /// Detect which terminal graphics protocol is supported
    fn detect_terminal_protocol() -> TerminalProtocol {
        // Check environment variables that indicate terminal capabilities
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("kitty") {
                return TerminalProtocol::Kitty;
            }
            if term.contains("xterm") {
                // Check for Sixel support
                if Self::test_sixel_support() {
                    return TerminalProtocol::Sixel;
                }
            }
        }

        // Check TERM_PROGRAM for kitty
        if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
            if term_program == "kitty" {
                return TerminalProtocol::Kitty;
            }
        }

        // Check for Kitty graphics protocol support by querying terminal
        if Self::test_kitty_support() {
            return TerminalProtocol::Kitty;
        }

        TerminalProtocol::None
    }

    /// Test if terminal supports Kitty graphics protocol
    fn test_kitty_support() -> bool {
        // This is a simplified test - in production you'd query the terminal
        // For now, we'll just check if we're in a kitty terminal
        std::env::var("KITTY_WINDOW_ID").is_ok()
    }

    /// Test if terminal supports Sixel graphics
    fn test_sixel_support() -> bool {
        // This is a simplified test - in production you'd query terminal capabilities
        // Common terminals that support Sixel: xterm (with Sixel enabled), foot, wezterm
        if let Ok(term) = std::env::var("TERM") {
            return term.contains("xterm") || term.contains("foot") || term.contains("wezterm");
        }
        false
    }

    /// Get cache directory for images
    fn get_cache_directory() -> Result<PathBuf> {
        let cache_dir = dirs::cache_dir()
            .or_else(|| dirs::data_dir())
            .unwrap_or_else(|| PathBuf::from("."))
            .join("comunicado")
            .join("images");
        Ok(cache_dir)
    }

    /// Resize image to fit within terminal dimensions
    fn resize_for_terminal(&self, img: &DynamicImage) -> DynamicImage {
        let (orig_width, orig_height) = (img.width(), img.height());

        // Calculate maximum pixel dimensions based on terminal size
        // Assume each character is roughly 8x16 pixels (common for terminal fonts)
        let max_pixel_width = self.max_width * 8;
        let max_pixel_height = self.max_height * 16;

        // Calculate scale factor to fit within bounds
        let scale_w = max_pixel_width as f32 / orig_width as f32;
        let scale_h = max_pixel_height as f32 / orig_height as f32;
        let scale = scale_w.min(scale_h).min(1.0); // Don't upscale

        if scale < 1.0 {
            let new_width = (orig_width as f32 * scale) as u32;
            let new_height = (orig_height as f32 * scale) as u32;
            img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
        } else {
            img.clone()
        }
    }

    /// Encode image for terminal display based on protocol
    fn encode_for_terminal(&self, img: &DynamicImage, format: ImageFormat) -> Result<String> {
        match self.protocol {
            TerminalProtocol::Kitty => self.encode_kitty(img, format),
            TerminalProtocol::Sixel => self.encode_sixel(img),
            TerminalProtocol::None => Ok(self.generate_placeholder(None, Some(20), Some(5))),
        }
    }

    /// Encode image using Kitty graphics protocol
    fn encode_kitty(&self, img: &DynamicImage, _format: ImageFormat) -> Result<String> {
        // Convert image to PNG for transmission
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        img.write_to(&mut cursor, ImageOutputFormat::Png)?;

        // Encode as base64
        use base64::{engine::general_purpose, Engine as _};
        let encoded = general_purpose::STANDARD.encode(&buffer);

        // Create Kitty graphics command
        // Format: \x1b_G<payload>\x1b\\
        // where payload includes: a=T,f=100,<base64_data>
        // a=T means transmit and display
        // f=100 means PNG format
        let kitty_command = format!("\x1b_Ga=T,f=100;{}\x1b\\", encoded);

        Ok(kitty_command)
    }

    /// Encode image using Sixel graphics protocol
    fn encode_sixel(&self, img: &DynamicImage) -> Result<String> {
        // Convert to RGB8 for Sixel encoding
        let rgb_img = img.to_rgb8();
        let (width, height) = rgb_img.dimensions();

        // This is a simplified Sixel encoder
        // In production, you'd use a proper Sixel library
        let mut sixel = String::new();

        // Sixel header
        sixel.push_str("\x1bPq");

        // Define some basic colors (simplified)
        for i in 0..16 {
            let r = (i * 16) % 256;
            let g = (i * 32) % 256;
            let b = (i * 64) % 256;
            sixel.push_str(&format!(
                "#{};2;{};{};{}",
                i,
                r * 100 / 255,
                g * 100 / 255,
                b * 100 / 255
            ));
        }

        // Convert pixels to sixel data (very simplified)
        // This would normally involve color quantization and sixel band encoding
        for y in (0..height).step_by(6) {
            for x in 0..width {
                let pixel = rgb_img.get_pixel(x, y);
                let color_index =
                    ((pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16) / 3 / 16) % 16;
                sixel.push_str(&format!("#{}", color_index));
                sixel.push('?'); // Simplified sixel character
            }
            sixel.push('-'); // New line in sixel
        }

        // Sixel terminator
        sixel.push_str("\x1b\\");

        Ok(sixel)
    }

    /// Convert MIME type to ImageFormat
    fn format_from_mime_type(mime: &str) -> Result<ImageFormat> {
        match mime.to_lowercase().as_str() {
            "image/jpeg" | "image/jpg" => Ok(ImageFormat::Jpeg),
            "image/png" => Ok(ImageFormat::Png),
            "image/gif" => Ok(ImageFormat::Gif),
            "image/webp" => Ok(ImageFormat::WebP),
            "image/bmp" => Ok(ImageFormat::Bmp),
            "image/tiff" => Ok(ImageFormat::Tiff),
            _ => Err(anyhow!("Unsupported image format: {}", mime)),
        }
    }

    /// Clear the image cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let total_size: usize = cache.values().map(|img| img.data.len()).sum();
        (total_entries, total_size)
    }
}

impl Default for ImageManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default ImageManager")
    }
}

/// Extract images from HTML content
pub fn extract_images_from_html(html: &str) -> Vec<ImageReference> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let img_selector = Selector::parse("img").unwrap();
    let mut images = Vec::new();

    for element in document.select(&img_selector) {
        let src = element.value().attr("src");
        let alt = element.value().attr("alt");
        let width = element.value().attr("width");
        let height = element.value().attr("height");

        if let Some(src) = src {
            images.push(ImageReference {
                src: src.to_string(),
                alt: alt.map(|s| s.to_string()),
                width: width.and_then(|w| w.parse().ok()),
                height: height.and_then(|h| h.parse().ok()),
            });
        }
    }

    images
}

/// Reference to an image found in content
#[derive(Debug, Clone)]
pub struct ImageReference {
    pub src: String,
    pub alt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl ImageReference {
    /// Check if this is a data URL (base64 embedded image)
    pub fn is_data_url(&self) -> bool {
        self.src.starts_with("data:")
    }

    /// Extract base64 data and MIME type from data URL
    pub fn parse_data_url(&self) -> Option<(String, Option<String>)> {
        if !self.is_data_url() {
            return None;
        }

        // Format: data:[<mediatype>][;base64],<data>
        let url_str = self.src.strip_prefix("data:")?;
        let parts: Vec<&str> = url_str.splitn(2, ',').collect();

        if parts.len() != 2 {
            return None;
        }

        let header = parts[0];
        let data = parts[1];

        // Parse media type
        let mime_type = if header.contains(";base64") {
            let mime_part = header.strip_suffix(";base64")?;
            if mime_part.is_empty() {
                None
            } else {
                Some(mime_part.to_string())
            }
        } else {
            // Not base64 encoded, we only support base64 for now
            return None;
        };

        Some((data.to_string(), mime_type))
    }

    /// Check if this is a valid HTTP/HTTPS URL
    pub fn is_http_url(&self) -> bool {
        self.src.starts_with("http://") || self.src.starts_with("https://")
    }

    /// Validate and parse as URL
    pub fn parse_url(&self) -> Result<Url> {
        Url::parse(&self.src).map_err(|e| anyhow!("Invalid URL: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_reference_data_url() {
        let img_ref = ImageReference {
            src: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string(),
            alt: Some("test".to_string()),
            width: None,
            height: None,
        };

        assert!(img_ref.is_data_url());
        assert!(!img_ref.is_http_url());

        let (data, mime) = img_ref.parse_data_url().unwrap();
        assert_eq!(mime, Some("image/png".to_string()));
        assert!(!data.is_empty());
    }

    #[test]
    fn test_image_reference_http_url() {
        let img_ref = ImageReference {
            src: "https://example.com/image.jpg".to_string(),
            alt: None,
            width: Some(100),
            height: Some(200),
        };

        assert!(!img_ref.is_data_url());
        assert!(img_ref.is_http_url());
        assert!(img_ref.parse_url().is_ok());
    }

    #[test]
    fn test_extract_images_from_html() {
        let html = r#"
            <html>
                <body>
                    <img src="https://example.com/image1.jpg" alt="Test Image 1" width="100" height="200">
                    <img src="data:image/png;base64,iVBORw0KGgo=" alt="Embedded">
                    <img src="/relative/path.png">
                </body>
            </html>
        "#;

        let images = extract_images_from_html(html);
        assert_eq!(images.len(), 3);

        assert_eq!(images[0].src, "https://example.com/image1.jpg");
        assert_eq!(images[0].alt, Some("Test Image 1".to_string()));
        assert_eq!(images[0].width, Some(100));
        assert_eq!(images[0].height, Some(200));

        assert!(images[1].is_data_url());
        assert_eq!(images[2].src, "/relative/path.png");
    }
}
