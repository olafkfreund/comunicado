# MCP Server Setup for Real TUI Testing

## ✅ Installation Complete

The mcpterm MCP server has been successfully installed and configured:

- **Binary location**: `~/.local/bin/mcpterm`
- **Source**: https://github.com/dwrtz/mcpterm.git
- **Configuration**: `~/.config/claude-code/mcp_config.json`

## 🎯 How to Use mcpterm for TUI Testing

The mcpterm MCP server provides two main tools:

### 1. `run` Tool
- **Purpose**: Execute commands in a stateful terminal session
- **Use case**: Running commands that need to maintain state across calls
- **Example**: `cd /some/directory` then `ls` (stays in that directory)

### 2. `runScreen` Tool  
- **Purpose**: Run TUI applications and capture screen output
- **Use case**: Testing interactive terminal applications like Comunicado
- **Example**: Start Comunicado, send keystrokes, capture UI state

## 🚀 Testing Comunicado with mcpterm

### Step 1: Start TUI Testing Session
Ask Claude Code:
```
Use the runScreen tool to start Comunicado in test mode:
- Command: ./target/debug/comunicado --config test_config.toml
- Send initial keystrokes to test basic navigation
```

### Step 2: Test Keyboard Shortcuts
Ask Claude Code:
```
Use runScreen to test Comunicado keyboard shortcuts:
- Send key "1" (Email view)
- Send key "2" (Calendar view)  
- Send key "3" (Contacts view)
- Send "^D" (Ctrl+D for Command Palette)
- Send "^[" (Escape to close)
- Capture screen after each step
```

### Step 3: Comprehensive Testing
Ask Claude Code:
```
Run a complete TUI test sequence using runScreen:
1. Start Comunicado
2. Test all major shortcuts: 1,2,3,^D,^[,^A,^[,/,^[
3. Verify UI responds correctly
4. Capture screenshots at each step
5. Test error conditions
6. Quit with "q"
```

## 🔑 Key Mappings for mcpterm

When using the `runScreen` tool, use these control sequences:

```
"^D" = Ctrl+D (Command Palette)
"^A" = Ctrl+A (Account Manager)
"^[" = Escape
"^C" = Ctrl+C  
"^J" = Enter/Return
"^M" = Carriage Return
"^H" = Backspace
"^L" = Form Feed (Clear screen)
"^U" = Clear line
"^W" = Delete word
"^Y" = Paste
"^V" = Literal input
"^K" = Kill line
"^E" = End of line
"^I" = Tab
```

## 📋 Test Sequence Examples

### Basic Navigation Test
```json
{
  "command": "./target/debug/comunicado --config test_config.toml",
  "input": "123^[q"
}
```
- Starts app
- Tests views 1, 2, 3
- Escapes any menus
- Quits

### Comprehensive Shortcut Test
```json
{
  "command": "./target/debug/comunicado --config test_config.toml", 
  "input": "1^D^[2^A^[3/^[q"
}
```
- Email view (1)
- Command palette (^D), close (^[)
- Calendar view (2)  
- Account manager (^A), close (^[)
- Contacts view (3)
- Search (/), close (^[)
- Quit (q)

## 🎯 Expected Results

When working correctly, mcpterm should:

1. **Start Comunicado** in a virtual terminal
2. **Send real keystrokes** to the application
3. **Capture screen output** showing UI changes
4. **Verify shortcuts work** by checking screen content
5. **Test error conditions** and edge cases
6. **Provide visual feedback** of what's happening

## 🔧 Troubleshooting

If mcpterm doesn't work:

1. **Check binary**: `ls -la ~/.local/bin/mcpterm`
2. **Test manually**: `echo '{"test": true}' | ~/.local/bin/mcpterm`
3. **Check PATH**: Make sure `~/.local/bin` is in PATH
4. **Rebuild if needed**: Go to mcpterm source and run `make`

## 📞 Usage with Claude Code

To use this setup:

1. **Start Claude Code** (this session)
2. **Ask Claude to use mcpterm tools**:
   - "Use the runScreen tool to test Comunicado TUI"
   - "Start Comunicado and test keyboard shortcuts"
   - "Capture screen output after each keystroke"
3. **Claude Code will automatically use the MCP server**
4. **Get real TUI testing results** with actual screenshots

## ✅ What This Gives You

- **Real TUI testing** (not mocked)
- **Actual keyboard input** sent to application
- **Screen capture** showing UI state
- **Comprehensive shortcut testing** 
- **Error condition testing**
- **Visual verification** of UI behavior

The mcpterm MCP server is now ready for Claude Code to use for comprehensive TUI testing!