# Image and Media Support

Comunicado brings rich media support to the terminal, allowing you to view images, animations, and other content directly within your email client without switching to external applications.

## Supported Media Types

### Images
Comunicado supports all common image formats:
- **JPEG/JPG** - Photos and compressed images
- **PNG** - Screenshots and graphics with transparency
- **GIF** - Both static images and animations
- **WebP** - Modern web format with excellent compression
- **BMP** - Basic bitmap images
- **TIFF** - High-quality image format

### Animations
- **GIF animations** - Full support with frame-by-frame playback
- **Animated WebP** - Modern animated format (where supported)

### Other Media
- **PDF attachments** - Text extraction and basic viewing
- **Text files** - Syntax highlighting for code files
- **Archive files** - Content listing for ZIP, TAR, etc.

## Terminal Compatibility

The quality of media display depends on your terminal's capabilities:

### Full Graphics Support
**Kitty Terminal**
The best experience with full graphics protocol support:
- True color images at full resolution
- Smooth GIF animations
- Proper image scaling and positioning
- Minimal performance overhead

**Foot Terminal**
Excellent support through Sixel graphics:
- High-quality image rendering
- Good animation performance
- Proper color reproduction

**Wezterm**
Good graphics support with modern protocols:
- Solid image display
- Animation support
- Cross-platform consistency

### Partial Support
**Modern XTerm (with Sixel)**
Basic graphics capabilities:
- Static image display
- Limited color palette
- Simple animations

**Alacritty**
Limited graphics support:
- ASCII art representation of images
- Text-based media information
- Fallback viewing options

### Fallback Support
**Other Terminals**
When graphics aren't available:
- ASCII art approximations
- Detailed media information
- External viewer integration
- System application launching

## Image Display Features

### Automatic Display
Images embedded in emails or attached to messages are automatically displayed when:
- Your terminal supports graphics protocols
- The image is reasonably sized
- Image display is enabled in settings

### Display Options
**Inline Display**
Images appear within the message content, maintaining the email's layout and flow.

**Attachment View**
Press `A` to see all attachments in a dedicated view, where you can:
- Browse through multiple images
- View full metadata
- Choose display options
- Save or share images

**Fullscreen View**
Press `f` when viewing an image to see it in fullscreen mode:
- Full terminal window usage
- Zoom in and out with `+` and `-`
- Pan around large images
- Return to normal view with `Esc`

### Image Information
When viewing images, Comunicado displays:
- File format and dimensions
- File size and compression ratio
- Color depth and profile information
- EXIF data (for photos)
- Creation and modification dates

## Animation Support

### GIF Animation Playback
Comunicado provides sophisticated GIF animation support:

**Automatic Playback**
GIF animations start playing automatically when displayed. The playback includes:
- Proper frame timing based on GIF metadata
- Smooth transitions between frames
- Respect for loop settings in the GIF file

**Playback Controls**
While viewing animations:
- `Space` - Play or pause the animation
- `Ctrl+Space` - Stop all animations in the current view
- `r` - Restart animation from the beginning
- `s` - Step through frames manually

**Performance Management**
To prevent terminal overload:
- Frame rate limiting (max 30 FPS by default)
- Memory-efficient frame caching
- Automatic pause when scrolling away
- Background processing for large animations

### Animation Information
For animated content, you can view:
- Total number of frames
- Animation duration
- Frame rate and timing information
- File size and compression details
- Loop count settings

## Media Organization

### Attachment Management
Comunicado provides comprehensive attachment handling:

**Attachment List View**
Press `A` in any message to see all attachments:
- Thumbnail previews (when available)
- File types and sizes
- Quick actions for each attachment

**Bulk Operations**
- Save all attachments with `Ctrl+S`
- View multiple images in sequence
- Extract archive contents
- Generate thumbnail sheets

### Media Cache
Comunicado intelligently caches media content:
- Downloaded images are cached locally
- Thumbnails are generated and stored
- Cache size limits prevent disk bloat
- Manual cache cleaning options available

**Cache Management**
- View cache statistics in settings
- Clear specific types of cached content
- Set cache size limits
- Configure cache retention policies

## Viewing Options

### Display Modes
**Adaptive Mode** (Default)
Automatically chooses the best display method based on:
- Terminal capabilities
- Image size and aspect ratio
- Available screen space
- Performance considerations

**Force Graphics Mode**
Always attempts to display using graphics protocols, even if it might cause performance issues.

**Text Mode**
Shows ASCII art representations and detailed file information instead of actual graphics.

**External Mode**
Automatically opens media in system default applications.

### Scaling and Sizing
**Automatic Scaling**
Images are automatically scaled to fit within:
- Available terminal space
- Reasonable display dimensions
- Aspect ratio preservation
- Readability requirements

**Manual Scaling**
When viewing images:
- `+` - Zoom in (up to 400%)
- `-` - Zoom out (down to 25%)
- `0` - Reset to automatic size
- `f` - Fit to screen width
- `F` - Fit to screen height

### Color Handling
**True Color Support**
On compatible terminals:
- Full 24-bit color reproduction
- Proper color space handling
- Gamma correction
- Color profile awareness

**Limited Color Support**
On older terminals:
- Intelligent color quantization
- Dithering for better approximation
- High contrast optimization
- Accessibility considerations

## Email Integration

### HTML Email Images
Images embedded in HTML emails are handled seamlessly:
- Automatic download and caching
- Inline display within message flow
- Alternative text display when images fail
- Privacy controls for external images

**External Image Handling**
For privacy and security:
- External images are blocked by default
- Manual approval for image loading
- Whitelist trusted senders
- Batch approval options

### Image Attachments
Direct image attachments receive special treatment:
- Automatic thumbnail generation
- Quick preview without opening
- Batch viewing of multiple images
- Export and sharing options

## Performance Considerations

### Memory Usage
**Efficient Caching**
- Compressed thumbnail storage
- Smart memory limits
- Automatic cleanup of old cache entries
- Priority-based cache eviction

**Large Image Handling**
- Progressive loading for very large images
- On-demand full resolution loading
- Memory-mapped file access
- Streaming decode for animations

### Network Efficiency
**Bandwidth Management**
- Parallel image downloads
- Resume interrupted downloads
- Compressed transfer when possible
- User-configurable download limits

**Offline Viewing**
- Cached images available offline
- Local thumbnail generation
- Metadata preservation
- Sync status indicators

## Accessibility Features

### Visual Accessibility
**High Contrast Mode**
- Automatic contrast enhancement
- Edge detection and sharpening
- Color inversion options
- Brightness adjustment

**Alternative Text**
- Screen reader compatible descriptions
- Automatic image analysis
- User-added descriptions
- Context-aware alternatives

### Keyboard Navigation
All media features are accessible via keyboard:
- Tab navigation through media elements
- Keyboard shortcuts for all actions
- No mouse dependency
- Voice control compatibility

## Security and Privacy

### Safe Media Handling
**File Type Validation**
- Strict file type checking
- Magic number verification
- Malformed file detection
- Safe fallback handling

**Content Scanning**
- Basic malware detection
- Suspicious content warnings
- User approval for unknown formats
- Sandboxed rendering when possible

### Privacy Protection
**External Content**
- Block tracking pixels by default
- User consent for external images
- Anonymous download options
- Sender whitelist management

**Metadata Handling**
- EXIF data privacy controls
- Location information stripping
- Automatic anonymization options
- Metadata viewing and editing

This comprehensive media support makes Comunicado a full-featured email client that doesn't compromise on rich content, even in a terminal environment. The intelligent fallbacks ensure that everyone can access content regardless of their terminal's capabilities, while those with modern terminals get a premium media experience.