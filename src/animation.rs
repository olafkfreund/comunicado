use anyhow::{anyhow, Result};
use image::{DynamicImage, ImageOutputFormat, codecs::gif::GifDecoder};
use image::AnimationDecoder;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};
use tokio::time::sleep;
use crate::images::{ImageManager, TerminalProtocol};

/// Animation frame with timing information
#[derive(Debug, Clone)]
pub struct AnimationFrame {
    /// The image data for this frame
    pub image: DynamicImage,
    /// Delay before showing next frame (in milliseconds)
    pub delay_ms: u32,
    /// Frame index in the animation
    pub index: u32,
}

/// Animation metadata
#[derive(Debug, Clone)]
pub struct AnimationInfo {
    /// Total number of frames
    pub frame_count: u32,
    /// Total animation duration in milliseconds
    pub duration_ms: u32,
    /// Whether the animation should loop
    pub loops: bool,
    /// Original image dimensions
    pub width: u32,
    pub height: u32,
}

/// Cached animation data
#[derive(Debug, Clone)]
struct CachedAnimation {
    frames: Vec<AnimationFrame>,
    info: AnimationInfo,
    encoded_frames: Option<Vec<String>>, // Pre-encoded frames for terminal display
}

/// Animation playback state
#[derive(Debug)]
struct AnimationState {
    current_frame: u32,
    #[allow(dead_code)]
    start_time: Instant,
    last_frame_time: Instant,
    is_playing: bool,
    loop_count: u32,
}

/// Animation manager for GIF and other animated image formats
pub struct AnimationManager {
    image_manager: Arc<ImageManager>,
    cache: Arc<RwLock<HashMap<String, CachedAnimation>>>,
    animations: Arc<RwLock<HashMap<String, AnimationState>>>,
    #[allow(dead_code)]
    frame_sender: Option<mpsc::UnboundedSender<AnimationCommand>>,
    max_frame_rate: u32, // Maximum FPS to prevent terminal overload
}

/// Commands for animation control
#[derive(Debug, Clone)]
pub enum AnimationCommand {
    Play { id: String, loops: bool },
    Pause { id: String },
    Stop { id: String },
    Seek { id: String, frame: u32 },
    SetSpeed { id: String, speed_multiplier: f32 },
}

/// Animation playback result
#[derive(Debug, Clone)]
pub enum AnimationResult {
    Frame { id: String, frame_data: String, frame_index: u32 },
    Finished { id: String },
    Error { id: String, error: String },
}

impl AnimationManager {
    /// Create a new animation manager
    pub fn new(image_manager: Arc<ImageManager>) -> Self {
        Self {
            image_manager,
            cache: Arc::new(RwLock::new(HashMap::new())),
            animations: Arc::new(RwLock::new(HashMap::new())),
            frame_sender: None,
            max_frame_rate: 30, // 30 FPS max
        }
    }
    
    /// Set maximum frame rate to prevent terminal overload
    pub fn set_max_frame_rate(&mut self, fps: u32) {
        self.max_frame_rate = fps.max(1).min(60); // Clamp between 1-60 FPS
    }
    
    /// Check if terminal supports animations
    pub fn supports_animations(&self) -> bool {
        // Animations are supported if the terminal supports images
        self.image_manager.supports_images()
    }
    
    /// Get supported terminal protocol
    pub fn protocol(&self) -> TerminalProtocol {
        self.image_manager.protocol()
    }
    
    /// Load and cache a GIF animation from URL
    pub async fn load_gif_from_url(&self, url: &str) -> Result<String> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if cache.contains_key(url) {
                return Ok(url.to_string()); // Return the ID for the animation
            }
        }
        
        // Download GIF data
        let response = reqwest::get(url).await?;
        let bytes = response.bytes().await?;
        
        // Load animation from bytes
        self.load_gif_from_bytes(&bytes, Some(url)).await
    }
    
    /// Load and cache a GIF animation from base64 data
    pub async fn load_gif_from_base64(&self, data: &str) -> Result<String> {
        // Create cache key from hash of data
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let cache_key = format!("gif_base64_{:x}", hasher.finalize());
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if cache.contains_key(&cache_key) {
                return Ok(cache_key);
            }
        }
        
        // Decode base64 data
        use base64::{Engine as _, engine::general_purpose};
        let bytes = general_purpose::STANDARD.decode(data)?;
        
        self.load_gif_from_bytes(&bytes, Some(&cache_key)).await
    }
    
    /// Load and cache a GIF animation from raw bytes
    pub async fn load_gif_from_bytes(&self, data: &[u8], cache_key: Option<&str>) -> Result<String> {
        let key = cache_key.map(|k| k.to_string()).unwrap_or_else(|| {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(data);
            format!("gif_bytes_{:x}", hasher.finalize())
        });
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if cache.contains_key(&key) {
                return Ok(key);
            }
        }
        
        // Decode GIF animation
        let cursor = Cursor::new(data);
        let decoder = GifDecoder::new(cursor)?;
        
        let frames_iter = decoder.into_frames();
        let mut frames = Vec::new();
        let mut total_duration = 0u32;
        
        for (index, frame_result) in frames_iter.enumerate() {
            let frame = frame_result?;
            let delay = frame.delay();
            
            // Convert delay to milliseconds
            let delay_ms = delay.numer_denom_ms().0 as u32 * 1000 / delay.numer_denom_ms().1 as u32;
            
            // Apply minimum frame delay to prevent too-fast animations
            let delay_ms = delay_ms.max(50); // Minimum 50ms (20 FPS max)
            
            total_duration += delay_ms;
            
            let animation_frame = AnimationFrame {
                image: DynamicImage::ImageRgba8(frame.into_buffer()),
                delay_ms,
                index: index as u32,
            };
            
            frames.push(animation_frame);
        }
        
        if frames.is_empty() {
            return Err(anyhow!("No frames found in GIF"));
        }
        
        // Get dimensions from first frame
        let (width, height) = (frames[0].image.width(), frames[0].image.height());
        
        let info = AnimationInfo {
            frame_count: frames.len() as u32,
            duration_ms: total_duration,
            loops: true, // GIFs typically loop
            width,
            height,
        };
        
        // Pre-encode frames for terminal display if supported
        let encoded_frames = if self.supports_animations() {
            let mut encoded = Vec::new();
            for frame in &frames {
                match self.encode_frame_for_terminal(&frame.image).await {
                    Ok(encoded_frame) => encoded.push(encoded_frame),
                    Err(e) => {
                        tracing::warn!("Failed to encode frame {}: {}", frame.index, e);
                        // Use placeholder for failed frames
                        encoded.push(self.generate_frame_placeholder(frame.index));
                    }
                }
            }
            Some(encoded)
        } else {
            None
        };
        
        // Cache the animation
        {
            let mut cache = self.cache.write().await;
            cache.insert(key.clone(), CachedAnimation {
                frames,
                info: info.clone(),
                encoded_frames,
            });
        }
        
        tracing::info!("Loaded GIF animation: {} frames, {}ms duration", 
                      info.frame_count, info.duration_ms);
        
        Ok(key)
    }
    
    /// Start playing an animation
    pub async fn play_animation(&self, animation_id: &str, loops: bool) -> Result<mpsc::UnboundedReceiver<AnimationResult>> {
        // Check if animation exists in cache
        let animation = {
            let cache = self.cache.read().await;
            cache.get(animation_id).cloned()
                .ok_or_else(|| anyhow!("Animation not found: {}", animation_id))?
        };
        
        // Create playback state
        {
            let mut animations = self.animations.write().await;
            animations.insert(animation_id.to_string(), AnimationState {
                current_frame: 0,
                start_time: Instant::now(),
                last_frame_time: Instant::now(),
                is_playing: true,
                loop_count: 0,
            });
        }
        
        // Create channel for animation frames
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // Spawn animation playback task
        let animation_id = animation_id.to_string();
        let animations = self.animations.clone();
        let frame_rate_limit = Duration::from_millis(1000 / self.max_frame_rate as u64);
        
        tokio::spawn(async move {
            let mut _last_frame_time = Instant::now();
            
            loop {
                // Check if animation is still playing
                let (current_frame, should_continue) = {
                    let animations_guard = animations.read().await;
                    if let Some(state) = animations_guard.get(&animation_id) {
                        if !state.is_playing {
                            break;
                        }
                        (state.current_frame, true)
                    } else {
                        (0, false)
                    }
                };
                
                if !should_continue {
                    break;
                }
                
                // Get current frame data
                if let Some(encoded_frames) = &animation.encoded_frames {
                    if let Some(frame_data) = encoded_frames.get(current_frame as usize) {
                        // Send frame to subscriber
                        if sender.send(AnimationResult::Frame {
                            id: animation_id.clone(),
                            frame_data: frame_data.clone(),
                            frame_index: current_frame,
                        }).is_err() {
                            break; // Receiver dropped
                        }
                    }
                }
                
                // Calculate delay for this frame
                let frame_delay = if let Some(frame) = animation.frames.get(current_frame as usize) {
                    Duration::from_millis(frame.delay_ms as u64)
                } else {
                    Duration::from_millis(100) // Default delay
                };
                
                // Apply frame rate limiting
                let actual_delay = frame_delay.max(frame_rate_limit);
                sleep(actual_delay).await;
                
                // Update animation state
                {
                    let mut animations_guard = animations.write().await;
                    if let Some(state) = animations_guard.get_mut(&animation_id) {
                        state.current_frame = (state.current_frame + 1) % animation.info.frame_count;
                        state.last_frame_time = Instant::now();
                        
                        // Check if we completed a loop
                        if state.current_frame == 0 {
                            state.loop_count += 1;
                            if !loops {
                                state.is_playing = false;
                                let _ = sender.send(AnimationResult::Finished {
                                    id: animation_id.clone(),
                                });
                                break;
                            }
                        }
                    }
                }
                
                _last_frame_time = Instant::now();
            }
            
            // Clean up animation state
            {
                let mut animations_guard = animations.write().await;
                animations_guard.remove(&animation_id);
            }
        });
        
        Ok(receiver)
    }
    
    /// Stop playing an animation
    pub async fn stop_animation(&self, animation_id: &str) -> Result<()> {
        let mut animations = self.animations.write().await;
        if let Some(state) = animations.get_mut(animation_id) {
            state.is_playing = false;
        }
        Ok(())
    }
    
    /// Pause an animation
    pub async fn pause_animation(&self, animation_id: &str) -> Result<()> {
        let mut animations = self.animations.write().await;
        if let Some(state) = animations.get_mut(animation_id) {
            state.is_playing = false;
        }
        Ok(())
    }
    
    /// Resume a paused animation
    pub async fn resume_animation(&self, animation_id: &str) -> Result<()> {
        let mut animations = self.animations.write().await;
        if let Some(state) = animations.get_mut(animation_id) {
            state.is_playing = true;
        }
        Ok(())
    }
    
    /// Get animation information
    pub async fn get_animation_info(&self, animation_id: &str) -> Result<AnimationInfo> {
        let cache = self.cache.read().await;
        cache.get(animation_id)
            .map(|anim| anim.info.clone())
            .ok_or_else(|| anyhow!("Animation not found: {}", animation_id))
    }
    
    /// Get current animation state
    pub async fn get_animation_state(&self, animation_id: &str) -> Option<(u32, bool)> {
        let animations = self.animations.read().await;
        animations.get(animation_id)
            .map(|state| (state.current_frame, state.is_playing))
    }
    
    /// Encode a single frame for terminal display
    async fn encode_frame_for_terminal(&self, image: &DynamicImage) -> Result<String> {
        // Use the image manager to encode the frame
        // This is a bit of a hack since ImageManager doesn't expose encode_for_terminal
        // In a real implementation, we'd refactor to share the encoding logic
        
        match self.image_manager.protocol() {
            TerminalProtocol::Kitty => self.encode_kitty_frame(image),
            TerminalProtocol::Sixel => self.encode_sixel_frame(image),
            TerminalProtocol::None => Ok(self.generate_frame_placeholder(0)),
        }
    }
    
    /// Encode frame using Kitty graphics protocol
    fn encode_kitty_frame(&self, img: &DynamicImage) -> Result<String> {
        // Convert image to PNG for transmission
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);
        
        img.write_to(&mut cursor, ImageOutputFormat::Png)?;
        
        // Encode as base64
        use base64::{Engine as _, engine::general_purpose};
        let encoded = general_purpose::STANDARD.encode(&buffer);
        
        // Create Kitty graphics command with delete previous image
        let kitty_command = format!(
            "\x1b_Ga=T,f=100,d=A;{}\x1b\\",
            encoded
        );
        
        Ok(kitty_command)
    }
    
    /// Encode frame using Sixel graphics protocol
    fn encode_sixel_frame(&self, img: &DynamicImage) -> Result<String> {
        // Simple Sixel encoding (would use proper library in production)
        let _rgb_img = img.to_rgb8();
        let mut sixel = String::new();
        
        // Sixel header with clear previous image
        sixel.push_str("\x1b[2J\x1bPq"); // Clear screen + Sixel start
        
        // Simple color mapping and data (simplified implementation)
        sixel.push_str("#0;2;0;0;0");   // Define black color
        sixel.push_str("#1;2;100;100;100"); // Define white color
        
        // Add some sample data (in real implementation, would properly encode pixels)
        sixel.push_str("~"); // Sample sixel data
        
        // Sixel terminator
        sixel.push_str("\x1b\\");
        
        Ok(sixel)
    }
    
    /// Generate placeholder for frame when encoding fails
    fn generate_frame_placeholder(&self, frame_index: u32) -> String {
        format!("┌─ GIF Frame {} ─┐\n│   [ANIMATION]   │\n│     LOADING     │\n└─────────────────┘", frame_index)
    }
    
    /// Clear animation cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        
        let mut animations = self.animations.write().await;
        animations.clear();
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let animations = self.animations.read().await;
        (cache.len(), animations.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_animation_manager_creation() {
        let image_manager = Arc::new(crate::images::ImageManager::new().unwrap());
        let animation_manager = AnimationManager::new(image_manager);
        
        assert!(animation_manager.max_frame_rate > 0);
    }
    
    #[tokio::test]
    async fn test_cache_operations() {
        let image_manager = Arc::new(crate::images::ImageManager::new().unwrap());
        let animation_manager = AnimationManager::new(image_manager);
        
        let (cached_count, active_count) = animation_manager.get_cache_stats().await;
        assert_eq!(cached_count, 0);
        assert_eq!(active_count, 0);
        
        animation_manager.clear_cache().await;
        
        let (cached_count, active_count) = animation_manager.get_cache_stats().await;
        assert_eq!(cached_count, 0);
        assert_eq!(active_count, 0);
    }
}