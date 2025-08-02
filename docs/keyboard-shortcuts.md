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

## ✅ AI Assistant Shortcuts (**NEW - Recently Added**)

All AI shortcuts use **Ctrl+Alt** combinations to avoid conflicts with existing functionality:

### Core AI Features
- `Ctrl+Alt+I` - Toggle AI assistant panel
- `Ctrl+Alt+G` - Open AI configuration and settings
- `Ctrl+Alt+S` - Get AI suggestions for current email
- `Ctrl+Alt+U` - Generate AI summary of current email
- `Ctrl+Alt+A` - Analyze email content with AI

### Email Composition AI
- `Ctrl+Alt+C` - AI assistance for email composition
- `Ctrl+Alt+R` - Generate quick reply suggestions
- `Ctrl+Alt+E` - Generate email content with AI

### Calendar AI
- `Ctrl+Alt+L` - AI calendar assistance
- `Ctrl+Alt+T` - Parse scheduling requests with AI

### AI Provider Support
- **Ollama** - Local AI processing (privacy-first)
- **OpenAI** - Cloud AI with GPT models
- **Anthropic** - Cloud AI with Claude models
- **Google AI** - Cloud AI with Gemini models

> **Privacy Note**: Use Ollama for local AI processing if privacy is a concern. Cloud providers require API keys and send data to external services.

## ✅ Global Shortcuts

These shortcuts work throughout the application:

### Application Control
- `q` - Quit Comunicado
- `Ctrl+C` - Force quit application
- `~` - Show start page
- `?` - Show help/keyboard shortcuts
- `P` - Open email in full-screen viewer (**NEW - Fixed!**)
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
- `Ctrl+L` - Open calendar view
- `Ctrl+M` - Return to email view
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
- **F3 (Calendar)** → **`Ctrl+L`** (go to calendar)
- **F1 (Help)** → **`?`** (help/shortcuts)
- **F4 (Settings)** → **`,`** (comma, vim-style)
- **F5 (Refresh)** → **`Alt+R`** (refresh folder)
- **F2 (Rename)** → **`Alt+N`** (rename folder)
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
- ✅ **AI Assistant Integration** - Complete AI assistance with 10 keyboard shortcuts
- ✅ **Multi-Provider AI Support** - Ollama (local), OpenAI, Anthropic, Google AI
- ✅ **Enhanced Calendar Operations** - Create, edit, delete events with backend integration
- ✅ **Email Operations Backend** - Delete, archive, mark read/unread with IMAP integration
- ✅ Complete calendar management system
- ✅ Reply, Reply All, and Forward functionality
- ✅ Message navigation (next/previous)
- ✅ Message status changes (read/unread)
- ✅ Archive and delete operations
- ✅ Spell checking with comprehensive controls
- ✅ Universal terminal compatibility

## Tips for Current Version

### Getting Started with AI Features
1. **First Setup**: Press `Ctrl+Alt+G` to configure your AI provider
   - Choose **Ollama** for local, privacy-first AI processing
   - Or configure cloud providers (OpenAI, Anthropic, Google) with API keys
2. **Basic Usage**: Press `Ctrl+Alt+I` to toggle the AI assistant panel
3. **Context-Aware**: Select an email first, then use AI shortcuts for best results
4. **Quick Start**: Try `Ctrl+Alt+U` to summarize a selected email

### Learn the Working Shortcuts First
Start with the shortcuts marked with ✅ that are actually implemented:
1. Basic navigation (`j`, `k`, `Enter`)
2. AI assistance (`Ctrl+Alt+I`, `Ctrl+Alt+S`, `Ctrl+Alt+U`)
3. Message actions (`Ctrl+R`, `Ctrl+F`, `n`, `p`)
4. Account management (`Ctrl+A`, `Ctrl+X`)

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

**Documentation Status**: Updated to include AI assistant integration and enhanced backend operations as of August 2025. Added 10 new AI keyboard shortcuts using Ctrl+Alt combinations. Email and calendar operations now have full IMAP/CalDAV backend integration. All function keys have been replaced with terminal-friendly alternatives. Features marked as "Planned" are not yet functional. For a complete list of verified working shortcuts, see `keyboard-shortcuts-working.md`.