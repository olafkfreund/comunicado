# Keyboard Shortcuts Reference

Comunicado is designed for keyboard-driven efficiency. This guide covers all available shortcuts, organized by context and functionality.

## Philosophy

Comunicado's keyboard shortcuts follow these principles:
- Familiar patterns from popular terminal applications
- Vim-style navigation where appropriate
- Consistent modifiers across similar actions
- Discoverable through help screens and status bars

## Global Shortcuts

These shortcuts work throughout the application:

### Application Control
- `Ctrl+Q` - Quit Comunicado
- `Ctrl+R` - Refresh current view
- `F1` or `?` - Show help screen
- `Esc` - Cancel current operation or go back
- `Tab` - Navigate between panels
- `Shift+Tab` - Navigate backwards between panels

### View Navigation
- `1` - Switch to email view
- `2` - Switch to calendar view
- `3` - Switch to contacts view
- `4` - Switch to settings view
- `Ctrl+1` through `Ctrl+9` - Switch to specific account

### Search and Commands
- `/` - Start search in current view
- `:` - Open command palette
- `Ctrl+P` - Quick action menu
- `Ctrl+Shift+P` - Advanced command palette

## Email Management

### Folder Navigation
- `j` or `Down` - Move down in folder list
- `k` or `Up` - Move up in folder list
- `Enter` - Open selected folder
- `g` then `g` - Go to first folder
- `G` - Go to last folder
- `Ctrl+N` - Create new folder
- `d` - Delete selected folder (with confirmation)
- `r` - Rename selected folder

### Message List Navigation
- `j` or `Down` - Next message
- `k` or `Up` - Previous message
- `Enter` or `Space` - Open selected message
- `g` then `g` - Go to first message
- `G` - Go to last message
- `Page Down` - Scroll down one screen
- `Page Up` - Scroll up one screen
- `Home` - Go to beginning of list
- `End` - Go to end of list

### Message Actions
- `r` - Reply to message
- `R` - Reply to all
- `f` - Forward message
- `d` - Delete message
- `u` - Mark as unread
- `s` - Mark as spam
- `a` - Archive message
- `*` - Toggle star/flag
- `m` - Move to folder (opens folder selector)
- `c` - Copy to folder
- `t` - Add/remove tags

### Message Reading
- `Space` - Scroll down in message
- `Shift+Space` - Scroll up in message
- `n` - Next message (without opening)
- `p` - Previous message (without opening)
- `v` - Toggle raw/formatted view
- `h` - Toggle header display
- `i` - View message information
- `Ctrl+U` - View message source

### Composition
- `c` or `Ctrl+N` - Compose new message
- `Ctrl+S` - Save draft
- `Ctrl+Enter` - Send message
- `Ctrl+X` - Discard draft
- `Ctrl+A` - Select all text
- `Ctrl+Z` - Undo
- `Ctrl+Y` - Redo
- `Tab` - Move between compose fields
- `Ctrl+K` - Insert link
- `Ctrl+B` - Bold text (if HTML mode)
- `Ctrl+I` - Italic text (if HTML mode)

### Attachments
- `A` - View attachments list
- `Enter` - Open selected attachment
- `s` - Save attachment to disk
- `o` - Open with system application
- `v` - View in built-in viewer
- `d` - Download attachment
- `Ctrl+A` - Attach file (in compose mode)

## Threading and Conversations

### Thread Navigation
- `t` - Toggle thread view
- `Enter` - Expand/collapse thread
- `]` - Next message in thread
- `[` - Previous message in thread
- `Ctrl+]` - Next thread
- `Ctrl+[` - Previous thread
- `z` - Collapse all threads
- `Z` - Expand all threads

### Thread Actions
- `T` - Toggle thread selection
- `Ctrl+T` - Select entire thread
- `Shift+D` - Delete entire thread
- `Shift+A` - Archive entire thread
- `Shift+U` - Mark entire thread as unread

## Calendar Management

### Calendar Navigation
- `j` or `Down` - Next day/event
- `k` or `Up` - Previous day/event
- `h` or `Left` - Previous week/month
- `l` or `Right` - Next week/month
- `t` - Go to today
- `g` - Go to specific date (opens date picker)
- `w` - Week view
- `m` - Month view
- `d` - Day view
- `a` - Agenda view

### Event Management
- `n` or `c` - Create new event
- `e` - Edit selected event
- `d` - Delete selected event
- `y` - Copy event
- `p` - Paste event
- `Enter` - View event details
- `i` - View event information
- `r` - RSVP to event (if applicable)

### Event Details
- `s` - Set reminder
- `l` - Add location
- `a` - Add attendees
- `n` - Add notes
- `R` - Mark as recurring
- `P` - Set privacy level

## Search and Filtering

### Search Interface
- `/` - Open search
- `Ctrl+F` - Find in current view
- `Enter` - Execute search
- `Esc` - Cancel search
- `Ctrl+G` - Search again (next result)
- `Ctrl+Shift+G` - Search previous result
- `F3` - Find next
- `Shift+F3` - Find previous

### Advanced Search
- `Ctrl+Shift+F` - Advanced search dialog
- `Alt+F` - Search by sender
- `Alt+S` - Search by subject
- `Alt+D` - Search by date range
- `Alt+T` - Search by tag
- `Alt+A` - Search attachments

### Filters
- `F` - Apply quick filter
- `Shift+F` - Manage filters
- `Ctrl+1` through `Ctrl+5` - Apply preset filters
- `Alt+1` through `Alt+5` - Toggle filter flags

## Account and Settings

### Account Management
- `Ctrl+A` - Add new account
- `Ctrl+X` - Remove account
- `Ctrl+E` - Edit account settings
- `Ctrl+S` - Sync account
- `Ctrl+Shift+S` - Sync all accounts
- `Ctrl+O` - Account status overview

### Settings Navigation
- `p` - Preferences
- `k` - Keyboard shortcuts
- `t` - Theme settings
- `n` - Notification settings
- `s` - Security settings
- `a` - Account settings
- `i` - Import/export settings

## Import and Export

### Maildir Operations
- `Ctrl+I` - Import from Maildir
- `Ctrl+E` - Export to Maildir
- `Ctrl+M` - Maildir management
- `p` - Preview import/export
- `v` - Validate Maildir structure
- `s` - Show import/export statistics

### Data Operations
- `Ctrl+B` - Backup data
- `Ctrl+R` - Restore from backup
- `Ctrl+C` - Clean up database
- `Ctrl+V` - Verify data integrity

## Advanced Features

### Desktop Integration
- `Ctrl+Shift+N` - Toggle desktop notifications
- `Ctrl+Shift+S` - Show system status
- `Ctrl+Shift+C` - Copy to system clipboard
- `Ctrl+Shift+V` - Paste from system clipboard

### Animation and Media
- `Space` - Play/pause animations
- `Ctrl+Space` - Stop all animations
- `i` - Toggle image display
- `f` - Fullscreen image view
- `+` - Zoom in on media
- `-` - Zoom out on media
- `0` - Reset zoom

### Developer and Debug
- `Ctrl+Shift+D` - Toggle debug mode
- `Ctrl+Shift+L` - Show logs
- `Ctrl+Shift+I` - Inspect element
- `Ctrl+Shift+R` - Reload configuration
- `Ctrl+Shift+T` - Run tests

## Context-Specific Shortcuts

### In Compose Window
- `Ctrl+Enter` - Send message
- `Ctrl+S` - Save as draft
- `Ctrl+D` - Discard message
- `Ctrl+Shift+P` - Preview message
- `Ctrl+L` - Insert link
- `Ctrl+Shift+A` - Add attachment
- `Ctrl+Shift+S` - Add signature
- `Ctrl+Shift+E` - Enable encryption
- `F7` - Spell check

### In Attachment Viewer
- `Space` - Next page/frame
- `Backspace` - Previous page/frame
- `f` - Toggle fullscreen
- `r` - Rotate image
- `s` - Save attachment
- `o` - Open with external application
- `i` - Show attachment information
- `Esc` - Close viewer

### In Search Results
- `n` - Next result
- `p` - Previous result
- `Enter` - Open result
- `o` - Open in new view
- `s` - Save search
- `Ctrl+A` - Select all results
- `Ctrl+N` - New search
- `Ctrl+R` - Refine search

## Customization

Comunicado allows you to customize most keyboard shortcuts:

### Accessing Customization
1. Press `4` to go to settings
2. Press `k` for keyboard shortcuts
3. Navigate to the shortcut you want to change
4. Press `Enter` to modify
5. Press the new key combination
6. Press `Enter` to confirm

### Shortcut Conflicts
If you assign a shortcut that conflicts with an existing one, Comunicado will:
- Warn you about the conflict
- Allow you to proceed (removing the old binding)
- Suggest alternative key combinations
- Let you reset to defaults

### Backup and Restore
- Export your shortcut configuration
- Import shortcuts from file
- Reset to application defaults
- Share configurations between systems

## Tips for Efficiency

### Learn Gradually
Start with the most common shortcuts and gradually add more to your repertoire:
1. Navigation (`j`, `k`, `Enter`)
2. Basic actions (`r`, `d`, `c`)
3. Advanced features as needed

### Muscle Memory
Practice common sequences like:
- `c` → compose → `Ctrl+Enter` (send)
- `/` → search → `Enter` → navigate results
- `r` → reply → edit → `Ctrl+Enter` (send reply)

### Context Awareness
Remember that shortcuts change meaning based on context:
- `d` deletes a message in message view
- `d` deletes a folder in folder view
- `d` sets date in calendar view

### Status Bar
The status bar always shows relevant shortcuts for your current context. When in doubt, check the bottom of the screen for available actions.

This keyboard-driven approach makes Comunicado incredibly efficient once you learn the patterns. Most users find they can navigate and manage email much faster than with traditional GUI clients after a short learning period.