# Complete Keyboard Shortcuts Reference

> **Last Updated**: 2025-08-02  
> **Implementation Status**: 93% Complete (71/76 actions working)  
> **Context-Aware**: Yes - shortcuts adapt to current UI focus

## 📊 Quick Stats

- **Total Shortcuts**: 76 keyboard actions
- **Fully Working**: 71 actions (93%)
- **Context-Dependent**: 13 actions
- **Missing Implementation**: 5 actions
- **Global Shortcuts**: 25 actions

## 🎯 How Context-Aware Shortcuts Work

Comunicado uses **smart context detection** - the same key can do different things depending on where you are:

- **'r'** - Reply to email (message list) or Sort by sender (global)
- **'d'** - Delete email (email viewer) or Delete folder (folder tree)
- **'e'** - Create event (calendar) or Edit email (email viewer)

The help system (?) shows only relevant shortcuts for your current context.

---

## 🌍 Global Shortcuts
*Work everywhere in the application*

| Key | Action | Description |
|-----|--------|-------------|
| **q** | Quit | Exit application |
| **Ctrl+C** | Force Quit | Force exit application |
| **?** | Help | Show keyboard shortcuts help |
| **Tab** | Next Pane | Move to next UI pane |
| **Shift+Tab** | Previous Pane | Move to previous UI pane |
| **↑/k** | Move Up | Navigate up in lists |
| **↓/j** | Move Down | Navigate down in lists |
| **←/h** | Move Left | Navigate left/previous |
| **→/l** | Move Right | Navigate right/next |
| **Enter** | Select | Select current item |
| **Esc** | Escape | Cancel/escape current operation |
| **Space** | Toggle Expanded | Expand/collapse current item |

---

## 📧 Email Management

### Message List Actions
*Available when message list is focused*

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **c** | Compose | ✅ | Compose new email |
| **Ctrl+D** | Show Drafts | ✅ | Show draft email list |
| **Ctrl+R** | Reply | ✅ | Reply to current message |
| **Shift+R** | Reply All | ✅ | Reply to all recipients |
| **Ctrl+F** | Forward | ✅ | Forward current message |
| **Shift+Del** | Delete | ✅ | Delete current message |
| **Shift+A** | Archive | ✅ | Archive current message |
| **Shift+M** | Mark Read | ✅ | Mark message as read |
| **Shift+U** | Mark Unread | ✅ | Mark message as unread |
| **n** | Next Message | ✅ | Navigate to next message |
| **p** | Previous Message | ✅ | Navigate to previous message |

### Email Viewer Mode
*Available only when viewing an email in full-screen mode*

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **r** | Reply | ✅ | Reply to viewed email |
| **Shift+R** | Reply All | ✅ | Reply to all recipients |
| **f** | Forward | ✅ | Forward viewed email |
| **e** | Edit | ✅ | Edit email (if draft) |
| **d** | Delete | ✅ | Delete viewed email |
| **a** | Archive | ✅ | Archive viewed email |
| **m** | Mark Read | ✅ | Mark as read |
| **u** | Mark Unread | ✅ | Mark as unread |
| **Esc** | Close Viewer | ✅ | Exit email viewer |

### Search and Filtering

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **/** | Start Search | ✅ | Search messages |
| **f** | Folder Search | ✅ | Search in folders |
| **Esc** | End Search | ✅ | Clear/exit search |

### View Controls

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **t** | Toggle Threading | ✅ | Enable/disable threaded view |
| **o** | Expand Thread | ✅ | Expand email thread |
| **C** | Collapse Thread | ✅ | Collapse email thread |
| **m** | Toggle View Mode | ✅ | Switch preview modes |
| **H** | Toggle Headers | ✅ | Show/hide email headers |
| **V** | Email Viewer | ✅ | Open full email viewer |

### Sorting

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **s** | Sort by Date | ✅ | Sort messages by date |
| **r** | Sort by Sender | ✅ | Sort messages by sender |
| **u** | Sort by Subject | ✅ | Sort messages by subject |

---

## 📎 Attachments
*Available when email with attachments is selected*

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **a** | First Attachment | ✅ | Select first attachment |
| **A** | View Attachment | ✅ | View selected attachment |
| **S** | Save Attachment | ❌ | Save attachment to disk |
| **O** | Open With System | ✅ | Open with system default app |
| **Ctrl+J** | Next Attachment | ✅ | Navigate to next attachment |
| **Ctrl+K** | Previous Attachment | ✅ | Navigate to previous attachment |

---

## 🗂️ Folder Management
*Available when folder tree is focused*

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+N** | Create Folder | ✅ | Create new folder |
| **d** | Delete Folder | ✅ | Delete selected folder |
| **R** | Refresh Folder | ✅ | Refresh folder contents |
| **Alt+R** | Function Refresh | ✅ | Folder refresh (F-key alternative) |
| **Alt+N** | Function Rename | ✅ | Rename folder (F-key alternative) |
| **Del** | Function Delete | ✅ | Delete folder (F-key alternative) |

---

## 👤 Account Management

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+A** | Add Account | ✅ | Add new email account |
| **Ctrl+X** | Remove Account | ✅ | Remove current account |
| **Ctrl+Shift+R** | Refresh Account | ✅ | Refresh account connection |
| **Ctrl+S** | Switch Account | ✅ | Switch to next account |

---

## 📅 Calendar
*Available in calendar mode*

### Navigation

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+L** | Show Calendar | ✅ | Switch to calendar view |
| **Ctrl+M** | Show Email | ✅ | Switch to email view |
| **←** | Previous Month | ✅ | Navigate to previous month |
| **→** | Next Month | ✅ | Navigate to next month |
| **. (period)** | Today | ✅ | Go to today's date |

### View Modes

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **1** | Day View | ✅ | Switch to day view |
| **2** | Week View | ✅ | Switch to week view |
| **3** | Month View | ✅ | Switch to month view |
| **4** | Agenda View | ✅ | Switch to agenda view |

### Event Management

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **e** | Create Event | ✅ | Create new calendar event |
| **Ctrl+E** | Edit Event | ✅ | Edit selected event |
| **Del** | Delete Event | ✅ | Delete selected event |
| **Space** | View Details | ❌ | View event details (coming soon) |

### Todo Management

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **T** | Create Todo | ✅ | Create new todo item |
| **Space** | Toggle Complete | ✅ | Mark todo complete/incomplete |
| **Ctrl+T** | View Todos | ❌ | View all todos (coming soon) |

---

## 🤖 AI Assistant
*AI-powered features with privacy controls*

### Email AI

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+Alt+I** | Toggle AI Panel | ✅ | Show/hide AI assistant |
| **Ctrl+Alt+S** | Email Suggestions | ✅ | Get AI email suggestions |
| **Ctrl+Alt+U** | Summarize Email | ✅ | AI email summarization |
| **Ctrl+Alt+R** | Quick Reply | ✅ | Generate AI reply suggestions |
| **Ctrl+Alt+A** | Email Analysis | ✅ | Analyze email content with AI |

### Compose AI

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+Alt+C** | Compose Assist | ✅ | AI compose suggestions |
| **Ctrl+Alt+E** | Content Generation | ✅ | Generate email content |

### Calendar AI

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+Alt+L** | Calendar Assist | ✅ | AI calendar assistance |
| **Ctrl+Alt+T** | Schedule Parser | ✅ | Parse schedule requests |

### AI Configuration

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+Alt+G** | AI Settings | ✅ | Open AI configuration |

---

## 👥 Contacts
*Contact management features*

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+K** | Contacts Popup | ✅ | Open contacts manager |
| **i** | View Contact | ⚠️ | View sender contact (needs message) |
| **Ctrl+I** | Edit Contact | ⚠️ | Edit sender contact (needs message) |
| **+** | Add Contact | ⚠️ | Add sender to contacts (needs message) |
| **-** | Remove Contact | ⚠️ | Remove from contacts (needs message) |
| **Shift+C** | Quick Actions | ⚠️ | Contact quick actions (needs message) |

---

## 📋 Copy Operations

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Ctrl+Y** | Copy Email | ✅ | Copy email content to clipboard |
| **Alt+C** | Copy Attachment | ✅ | Copy attachment info to clipboard |

---

## 🎛️ Content Navigation

| Key | Action | Status | Description |
|-----|--------|--------|-------------|
| **Home** | Scroll to Top | ✅ | Scroll content to beginning |
| **End** | Scroll to Bottom | ✅ | Scroll content to end |

---

## ⚠️ Context Requirements

### 📍 Context-Dependent Shortcuts

Many shortcuts require specific contexts to work:

#### Requires Selected Message
- Most email actions (Reply, Forward, Delete, etc.)
- AI email operations (Summarize, Analysis, etc.)
- Contact operations (View, Add, Remove)

#### Requires Specific UI Focus
- **Folder Tree**: Folder operations (Create, Delete, Rename)
- **Message List**: Message navigation and selection
- **Attachment View**: Attachment operations
- **Calendar Mode**: Calendar-specific operations
- **Email Viewer**: Email viewer specific actions

#### Requires Specific Mode
- **Compose Mode**: Compose-specific AI features
- **Email Mode**: Email-related shortcuts
- **Calendar Mode**: Calendar-related shortcuts

---

## 🔧 Troubleshooting

### Shortcut Not Working?

1. **Check Context**: Is the right UI element focused?
2. **Check Mode**: Are you in the correct mode (Email/Calendar)?
3. **Check Selection**: Is there a message/event selected?
4. **Check Implementation**: Some features are still in development

### Help Commands

- **?** - Shows context-appropriate shortcuts
- **Ctrl+Alt+G** - AI configuration and feature status
- **Esc** - Usually cancels current operation

---

## 🚧 Coming Soon

These features are planned for future releases:

- **Save Attachment** (S) - Currently shows "coming soon"
- **View Event Details** (Space in calendar) - UI implementation pending
- **View Todos** (Ctrl+T) - UI implementation pending

---

## 🎮 Pro Tips

### Vim-Style Navigation
- Use **h/j/k/l** for navigation (left/down/up/right)
- All arrow keys work too for non-vim users

### AI Features
- Most AI features require valid configuration
- Use **Ctrl+Alt+G** to set up AI providers
- Local AI (Ollama) available for privacy

### Context Awareness
- The **?** key shows only relevant shortcuts for your current context
- Focus indicators show which pane is active
- Shortcuts adapt intelligently to your current workflow

### Efficiency Tips
- Use **Tab/Shift+Tab** to navigate between UI panes
- Use **Enter** to select, **Esc** to cancel
- Learn the AI shortcuts - they significantly boost productivity

---

*This documentation reflects the current state of keyboard shortcuts in Comunicado. For the most up-to-date information, press **?** in the application to see context-specific shortcuts.*