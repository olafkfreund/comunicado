# Email Management

Comunicado provides a comprehensive set of tools for managing your email efficiently from the terminal. This guide covers everything from basic reading and composition to advanced organization and workflow features.

## Interface Overview

When you start Comunicado, you'll see the main email interface with several panels:

**Folder Panel (Left)**
Shows your email folders in a tree structure. IMAP folders are synchronized and displayed with unread counts. You can expand and collapse folder hierarchies and see real-time updates as new mail arrives.

**Message List (Center)**
Displays messages in the selected folder with key information like sender, subject, date, and status indicators. Messages are sorted by date by default, but you can change the sorting criteria.

**Message Preview (Right)**
Shows the content of the selected message with proper formatting, images, and attachments. The preview updates immediately as you navigate through your message list.

**Status Bar (Bottom)**
Provides information about the current state, available shortcuts, and system status including sync progress and network connectivity.

## Reading Email

### Basic Navigation

Moving through your email is designed to be fast and intuitive. Use `j` and `k` to move up and down through messages, just like in vim or less. The preview panel updates automatically as you select different messages.

When you want to read a message in detail, press `Enter` to open it in full view. You can scroll through longer messages using `Space` to go forward and `Shift+Space` to go backward. Press `Esc` to return to the message list.

### Message Display Options

Comunicado offers several ways to view your messages:

**Formatted View** (default)
Messages are displayed with proper formatting, colors, and structure. HTML emails are converted to terminal-friendly format while preserving the important visual elements.

**Raw View**
Press `v` to toggle to raw view, which shows the original message source including all headers. This is useful for troubleshooting or when you need to see technical details.

**Header Toggle**
Press `h` to show or hide detailed message headers. By default, Comunicado shows only the essential headers (From, To, Subject, Date), but you can expand this to see all headers including routing information and authentication results.

### Handling Different Content Types

**Plain Text Messages**
Plain text emails are displayed with proper line wrapping and formatting. URLs are highlighted and can be opened with external applications.

**HTML Messages**
HTML emails are converted to terminal format while preserving:
- Text formatting (bold, italic, colors)
- Links and hypertext
- Basic layout and structure
- Embedded images (in compatible terminals)

**Rich Media**
- Images are displayed inline when your terminal supports graphics
- GIF animations play automatically in compatible terminals
- Attachments are listed at the bottom of the message
- Links to external content are highlighted

## Composing Email

### Starting a New Message

Press `c` to start composing a new message. This opens the compose interface with fields for recipients, subject, and message body. Use `Tab` to move between fields and start typing.

The compose interface includes:
- **To**: Primary recipients (required)
- **Cc**: Carbon copy recipients
- **Bcc**: Blind carbon copy recipients
- **Subject**: Message subject line
- **Body**: The message content

### Writing Your Message

The message body supports both plain text and basic formatting. As you type, Comunicado provides:
- Automatic line wrapping
- Spell checking (if enabled)
- Basic text formatting shortcuts
- Real-time character and word count

You can paste content from the system clipboard using `Ctrl+Shift+V`, and cut/copy/paste within the message using standard shortcuts.

### Attachments

To add attachments to your message:
1. Press `Ctrl+A` while composing
2. Navigate to the file you want to attach
3. Press `Enter` to select it
4. Repeat for multiple attachments

Attached files are shown at the bottom of the compose window with their names and sizes. You can remove attachments by selecting them and pressing `Delete`.

### Sending and Saving

When your message is ready:
- `Ctrl+Enter` - Send the message immediately
- `Ctrl+S` - Save as draft for later
- `Ctrl+X` - Discard the message

Comunicado automatically saves drafts periodically, so you won't lose your work if something unexpected happens.

## Message Actions

### Reply and Forward

**Reply** (`r`)
Responds to the sender only. The original message is quoted below your response, and the subject line gets "Re:" prepended if it wasn't already there.

**Reply All** (`R`)
Responds to the sender and all other recipients from the original message. Use this carefully to avoid unnecessary email traffic.

**Forward** (`f`)
Sends the message to new recipients. The original message is included in its entirety, and the subject gets "Fwd:" prepended.

### Organization Actions

**Delete** (`d`)
Moves the message to the trash folder. Depending on your account settings, messages may be permanently deleted after a certain period.

**Archive** (`a`)
Removes the message from your inbox but keeps it accessible. Most email providers have an archive folder where these messages are stored.

**Mark as Spam** (`s`)
Reports the message as spam and moves it to the spam folder. This helps train your email provider's spam filters.

**Flag/Star** (`*`)
Marks the message as important or flagged for follow-up. Flagged messages are easily identifiable and can be filtered or searched.

**Mark as Unread** (`u`)
Changes a read message back to unread status. Useful for marking messages that need attention later.

### Moving and Copying

**Move to Folder** (`m`)
Opens a folder selector where you can choose a destination folder. The message is moved from its current location to the selected folder.

**Copy to Folder** (`c` in folder context)
Creates a copy of the message in another folder while leaving the original in place.

## Folder Management

### Folder Navigation

Your email folders are displayed in a hierarchical tree on the left side of the interface. Folders with unread messages show the unread count in parentheses.

Navigate folders using:
- `j`/`k` - Move up and down
- `Enter` - Open the selected folder
- `Space` - Expand or collapse a folder with subfolders

### Folder Operations

**Create New Folder** (`Ctrl+N`)
Creates a new folder at the current level. You'll be prompted for a folder name. Subfolders can be created by selecting a parent folder first.

**Rename Folder** (`r`)
Changes the name of the selected folder. This operation syncs with your email server and may take a moment to complete.

**Delete Folder** (`d`)
Removes a folder and all its contents. Comunicado will ask for confirmation before proceeding with this destructive operation.

### Special Folders

Comunicado recognizes and handles special folders automatically:
- **Inbox** - New incoming messages
- **Sent** - Messages you've sent
- **Drafts** - Unfinished messages
- **Trash** - Deleted messages
- **Spam/Junk** - Suspected spam messages
- **Archive** - Archived messages

These folders may have different names depending on your email provider, but Comunicado maps them correctly.

## Search and Filtering

### Basic Search

Press `/` to open the search interface. You can search across:
- Message subjects
- Sender and recipient addresses
- Message body content
- Attachment names

Search results are displayed in a special view that shows matches from all folders. Each result shows the folder location and key message details.

### Advanced Search

For more complex searches, use `Ctrl+Shift+F` to open the advanced search dialog. This allows you to:
- Search specific date ranges
- Filter by message status (read/unread, flagged, etc.)
- Search specific folders only
- Combine multiple search criteria

### Saved Searches

You can save frequently used searches for quick access:
1. Perform a search
2. Press `s` in the search results
3. Give your search a name
4. Access it later from the search menu

## Email Threading

### Understanding Threads

Comunicado automatically groups related messages into conversation threads. Messages are threaded based on:
- Reply chains (In-Reply-To headers)
- Subject line matching
- Reference headers
- Message-ID relationships

### Thread Display

When thread view is enabled (press `t` to toggle), you'll see:
- Thread subjects with message counts
- Expandable conversation trees
- Chronological message ordering within threads
- Visual indicators for thread status

### Thread Navigation

Within a threaded conversation:
- `]` - Next message in thread
- `[` - Previous message in thread
- `Enter` - Expand or collapse the thread
- `Ctrl+]` - Jump to next thread
- `Ctrl+[` - Jump to previous thread

### Thread Actions

You can perform actions on entire threads:
- `Shift+D` - Delete entire conversation
- `Shift+A` - Archive entire conversation
- `Shift+U` - Mark entire thread as unread
- `T` - Select/deselect entire thread

## Multiple Account Management

### Account Switching

If you have multiple email accounts configured:
- `Ctrl+1` through `Ctrl+9` - Switch to specific accounts
- `Ctrl+Tab` - Cycle through accounts
- Account names are shown in the status bar

### Unified Views

Comunicado can show messages from multiple accounts in unified views:
- **Unified Inbox** - All incoming messages
- **Unified Sent** - All sent messages
- **Unified Search** - Search across all accounts

### Account-Specific Actions

Some actions are account-specific:
- Folder creation and management
- Server synchronization
- Account settings and authentication
- Signature and identity settings

## Synchronization

### Automatic Sync 🆕

Comunicado provides comprehensive automatic synchronization with your email servers:

**Real-time Sync**
- New messages are downloaded in real-time using IMAP IDLE
- Sent messages are uploaded to the server immediately
- Folder changes are synchronized automatically 
- Message flags and status updates are synced instantly

**Background Sync** (New Feature)
- Configurable automatic sync intervals (1 minute to 24 hours)
- Optional startup sync when launching the application
- Incremental sync for efficient bandwidth usage
- Concurrent sync limits to prevent system overload
- Automatic retry with exponential backoff for failed syncs

**Sync Configuration**
Access sync settings through the Settings panel (Ctrl+,) → General tab:
- 🔄 **Auto-sync emails**: Enable/disable automatic synchronization
- ⏱️ **Sync interval**: Configure how often to sync (default: 15 minutes)
- 🚀 **Fetch on startup**: Sync immediately when app starts
- 📬 **Use incremental sync**: Download only new/changed messages
- 🔁 **Max concurrent syncs**: Limit simultaneous sync operations (1-10)

All automatic sync operations run in the background without blocking the user interface.

### Manual Sync

You can force synchronization at any time:
- `Ctrl+R` - Refresh current folder
- `Ctrl+Shift+S` - Sync all folders and accounts
- Settings → Force sync specific account or all accounts
- Sync status is shown in the status bar

### Offline Mode

When network connectivity is limited:
- Read previously downloaded messages
- Compose messages (queued for sending)
- Perform local operations (search, organize)
- Changes sync automatically when connection is restored

## Performance and Efficiency

### Keyboard-Driven Workflow

Comunicado is designed for efficiency through keyboard shortcuts. Common workflows become muscle memory:
- Reading: `j`/`k` to navigate, `Enter` to read, `Esc` to go back
- Responding: `r` to reply, type response, `Ctrl+Enter` to send
- Organizing: `d` to delete, `a` to archive, `m` to move

### Bulk Operations

Select multiple messages for bulk operations:
- `Ctrl+A` - Select all messages in current view
- `Space` - Toggle selection of current message
- `Shift+Click` - Select range (if using mouse)

Perform actions on selected messages:
- `d` - Delete all selected
- `a` - Archive all selected
- `m` - Move all selected to folder

### Quick Actions

Comunicado provides quick actions for common tasks:
- Quick reply templates
- Preset folder destinations
- Saved search filters
- Custom keyboard shortcuts

This comprehensive email management system makes Comunicado suitable for everything from casual email use to heavy professional communication, all while maintaining the speed and efficiency that terminal applications are known for.