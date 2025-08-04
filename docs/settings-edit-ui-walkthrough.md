# Settings Edit UI Walkthrough

This document shows exactly what happens when you press "e" to edit settings in the Comunicado application.

## Visual Demo: Editing AI Provider Setting

### Step 1: Navigate to AI Settings
```
┌─────────────────────────── ⚙️ Application Settings ────────────────────────────┐
│ General │ Accounts │ UI & Theme │ Keyboard │ Performance │ Privacy │ AI Assistant │ Advanced │
│                                                                    ^^^^^^^^^^^^          │
│ ┌─ AI Assistant Settings ──────────────────────────────────────────────────────┐      │
│ │ ► 🤖 AI assistant: Enabled                                                    │      │
│ │   🔧 Provider: Ollama          ← Currently selected item                     │      │
│ │   🔒 Privacy mode: Local only                                                │      │
│ │   🔍 Test connection                                                          │      │
│ │   ⚙️ Configure features                                                      │      │
│ │   💾 Cache settings                                                          │      │
│ │   ⚡ Performance settings                                                    │      │
│ │   🛠️ Advanced AI config                                                     │      │
│ └───────────────────────────────────────────────────────────────────────────────┘      │
│ Tab/Shift+Tab: Switch tabs | ↑↓: Navigate | Enter/Space: Select | E: Edit | Ctrl+R: Reset | Ctrl+S: Save | Q/Esc: Close │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### Step 2: Press 'e' to Enter Edit Mode
```
┌─────────────────────────── ⚙️ Application Settings ────────────────────────────┐
│ General │ Accounts │ UI & Theme │ Keyboard │ Performance │ Privacy │ AI Assistant │ Advanced │
│                                                                    ^^^^^^^^^^^^          │
│ ┌─ AI Assistant Settings ──────────────────────────────────────────────────────┐      │
│ │ ► 🤖 AI assistant: Enabled                                                    │      │
│ │   🔧 Provider: Ollama          ← Still selected, now in edit mode            │      │
│ │   🔒 Privacy mode: Local only                                                │      │
│ │   🔍 Test connection                                                          │      │
│ │   ⚙️ Configure features                                                      │      │
│ │   💾 Cache settings                                                          │      │
│ │   ⚡ Performance settings                                                    │      │
│ │   🛠️ Advanced AI config                                                     │      │
│ └───────────────────────────────────────────────────────────────────────────────┘      │
│ Editing:  | Enter: Save | Esc: Cancel         ← Footer shows edit mode          │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### Step 3: Type New Value (e.g., "ChatGPT")
```
┌─────────────────────────── ⚙️ Application Settings ────────────────────────────┐
│ General │ Accounts │ UI & Theme │ Keyboard │ Performance │ Privacy │ AI Assistant │ Advanced │
│                                                                    ^^^^^^^^^^^^          │
│ ┌─ AI Assistant Settings ──────────────────────────────────────────────────────┐      │
│ │ ► 🤖 AI assistant: Enabled                                                    │      │
│ │   🔧 Provider: Ollama          ← Display doesn't change yet                  │      │
│ │   🔒 Privacy mode: Local only                                                │      │
│ │   🔍 Test connection                                                          │      │
│ │   ⚙️ Configure features                                                      │      │
│ │   💾 Cache settings                                                          │      │
│ │   ⚡ Performance settings                                                    │      │
│ │   🛠️ Advanced AI config                                                     │      │
│ └───────────────────────────────────────────────────────────────────────────────┘      │
│ Editing: ChatGPT | Enter: Save | Esc: Cancel   ← Your input appears here       │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### Step 4: Press Enter to Save
```
┌─────────────────────────── ⚙️ Application Settings ────────────────────────────┐
│ General │ Accounts │ UI & Theme │ Keyboard │ Performance │ Privacy │ AI Assistant │ Advanced │
│                                                                    ^^^^^^^^^^^^          │
│ ┌─ AI Assistant Settings ──────────────────────────────────────────────────────┐      │
│ │ ► 🤖 AI assistant: Enabled                                                    │      │
│ │   🔧 Provider: Ollama          ← Display still shows old value               │      │
│ │   🔒 Privacy mode: Local only                                                │      │
│ │   🔍 Test connection                                                          │      │
│ │   ⚙️ Configure features                                                      │      │
│ │   💾 Cache settings                                                          │      │
│ │   ⚡ Performance settings                                                    │      │
│ │   🛠️ Advanced AI config                                                     │      │
│ └───────────────────────────────────────────────────────────────────────────────┘      │
│ * Modified | Status: AI provider set to 'ChatGPT' | Tab/Shift+Tab: Switch tabs | ↑↓: Navigate | Enter/Space: Select | E: Edit | Ctrl+R: Reset | Ctrl+S: Save | Q/Esc: Close │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

## What Actually Happens Behind the Scenes

### Edit Mode State Changes:
1. **Press 'e'**: `edit_mode = true`, `input_buffer.clear()`
2. **Type characters**: Each character gets added to `input_buffer`
3. **Press Enter**: Calls `apply_edit()` → `apply_ai_edit()` → Shows status message
4. **Exit edit mode**: `edit_mode = false`, `input_buffer.clear()`

### Current Limitations:

#### ❌ **Visual Display Issues**
- The setting values shown are **hardcoded strings** in render functions
- Changing a setting doesn't update the displayed value
- Example: AI Provider always shows "Ollama" regardless of actual setting

#### ❌ **Limited Edit Support**  
**Only these settings actually have edit functionality:**

**General Tab (3/10 editable):**
- Sync interval (validates 1-1440 minutes)
- Max concurrent syncs (validates 1-10) 
- Default folder (accepts any text)

**UI Tab (1/7 editable):**
- Font size (validates 8-24)

**Performance Tab (3/6 editable):**
- Cache size (accepts any number)
- Max concurrent operations (validates 1-50)
- Cleanup interval (accepts any number in hours)

**AI Tab (1/8 editable):** ← **Just added!**
- AI Provider (accepts any non-empty text)

**Other tabs:** No edit functionality at all!

#### ❌ **No Real Persistence**
- Changes only show status messages
- Settings don't persist between sessions
- No actual configuration file updates

## Improved UI Mockup (Future Enhancement)

Here's what the edit experience *should* look like:

### Enhanced Edit Mode with Inline Editing:
```
┌─────────────────────────── ⚙️ Application Settings ────────────────────────────┐
│ General │ Accounts │ UI & Theme │ Keyboard │ Performance │ Privacy │ AI Assistant │ Advanced │
│ ┌─ AI Assistant Settings ──────────────────────────────────────────────────────┐      │
│ │   🤖 AI assistant: Enabled                                                    │      │
│ │ ► 🔧 Provider: [ChatGPT________________] ← Inline text input box              │      │
│ │   🔒 Privacy mode: Local only                                                │      │
│ │   🔍 Test connection                                                          │      │
│ └───────────────────────────────────────────────────────────────────────────────┘      │
│ * Modified | Editing AI Provider | Enter: Save | Esc: Cancel | Tab: Next Field │      │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

## Current Working Settings Summary

✅ **Working Edit Settings (9 total):**
- General: Sync interval, Max concurrent syncs, Default folder  
- UI: Font size
- Performance: Cache size, Max concurrent operations, Cleanup interval
- **AI: Provider** ← Just implemented!

❌ **Missing Edit Settings (38 total):**
- All boolean toggles (should use Enter/Space, not 'e')
- All action buttons (should use Enter/Space, not 'e') 
- Most text settings in other tabs

## Recommendations

1. **Fix Visual Feedback**: Make setting displays show actual current values
2. **Add Missing Editors**: Implement edit functions for remaining text/numeric settings  
3. **Add Persistence**: Save changes to actual configuration files
4. **Improve UX**: Add inline editing with proper input validation UI
5. **Better Keybindings**: Use 'e' only for text/numeric, Enter/Space for booleans/actions