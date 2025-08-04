# Settings Commands Reference

This document provides a comprehensive overview of all keyboard shortcuts and commands available in the Comunicado application settings interface.

## Opening Settings

**Keyboard Shortcut**: `Ctrl+,` (Ctrl + Comma)  
**Action**: Opens the application settings interface

## Global Navigation Commands

### Tab Navigation
- **Tab**: Switch to next settings tab
- **Shift+Tab**: Switch to previous settings tab

### Item Navigation  
- **↑ / k**: Move up to previous item
- **↓ / j**: Move down to next item

### Selection and Editing
- **Enter / Space**: Select current item (toggle boolean settings or activate actions)
- **e**: Enter edit mode for text/numeric settings
- **Ctrl+R**: Reset current setting to default value
- **Ctrl+S**: Save all settings changes

### Closing Settings
- **Esc / q**: Close settings and return to main interface

## Edit Mode Commands

When in edit mode (after pressing 'e'):

- **Any character**: Add character to input buffer
- **Backspace**: Remove last character from input buffer  
- **Enter**: Apply changes and exit edit mode
- **Esc**: Cancel changes and exit edit mode

## Settings Tabs Overview

The settings interface contains 8 tabs with the following items:

### 1. General Tab (10 items)
| Index | Setting | Type | Description |
|-------|---------|------|-------------|
| 0 | Auto-sync emails | Boolean | Toggle automatic email synchronization |
| 1 | Sync interval | Editable | Set sync interval in minutes (1-1440) |
| 2 | Fetch on startup | Boolean | Toggle fetching emails on application start |
| 3 | Use incremental sync | Boolean | Toggle incremental synchronization mode |
| 4 | Max concurrent syncs | Editable | Set maximum concurrent sync operations (1-10) |
| 5 | Default folder | Editable | Set default email folder name |
| 6 | Confirm before delete | Boolean | Toggle delete confirmation dialog |
| 7 | Show notifications | Boolean | Toggle desktop notifications |
| 8 | Thread grouping | Editable | Configure email thread grouping method |
| 9 | Mark as read on reply | Boolean | Toggle auto-mark read when replying |

### 2. Accounts Tab (6 items)
| Index | Action | Description |
|-------|--------|-------------|
| 0 | Manage email accounts | Open account management interface |
| 1 | Test connection | Test current account connection |
| 2 | Configure OAuth | Setup OAuth authentication |
| 3 | Backup accounts | Export account configurations |
| 4 | Restore accounts | Import account configurations |
| 5 | Import accounts | Import accounts from other clients |

### 3. UI & Theme Tab (7 items)
| Index | Setting | Type | Description |
|-------|---------|------|-------------|
| 0 | Theme | Cycle | Switch between available themes |
| 1 | Compact mode | Boolean | Toggle compact interface layout |
| 2 | Show sidebar | Boolean | Toggle sidebar visibility |
| 3 | Show status bar | Boolean | Toggle status bar visibility |
| 4 | Font size | Editable | Set UI font size (8-24) |
| 5 | Animations | Boolean | Toggle UI animations |
| 6 | Configure layout | Action | Open layout configuration |

### 4. Keyboard Tab (5 items)
| Index | Action | Description |
|-------|--------|-------------|
| 0 | Configure shortcuts | Open keyboard shortcut editor |
| 1 | Reset to defaults | Restore default keyboard shortcuts |
| 2 | Import configuration | Load keyboard config from file |
| 3 | Export configuration | Save keyboard config to file |
| 4 | Vim mode | Boolean | Toggle Vim-style keybindings |

### 5. Performance Tab (6 items)
| Index | Setting | Type | Description |
|-------|---------|------|-------------|
| 0 | Cache size | Editable | Set cache size in MB |
| 1 | Preload images | Boolean | Toggle image preloading |
| 2 | Max concurrent | Editable | Set max concurrent operations (1-50) |
| 3 | Background sync | Boolean | Toggle background synchronization |
| 4 | Cleanup interval | Editable | Set cleanup interval in hours |
| 5 | Run cleanup now | Action | Execute immediate cleanup |

### 6. Privacy Tab (5 items)
| Index | Setting | Type | Description |
|-------|---------|------|-------------|
| 0 | Tracking protection | Boolean | Toggle tracking protection |
| 1 | External images | Boolean | Toggle external image loading |
| 2 | Data retention policy | Action | Configure data retention settings |
| 3 | Clear cache | Action | Clear application cache |
| 4 | Export user data | Action | Export user data for backup |

### 7. AI Assistant Tab (8 items) 
| Index | Setting | Type | Description |
|-------|---------|------|-------------|
| 0 | AI assistant | Boolean | Enable/disable AI features |
| 1 | Provider | Editable | Set AI provider (Ollama, GPT, etc.) |
| 2 | Privacy mode | Action | Configure AI privacy settings |
| 3 | Test connection | Action | Test AI service connection |
| 4 | Configure features | Action | Setup AI feature preferences |
| 5 | Cache settings | Action | Configure AI response caching |
| 6 | Performance settings | Action | Optimize AI performance |
| 7 | Advanced AI config | Action | Open full AI configuration |

### 8. Advanced Tab (6 items)
| Index | Setting | Type | Description |
|-------|---------|------|-------------|
| 0 | Debug mode | Boolean | Toggle debug logging |
| 1 | Logging configuration | Action | Configure log levels and output |
| 2 | Database maintenance | Action | Run database optimization |
| 3 | Reset all settings | Action | ⚠️ Reset entire configuration |
| 4 | Export configuration | Action | Backup complete settings |
| 5 | Import configuration | Action | Restore settings from backup |

## Setting Types Explained

- **Boolean**: Toggle settings that can be turned on/off using Enter or Space
- **Editable**: Text or numeric settings that require pressing 'e' to edit
- **Action**: Commands that execute immediately when selected with Enter or Space
- **Cycle**: Settings that cycle through multiple options when selected

## Status Messages

The settings interface displays status messages at the bottom when:
- Settings are modified (shows "* Modified")
- Actions are performed (shows temporary status)
- Validation errors occur (shows error messages)
- Edit mode is active (shows current input and instructions)

## Footer Help Text

The settings footer displays context-sensitive help:

**Normal Mode**: `Tab/Shift+Tab: Switch tabs | ↑↓: Navigate | Enter/Space: Select | E: Edit | Ctrl+R: Reset | Ctrl+S: Save | Q/Esc: Close`

**Edit Mode**: `Editing: [current input] | Enter: Save | Esc: Cancel`

## Testing Coverage

All commands have been verified through comprehensive automated tests covering:
- ✅ All keyboard shortcuts work as documented
- ✅ Navigation between tabs and items
- ✅ Edit mode functionality for all editable settings
- ✅ Proper key handling when settings UI is visible/hidden
- ✅ Status message management
- ✅ Complete workflow integration tests
- ✅ Keyboard shortcut conflict resolution (the 'e' key bug fix)

## Implementation Notes

- Settings UI uses `EventResult::Handled` to prevent key conflicts with global shortcuts
- All editable settings include validation with appropriate error messages
- Boolean settings provide immediate visual feedback when toggled
- The interface maintains state consistency across tab switches
- All shortcuts follow standard TUI conventions and vim-style alternatives where appropriate