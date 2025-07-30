# GIF Animation Support in Comunicado

## Overview

Comunicado now includes basic support for GIF animations in compatible terminals. This feature allows users to view animated GIFs directly in their terminal email client without needing to open external applications.

## Features

### Core Animation System

- **GIF Decoding**: Full support for GIF animation parsing using the `image` crate's `GifDecoder`
- **Frame Management**: Efficient frame extraction with timing information and caching
- **Terminal Protocols**: Support for both Kitty and Sixel graphics protocols
- **Frame Rate Limiting**: Prevents terminal overload with configurable maximum frame rates
- **Memory Caching**: Intelligent caching system for loaded animations to improve performance

### Integration Points

- **Email Content Preview**: Automatic detection and playback of GIF animations in HTML emails
- **Attachment Viewer**: Support for animated GIF attachments
- **Image Manager Integration**: Seamless integration with existing image display system

## Supported Terminals

### Full Support
- **Kitty**: Native graphics protocol with optimal performance
- **Foot**: Sixel graphics protocol support
- **Wezterm**: Sixel graphics protocol support
- **Modern XTerm**: With Sixel enabled

### Fallback Support
- **Other Terminals**: ASCII art placeholders when graphics protocols are unavailable

## Technical Implementation

### Core Components

1. **AnimationManager** (`src/animation.rs`)
   - Manages GIF loading, caching, and playback
   - Handles frame timing and terminal protocol encoding
   - Provides async API for animation control

2. **ContentPreview Integration** (`src/ui/content_preview.rs`)
   - Automatic GIF detection in HTML content
   - Animation lifecycle management
   - Frame rendering integration

3. **ImageManager Enhancement** (`src/images.rs`)
   - Added Clone trait support for animation integration
   - Shared terminal protocol detection

### Key Features

#### Animation Loading
```rust
// Load from URL
let animation_id = animation_manager.load_gif_from_url("https://example.com/image.gif").await?;

// Load from base64 data (email attachments)
let animation_id = animation_manager.load_gif_from_base64(base64_data).await?;

// Load from raw bytes
let animation_id = animation_manager.load_gif_from_bytes(&bytes, None).await?;
```

#### Animation Playback
```rust
// Start playing animation
let mut receiver = animation_manager.play_animation(&animation_id, true).await?;

// Handle animation frames
while let Some(result) = receiver.recv().await {
    match result {
        AnimationResult::Frame { frame_data, .. } => {
            // Display frame in terminal
        }
        AnimationResult::Finished { .. } => break,
        AnimationResult::Error { error, .. } => {
            eprintln!("Animation error: {}", error);
            break;
        }
    }
}
```

#### Animation Control
```rust
// Pause animation
animation_manager.pause_animation(&animation_id).await?;

// Resume animation
animation_manager.resume_animation(&animation_id).await?;

// Stop animation
animation_manager.stop_animation(&animation_id).await?;
```

## Performance Considerations

### Frame Rate Limiting
- Default maximum: 30 FPS to prevent terminal overload
- Configurable via `set_max_frame_rate()`
- Minimum frame delay: 50ms (20 FPS max)

### Memory Management
- Intelligent caching with SHA-256 based keys
- Pre-encoded frames for terminal display
- Automatic cleanup of stopped animations

### Network Efficiency
- Cached downloads for repeated GIF URLs
- Base64 support for embedded email GIFs
- Lazy loading of animation frames

## Configuration

### Terminal Protocol Detection
The system automatically detects supported graphics protocols:

```rust
// Check if animations are supported
if animation_manager.supports_animations() {
    // Protocol-specific rendering
    match animation_manager.protocol() {
        TerminalProtocol::Kitty => { /* Use Kitty protocol */ }
        TerminalProtocol::Sixel => { /* Use Sixel protocol */ }
        TerminalProtocol::None => { /* Fallback to ASCII */ }
    }
}
```

### Frame Rate Configuration
```rust
// Set maximum frame rate (1-60 FPS)
animation_manager.set_max_frame_rate(20); // 20 FPS max
```

## Usage Examples

### Automatic HTML Processing
When viewing HTML emails with GIF images, the system automatically:

1. Detects GIF images in HTML content
2. Downloads and caches the GIF data
3. Starts playback with looping enabled
4. Replaces image placeholders with animation frames

### Manual Animation Control
```rust
// Initialize in content preview
content_preview.initialize_animation_manager();

// Process HTML content for animations
content_preview.process_html_animations(&html_content).await?;

// Check if terminal supports animations
if content_preview.animations_supported() {
    println!("Animations are supported!");
}

// Clear all animations when done
content_preview.clear_all_animations().await;
```

## Limitations

### Current Limitations
- **GIF Only**: Currently supports only GIF animations (no WebM, MP4, etc.)
- **Terminal Dependency**: Requires compatible terminal for optimal experience
- **Memory Usage**: Large GIFs consume significant memory for frame caching
- **CPU Usage**: Animation playback uses CPU for frame processing and display

### Future Enhancements
- Support for additional animated formats (WebP, APNG)
- Streaming playback without full frame caching
- Hardware acceleration for compatible terminals
- User controls for animation speed and looping
- Integration with keyboard shortcuts for animation control

## Troubleshooting

### Common Issues

1. **Animations Not Playing**
   - Check terminal compatibility with `animation_manager.supports_animations()`
   - Verify GIF file is valid and accessible
   - Check network connectivity for URL-based GIFs

2. **Poor Performance**
   - Reduce frame rate with `set_max_frame_rate()`
   - Clear animation cache regularly
   - Limit number of simultaneous animations

3. **Display Issues**
   - Ensure terminal size is adequate for animation display
   - Check terminal's graphics protocol support
   - Verify color depth and terminal capabilities

### Debug Information
```rust
// Get cache statistics
let (cached_count, active_count) = animation_manager.get_cache_stats().await;
println!("Cached: {}, Active: {}", cached_count, active_count);

// Get animation info
let info = animation_manager.get_animation_info(&animation_id).await?;
println!("Frames: {}, Duration: {}ms", info.frame_count, info.duration_ms);
```

## Testing

Run the animation system test example:
```bash
cargo run --example animation_test
```

This example demonstrates:
- Terminal compatibility detection
- GIF loading from different sources
- Animation playback and control
- Cache management

## Integration with Email Client

The animation system is fully integrated with Comunicado's email functionality:

- **HTML Emails**: Automatic GIF detection and playback
- **Email Attachments**: Direct viewing of animated GIF attachments
- **Performance**: Efficient caching and frame management
- **User Experience**: Seamless animation playback without external tools

This provides a modern, terminal-native email experience with rich media support while maintaining the efficiency and keyboard-driven workflow that terminal users expect.