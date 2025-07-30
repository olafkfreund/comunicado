# Keyboard Shortcuts Reference

Comunicado is designed for keyboard-driven efficiency. This guide covers all available shortcuts, organized by context and functionality.

> **Status Update**: This documentation has been updated to reflect only the keyboard shortcuts that are actually implemented and working. See `keyboard-shortcuts-working.md` for a verified list of all working shortcuts.

## Philosophy

Comunicado's keyboard shortcuts follow these principles:
- **Terminal compatibility first**: No function keys (F1-F12) to ensure universal compatibility
- Familiar patterns from popular terminal applications
- Vim-style navigation where appropriate
- Consistent modifiers across similar actions
- Context-aware design to prevent conflicts
- Discoverable through help screens and status bars

## ✅ Global Shortcuts

These shortcuts work throughout the application:

### Application Control
- `q` - Quit Comunicado
- `Ctrl+C` - Force quit application
- `~` - Show start page
- `?` - Show help/keyboard shortcuts
- `g` - Go to calendar
- `,` - Settings/configuration (when implemented)
- `Esc` - Cancel current operation or go back
- `Tab` - Navigate to next panel
- `Shift+Tab` - Navigate to previous panel

### Search and Commands
- `/` - Start search in message list
- `f` - Start folder search

## ✅ Email Management

### Basic Navigation
- `j` / `Down` - Move down in current list
- `k` / `Up` - Move up in current list
- `h` - Move left (vim-style, context-dependent)
- `l` - Move right (vim-style, context-dependent)
- `Enter` - Select current item
- `Space` - Toggle expanded/collapsed state

### Email Actions
- `c` - Compose new email
- `Ctrl+D` - Show draft list

### Message Actions (**New - Recently Fixed**)
- `Ctrl+R` - Reply to selected message
- `Shift+R` - Reply to all recipients
- `Ctrl+F` - Forward selected message
- `Shift+Delete` - Delete selected message
- `Shift+A` - Archive selected message
- `Shift+M` - Mark message as read
- `Shift+U` - Mark message as unread

### Message Navigation (**New - Recently Fixed**)
- `n` - Navigate to next message
- `p` - Navigate to previous message

### Folder Operations
- `Ctrl+N` - Create new folder (updated shortcut)
- `d` - Delete selected folder
- `R` - Rename folder
- `F` - Force refresh folder (full IMAP sync)
- `Ctrl+R` - Refresh folder
- `Delete` - Delete folder
- `.` - Show folder context menu

### View Controls
- `t` - Toggle threaded view
- `o` - Expand selected thread
- `C` - Collapse selected thread
- `m` - Toggle view mode (formatted/raw/headers)
- `H` - Toggle header display

### Sorting
- `s` - Sort messages by date
- `r` - Sort messages by sender
- `u` - Sort messages by subject

### Content Preview
- `Home` - Scroll to top of content
- `End` - Scroll to bottom of content
- `a` - Select first attachment
- `v` - View selected attachment
- `O` - Open attachment with system application
- `Ctrl+J` - Navigate to next attachment
- `Ctrl+K` - Navigate to previous attachment

### Copy Operations
- `Ctrl+Y` - Copy email content to clipboard
- `Alt+C` - Copy attachment info to clipboard

## ✅ Account Management
- `Ctrl+A` - Add new account
- `Ctrl+X` - Remove current account
- `Ctrl+R` - Refresh current account

## ✅ Contacts
- `Ctrl+Shift+C` - Open contacts popup

## ✅ Calendar Management

### Calendar Navigation
- `g` - Go to calendar (global access)
- `Esc` - Close calendar/return to previous view

### Calendar Views
- `1` - Day view
- `2` - Week view
- `3` - Month view
- `4` - Agenda view
- `←` / `→` - Previous/Next month
- `.` - Jump to today

### Event Management
- `e` - Create new event
- `Enter` - View event details
- `Ctrl+E` - Edit selected event
- `d` - Delete selected event
- `Ctrl+S` - Save event (in event form)

### Todo Management
- `T` - Create new todo
- `t` - View todos
- `Space` - Toggle todo completion

## ✅ Compose Mode

### Basic Actions
- `Ctrl+S` - Send email
- `Ctrl+D` - Save as draft
- `Esc` - Cancel composition

### Spell Checking
- `Ctrl+Z` - Toggle spell checking
- `Ctrl+N` - Next spelling error
- `Ctrl+P` - Previous spelling error  
- `Ctrl+,` - Spell check configuration

## ⚠️ Features Not Yet Implemented

The following sections document features that are planned but not yet working. These shortcuts are **not functional** in the current version:


### Advanced Threading (Planned)
- Complex thread navigation
- Thread manipulation beyond basic expand/collapse
- Thread-level operations

### Advanced Search (Planned)
- Full-text search with complex queries
- Search by sender, subject, date range
- Tag-based filtering
- Attachment content search

### Import/Export (Planned)
- Maildir import and export
- Data backup and restore
- Configuration import/export
- Migration from other email clients

### Media and Animation Support (Planned)
- Image display controls
- Animation playback
- Media zoom and navigation
- Fullscreen viewing

### Developer Features (Planned)
- Debug mode toggles
- Log viewing
- Configuration reloading
- Test execution

## Key Changes Made During Recent Updates

### Terminal Compatibility Migration (July 2025)

**BREAKING CHANGE**: All function keys (F1-F12) have been replaced with terminal-friendly alternatives for universal compatibility across all terminal environments, including VSCode terminal and remote SSH sessions.

#### F-Key Migration Summary:
- **F3 (Calendar)** → **`g`** (go to calendar)
- **F1 (Help)** → **`?`** (help/shortcuts)
- **F4 (Settings)** → **`,`** (comma, vim-style)
- **F5 (Refresh)** → **`Ctrl+R`** (refresh)
- **F2 (Rename)** → **`R`** (uppercase R)
- **F1 (Save)** → **`Ctrl+S`** (save in forms)
- **F3 (Delete)** → **`d`** (delete in forms)

#### Compose Mode Migration:
- **F1 (Send)** → **`Ctrl+S`** (send email)
- **F2 (Save Draft)** → **`Ctrl+D`** (save draft)
- **F7 (Spell Check)** → **`Ctrl+Z`** (toggle spell check)
- **F8/F9 (Spell Nav)** → **`Ctrl+N`/`Ctrl+P`** (next/prev error)
- **F10 (Spell Config)** → **`Ctrl+,`** (configuration)

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
- ✅ Complete calendar management system
- ✅ Reply, Reply All, and Forward functionality
- ✅ Message navigation (next/previous)
- ✅ Message status changes (read/unread)
- ✅ Archive and delete operations
- ✅ Spell checking with comprehensive controls
- ✅ Universal terminal compatibility

## Tips for Current Version

### Learn the Working Shortcuts First
Start with the shortcuts marked with ✅ that are actually implemented:
1. Basic navigation (`j`, `k`, `Enter`)
2. Message actions (`Ctrl+R`, `Ctrl+F`, `n`, `p`)
3. Account management (`Ctrl+A`, `Ctrl+X`)

### Context Awareness
Remember that shortcuts change meaning based on context:
- `d` deletes a folder when folder tree is focused
- `Shift+Delete` deletes messages when message list is focused
- `n`/`p` navigate between messages when message list is focused

### Status Bar
The status bar shows relevant shortcuts for your current context. When in doubt, check the bottom of the screen for available actions.

### Future Features
Many advanced features are planned but not yet implemented. This documentation will be updated as features are added in future releases.

---

**Documentation Status**: Updated to reflect F-key removal and calendar implementation as of July 2025. All function keys have been replaced with terminal-friendly alternatives. Features marked as "Planned" are not yet functional. For a complete list of verified working shortcuts, see `keyboard-shortcuts-working.md`.