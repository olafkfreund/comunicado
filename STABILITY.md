# Comunicado Stability Guide

## System Crash Prevention

If Comunicado has been causing system crashes, follow these safety measures:

### Safe Launch Methods

1. **Resource Limited Launch**:
```bash
# Limit memory usage to prevent system overload
systemd-run --user --scope -p MemoryMax=1G cargo run

# Or with nice priority to prevent CPU hogging
nice -n 10 cargo run
```

2. **Debug Mode Launch**:
```bash
# Run with debug logging to identify crash points
RUST_LOG=debug,comunicado=trace cargo run

# Run with memory debugging
RUST_BACKTRACE=1 cargo run
```

3. **Safe Testing Mode**:
```bash
# Test compilation without running
cargo check

# Build without running
cargo build

# Test a specific module
cargo test --lib
```

### Known Stability Issues

1. **Terminal Graphics**: Some terminals may crash with sixel/kitty graphics
2. **Memory Usage**: Background sync processes may consume excessive memory  
3. **Event Loops**: High-frequency event processing could cause UI freezing
4. **Network Operations**: Blocking network calls could freeze the main thread

### Preventive Measures Added

- Panic catching in UI render functions
- Event bus memory optimization 
- Background task timeout limits
- Resource usage monitoring
- Graceful degradation on errors

### If System Still Crashes

1. Check system logs: `journalctl -f`
2. Monitor memory: `htop` or `systemctl --user status`
3. Use a VM or container for testing
4. Report the issue with full system details

### Recovery

If the system becomes unresponsive:
- Use Magic SysRq keys: Alt+SysRq+f (kill memory hogs)
- SSH in from another machine if possible
- Hard reset as last resort

### Safer Development

Consider using:
- Docker container for isolation
- Virtual machine for testing
- Resource cgroups for limits
- Separate user account for testing