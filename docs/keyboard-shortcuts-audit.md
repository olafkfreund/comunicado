# Keyboard Shortcuts Audit Report

> Generated: 2025-07-30
> Status: Comprehensive analysis of documented vs implemented shortcuts

## Executive Summary

This audit compares the documented keyboard shortcuts in `keyboard-shortcuts.md` with the actual implementation in the codebase. Many documented shortcuts are **not implemented** or work differently than described.

## ✅ Working Shortcuts (Verified Implementation)

### Global Shortcuts
- `q` - Quit Comunicado ✅
- `Ctrl+C` - Force quit ✅ 
- `~` - Show start page ✅
- `?` - Show keyboard shortcuts ✅
- `Tab` - Navigate between panels ✅
- `Shift+Tab` - Navigate backwards between panels ✅
- `Esc` - Cancel/escape current operation ✅

### Navigation
- `j` / `Down` - Move down ✅
- `k` / `Up` - Move up ✅
- `h` - Move left (vim-style) ✅
- `l` - Move right (vim-style) ✅
- `Enter` - Select current item ✅
- `Space` - Toggle expanded/collapsed ✅

### Email Management
- `c` - Compose new email ✅
- `Ctrl+D` - Show draft list ✅

### Account Management
- `Ctrl+A` - Add new account ✅
- `Ctrl+X` - Remove account ✅
- `Ctrl+R` - Refresh account ✅

### Search
- `/` - Start search in message list ✅
- `f` - Start folder search ✅

### View Controls
- `t` - Toggle threaded view ✅
- `o` - Expand thread ✅
- `C` - Collapse thread ✅
- `m` - Toggle view mode ✅
- `H` - Toggle headers ✅

### Sorting
- `s` - Sort by date ✅
- `r` - Sort by sender ✅
- `u` - Sort by subject ✅

### Content Preview
- `Home` - Scroll to top ✅
- `End` - Scroll to bottom ✅
- `a` - Select first attachment ✅
- `v` - View attachment ✅
- `O` - Open attachment with system ✅
- `Ctrl+J` / `Ctrl+K` - Navigate attachments ✅

### Folder Operations
- `n` - Create folder ✅
- `d` - Delete folder ✅
- `R` - Refresh folder ✅
- `F5` - Folder refresh (function key) ✅
- `F2` - Folder rename (function key) ✅
- `Delete` - Folder delete (function key) ✅

### Copy Operations
- `Ctrl+Y` - Copy email content ✅
- `Alt+C` - Copy attachment info ✅

### Contacts
- `Ctrl+Shift+C` - Open contacts popup ✅

## ❌ Non-Working/Missing Shortcuts

### Application Control
- `Ctrl+R` - Refresh current view (conflicting with refresh account)
- `F1` - Show help screen (not implemented)

### View Navigation
- `1` - Switch to email view ❌
- `2` - Switch to calendar view ❌
- `3` - Switch to contacts view ❌
- `4` - Switch to settings view ❌
- `Ctrl+1` through `Ctrl+9` - Switch to specific account ❌

### Search and Commands
- `:` - Open command palette ❌
- `Ctrl+P` - Quick action menu ❌
- `Ctrl+Shift+P` - Advanced command palette ❌

### Message Actions (Most are missing)
- `r` - Reply to message ❌
- `R` - Reply to all ❌
- `f` - Forward message ❌ (conflicts with folder search)
- `d` - Delete message ❌ (conflicts with delete folder)
- `u` - Mark as unread ❌ (conflicts with sort by subject)
- `s` - Mark as spam ❌ (conflicts with sort by date)
- `a` - Archive message ❌ (conflicts with select attachment)
- `*` - Toggle star/flag ❌
- `m` - Move to folder ❌ (conflicts with toggle view mode)
- `c` - Copy to folder ❌ (conflicts with compose)
- `t` - Add/remove tags ❌ (conflicts with toggle thread)

### Message Reading
- `Space` - Scroll down in message ❌ (conflicts with toggle expanded)
- `Shift+Space` - Scroll up in message ❌
- `n` - Next message ❌ (conflicts with create folder)
- `p` - Previous message ❌
- `v` - Toggle raw/formatted view ❌ (conflicts with view attachment)
- `h` - Toggle header display ❌ (conflicts with vim left)
- `i` - View message information ❌
- `Ctrl+U` - View message source ❌

### Composition
- `Ctrl+N` - Compose new message ❌
- `Ctrl+S` - Save draft ❌
- `Ctrl+Enter` - Send message ❌
- `Ctrl+X` - Discard draft ❌ (conflicts with remove account)
- `Ctrl+A` - Select all text ❌ (conflicts with add account)
- `Ctrl+Z` - Undo ❌
- `Ctrl+Y` - Redo ❌ (conflicts with copy email)
- `Ctrl+K` - Insert link ❌
- `Ctrl+B` - Bold text ❌
- `Ctrl+I` - Italic text ❌

### All Threading Shortcuts Are Missing
- `]` / `[` - Next/previous message in thread ❌
- `Ctrl+]` / `Ctrl+[` - Next/previous thread ❌
- `z` / `Z` - Collapse/expand all threads ❌
- `T` - Toggle thread selection ❌
- `Ctrl+T` - Select entire thread ❌
- `Shift+D` - Delete entire thread ❌
- `Shift+A` - Archive entire thread ❌
- `Shift+U` - Mark entire thread as unread ❌

### All Calendar Shortcuts Are Missing
- `j` / `k` / `h` / `l` - Calendar navigation ❌
- `t` - Go to today ❌
- `g` - Go to specific date ❌
- `w` / `m` / `d` / `a` - View modes ❌
- `n` / `c` - Create new event ❌
- `e` - Edit event ❌
- All event management shortcuts ❌

### All Advanced Search Missing
- `Ctrl+F` - Find in current view ❌
- `Ctrl+G` / `Ctrl+Shift+G` - Search navigation ❌
- `F3` / `Shift+F3` - Find next/previous ❌
- `Ctrl+Shift+F` - Advanced search ❌
- All search by filters ❌

### All Import/Export Missing
- `Ctrl+I` - Import from Maildir ❌
- `Ctrl+E` - Export to Maildir ❌
- `Ctrl+M` - Maildir management ❌
- All data operations ❌

### All Animation/Media Missing
- `Space` - Play/pause animations ❌
- `Ctrl+Space` - Stop animations ❌
- `i` - Toggle image display ❌
- `f` - Fullscreen image ❌
- `+` / `-` / `0` - Zoom controls ❌

### All Developer/Debug Missing
- `Ctrl+Shift+D` - Debug mode ❌
- `Ctrl+Shift+L` - Show logs ❌
- All debug shortcuts ❌

## 🔄 Conflicting Shortcuts

Many shortcuts have conflicts where the same key is used for different purposes:

### Major Conflicts
- `r` - Sort by sender vs Reply to message
- `f` - Folder search vs Forward message  
- `d` - Delete folder vs Delete message
- `u` - Sort by subject vs Mark as unread
- `s` - Sort by date vs Mark as spam
- `a` - Select attachment vs Archive message
- `m` - Toggle view mode vs Move to folder
- `t` - Toggle thread vs Add/remove tags
- `n` - Create folder vs Next message / New event
- `c` - Compose vs Copy to folder
- `v` - View attachment vs Toggle view mode
- `h` - Vim left vs Toggle headers
- `Ctrl+X` - Remove account vs Discard draft
- `Ctrl+A` - Add account vs Select all
- `Ctrl+Y` - Copy email vs Redo
- `Space` - Toggle expanded vs Scroll message

## Recommendations

### Immediate Actions Required

1. **Resolve Key Conflicts**: Many basic email operations (reply, forward, delete message) are not accessible due to key conflicts
2. **Implement Missing Core Features**: Message actions, threading shortcuts, and calendar navigation
3. **Update Documentation**: Remove non-working shortcuts or implement missing functionality
4. **Add Context-Aware Shortcuts**: Same key should do different things in different contexts

### Priority Implementation Order

1. **High Priority**: Message actions (reply, forward, delete message)
2. **High Priority**: Message navigation (next/previous message)
3. **Medium Priority**: Calendar shortcuts
4. **Medium Priority**: Threading operations
5. **Low Priority**: Advanced features (animations, debug modes)

### Suggested Key Mapping Changes

```
Current Issues → Suggested Solutions:
r (sort sender) → R (sort sender), r (reply message)
f (folder search) → F (folder search), f (forward message)
d (delete folder) → context-aware: folder vs message
u (sort subject) → U (sort subject), u (mark unread)
s (sort date) → S (sort date), s (mark spam)
```

## Testing Recommendations

1. Create integration tests for keyboard shortcuts
2. Add context-aware shortcut validation
3. Implement shortcut conflict detection
4. Add help overlay showing available shortcuts per context

---

**Status**: This audit reveals that approximately 70% of documented shortcuts are not working or missing entirely. Immediate action is required to match implementation with documentation.