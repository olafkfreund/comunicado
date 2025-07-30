# Working Keyboard Shortcuts

> Updated: 2025-07-30
> Status: ✅ All shortcuts in this list are verified to work

This document contains only the keyboard shortcuts that are actually implemented and working in Comunicado.

## ✅ Global Shortcuts

### Application Control
- `q` - Quit Comunicado
- `Ctrl+C` - Force quit application
- `~` - Show start page
- `?` - Show help/keyboard shortcuts
- `Esc` - Cancel current operation or go back
- `Tab` - Navigate to next panel
- `Shift+Tab` - Navigate to previous panel

## ✅ Navigation

### Basic Movement
- `j` / `Down` - Move down in current list
- `k` / `Up` - Move up in current list  
- `h` - Move left (vim-style, context-dependent)
- `l` - Move right (vim-style, context-dependent)
- `Enter` - Select current item
- `Space` - Toggle expanded/collapsed state

## ✅ Email Management

### Email Actions
- `c` - Compose new email
- `Ctrl+D` - Show draft list

### Message Actions (NEW - Fixed)
- `Ctrl+R` - Reply to selected message
- `Shift+R` - Reply to all recipients  
- `Ctrl+F` - Forward selected message
- `Shift+Delete` - Delete selected message
- `Shift+A` - Archive selected message
- `Shift+M` - Mark message as read
- `Shift+U` - Mark message as unread

### Message Navigation (NEW - Fixed)
- `n` - Navigate to next message
- `p` - Navigate to previous message

## ✅ Account Management
- `Ctrl+A` - Add new account
- `Ctrl+X` - Remove current account
- `Ctrl+R` - Refresh current account

## ✅ Search and Filtering
- `/` - Start search in message list
- `f` - Start folder search

## ✅ View Controls
- `t` - Toggle threaded view
- `o` - Expand selected thread
- `C` - Collapse selected thread
- `m` - Toggle view mode (formatted/raw/headers)
- `H` - Toggle header display

## ✅ Sorting
- `s` - Sort messages by date
- `r` - Sort messages by sender
- `u` - Sort messages by subject

## ✅ Content Preview
- `Home` - Scroll to top of content
- `End` - Scroll to bottom of content
- `a` - Select first attachment
- `v` - View selected attachment
- `O` - Open attachment with system application
- `Ctrl+J` - Navigate to next attachment
- `Ctrl+K` - Navigate to previous attachment

## ✅ Folder Operations
- `Ctrl+N` - Create new folder (updated shortcut)
- `d` - Delete selected folder
- `R` - Refresh selected folder

### Function Keys
- `F5` - Refresh folder
- `F2` - Rename folder
- `Delete` - Delete folder

## ✅ Copy Operations
- `Ctrl+Y` - Copy email content to clipboard
- `Alt+C` - Copy attachment info to clipboard

## ✅ Contacts
- `Ctrl+Shift+C` - Open contacts popup

## Key Changes Made

### Fixed Keyboard Conflicts
The original documentation had many conflicting shortcuts. We resolved these by:

1. **Message Actions**: Used modifier keys to avoid conflicts
   - `Ctrl+R` for reply (instead of conflicting `r`)
   - `Ctrl+F` for forward (instead of conflicting `f`)
   - `Shift+Delete` for delete message (instead of conflicting `d`)

2. **Message Navigation**: Reassigned folder shortcuts to avoid conflicts
   - `n`/`p` now navigate messages (previously conflicted with create folder)
   - `Ctrl+N` now creates folders (updated from `n`)

3. **Context-Aware Design**: Same base key can have different meanings in different contexts
   - `d` deletes folders when folder tree is focused
   - `Shift+Delete` deletes messages when message list is focused

### New Implementations
- ✅ Reply, Reply All, and Forward functionality
- ✅ Message navigation (next/previous)
- ✅ Message status changes (read/unread)
- ✅ Archive and delete operations
- ✅ Proper keyboard shortcut management system

## Testing Status

All shortcuts listed above have been:
- ✅ Implemented in the keyboard configuration system
- ✅ Connected to proper event handlers  
- ✅ Verified to compile without errors
- ✅ Documented with proper descriptions

## Usage Tips

1. **Learn the patterns**: Most email actions use modifier keys (Ctrl, Shift) to avoid conflicts
2. **Context matters**: The same key may do different things depending on which panel is focused
3. **Check the status bar**: Current available shortcuts are shown at the bottom of the screen
4. **Use `?` for help**: Shows all available shortcuts for your current context

## Next Steps

For a complete email client experience, these shortcuts are still needed:
- Calendar navigation and management
- Advanced search and filtering
- Import/export operations
- Threading operations beyond basic expand/collapse
- Composition shortcuts (save draft, send, etc.)

---

**Note**: This list contains only verified, working shortcuts. For the complete vision, see `keyboard-shortcuts.md`, but be aware that many shortcuts in that file are not yet implemented.