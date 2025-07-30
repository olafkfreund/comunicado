# Terminal Compatibility Guide

> Terminal graphics protocol support and image display capabilities

## Overview

Comunicado is designed for universal terminal compatibility, supporting both advanced graphics protocols for image display and keyboard input systems that work across all terminal environments. This document outlines compatibility across different terminal emulators and provides setup instructions for optimal experience.

## Keyboard Compatibility

Comunicado uses **zero function keys (F1-F12)** to ensure universal keyboard compatibility across all terminal environments.

### Terminal Keyboard Support

| Terminal Environment | Compatibility | Notes |
|---------------------|---------------|-------|
| **VSCode Terminal** | ✅ Full | All shortcuts work perfectly |
| **SSH Sessions** | ✅ Full | No function key dependencies |
| **Tmux/Screen** | ✅ Full | Standard keys pass through cleanly |
| **Remote Terminals** | ✅ Full | Works in any remote environment |
| **Docker Containers** | ✅ Full | No special terminal requirements |
| **Cloud IDEs** | ✅ Full | GitHub Codespaces, GitPod, etc. |

### Why No Function Keys?

Function keys (F1-F12) can be:
- Intercepted by terminal applications
- Mapped to different codes in different environments
- Not available or unreliable in remote sessions
- Conflicting with IDE or editor shortcuts

Instead, Comunicado uses:
- **Single characters**: `g` (calendar), `?` (help), `,` (settings)
- **Ctrl combinations**: `Ctrl+S` (save), `Ctrl+R` (refresh)
- **Modifier keys**: `Shift+Delete` (delete message)
- **Context-aware keys**: `d` (delete in appropriate context)

## Supported Graphics Protocols

### Kitty Graphics Protocol
- **Best support** - Full-color images with high quality
- **Format support** - PNG, JPEG, GIF, WebP
- **Features** - Async loading, caching, proper scaling

### Sixel Graphics Protocol  
- **Good support** - Color images with some limitations
- **Format support** - All major formats converted to Sixel
- **Features** - Wide terminal compatibility

### ASCII Fallback
- **Universal support** - Works in any terminal
- **Display** - Text-based placeholder boxes
- **Information** - Shows alt text and dimensions

## Terminal Emulator Support

### Full Graphics Support (Recommended)

| Terminal | Kitty Protocol | Sixel Protocol | Notes |
|----------|----------------|----------------|-------|
| **Kitty** | ✅ Native | ✅ Yes | Best experience, automatic detection |
| **Foot** | ❌ No | ✅ Native | Excellent Sixel support |
| **WezTerm** | ✅ Yes | ✅ Yes | Good dual protocol support |
| **Alacritty** | ❌ No | ❌ No | ASCII fallback only |

### Partial Graphics Support

| Terminal | Kitty Protocol | Sixel Protocol | Notes |
|----------|----------------|----------------|-------|
| **XTerm** | ❌ No | ✅ Optional | Requires `-ti vt340` or Sixel compilation |
| **GNOME Terminal** | ❌ No | ❌ No | ASCII fallback only |
| **Konsole** | ❌ No | ✅ Recent | Sixel in newer versions |
| **Terminator** | ❌ No | ❌ No | ASCII fallback only |

### Cloud/Remote Terminals

| Terminal | Kitty Protocol | Sixel Protocol | Notes |
|----------|----------------|----------------|-------|
| **SSH + Kitty** | ✅ Yes | ✅ Yes | Forward terminal type properly |
| **Tmux** | ⚠️ Limited | ⚠️ Limited | May interfere with graphics |
| **Screen** | ❌ No | ❌ No | ASCII fallback only |

## Setup Instructions

### Kitty Terminal

```bash
# Install Kitty (if not already installed)
curl -L https://sw.kovidgoyal.net/kitty/installer.sh | sh /dev/stdin

# Or via package manager
sudo apt install kitty          # Ubuntu/Debian
brew install kitty              # macOS
```

**Configuration**: No special setup required - Comunicado automatically detects Kitty graphics support.

### Foot Terminal

```bash
# Install Foot
sudo apt install foot           # Ubuntu/Debian
sudo pacman -S foot            # Arch Linux

# Enable Sixel in config (~/.config/foot/foot.ini)
[tweak]
sixel=yes
```

### WezTerm

```bash
# Install WezTerm
curl -fsSL https://apt.fury.io/wez/gpg.key | sudo gpg --yes --dearmor -o /usr/share/keyrings/wezterm-fury.gpg
echo 'deb [signed-by=/usr/share/keyrings/wezterm-fury.gpg] https://apt.fury.io/wez/ * *' | sudo tee /etc/apt/sources.list.d/wezterm.list
sudo apt update
sudo apt install wezterm
```

### XTerm with Sixel

```bash
# Compile XTerm with Sixel support
sudo apt build-dep xterm
wget https://invisible-island.net/datafiles/release/xterm.tar.gz
tar -xzf xterm.tar.gz
cd xterm-*
./configure --enable-sixel-graphics
make
sudo make install

# Run with Sixel support
xterm -ti vt340
```

## Testing Image Display

### 1. Check Terminal Detection

When Comunicado starts, check the logs for protocol detection:

```
INFO: Detected terminal protocol: Kitty
INFO: Image display enabled with Kitty graphics
```

### 2. Test with Sample Email

Create a test HTML email with images:

```html
<img src="https://httpbin.org/image/png" alt="Test PNG">
<img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChAFfcSO3UQAAAABJRU5ErkJggg==" alt="Embedded">
```

### 3. Verify Display Modes

- **Full Graphics**: Images render inline with proper colors
- **Sixel**: Images render with some color quantization
- **Fallback**: ASCII boxes with alt text

## Troubleshooting

### Images Not Displaying

1. **Check Terminal Support**
   ```bash
   echo $TERM
   echo $TERM_PROGRAM
   ```

2. **Test Graphics Manually**
   ```bash
   # Kitty test
   kitty +kitten icat /path/to/image.png
   
   # Sixel test (if supported)
   img2sixel /path/to/image.png
   ```

3. **Check Environment Variables**
   ```bash
   # Kitty detection
   echo $KITTY_WINDOW_ID
   
   # General terminal capabilities
   echo $COLORTERM
   ```

### Performance Issues

- **Large Images**: Comunicado automatically resizes images for terminal display
- **Slow Loading**: Images are cached after first download
- **Memory Usage**: Clear cache with internal commands if needed

### Remote Sessions

- **SSH**: Ensure terminal type forwarding: `ssh -t user@host`
- **Tmux/Screen**: May require `set -g allow-passthrough on` in tmux
- **Colors**: Use `TERM=xterm-256color` for better color support

## Protocol Detection Logic

Comunicado uses this detection order:

1. **Environment Variables**: `$KITTY_WINDOW_ID`, `$TERM_PROGRAM`
2. **Terminal Type**: `$TERM` contains known graphics-capable terminals
3. **Capability Testing**: Query terminal for graphics support
4. **Fallback**: ASCII placeholders if no graphics detected

## Best Practices

### For Maximum Compatibility

1. **Use Kitty or Foot** - Best image experience
2. **Set Proper TERM** - Ensure correct terminal identification
3. **Update Terminals** - Newer versions have better graphics support
4. **Test Regularly** - Verify graphics work after terminal updates

### For Development

1. **Test Multiple Terminals** - Verify fallbacks work properly
2. **Check Performance** - Monitor image loading times
3. **Validate Formats** - Test various image types and sizes

## Future Enhancements

- **iTerm2 Support** - Inline images for macOS users
- **Terminology Support** - EFL-based terminal graphics
- **Performance Tuning** - Better caching and compression
- **Format Extensions** - Support for more image types

---

*For latest terminal compatibility updates, check the [terminal emulator documentation](https://github.com/kovidgoyal/kitty/blob/master/docs/graphics-protocol.md) and [Sixel specification](https://en.wikipedia.org/wiki/Sixel).*