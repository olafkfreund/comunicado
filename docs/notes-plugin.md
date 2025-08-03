# Comunicado Notes Plugin

A comprehensive note-taking plugin for Comunicado that provides advanced markdown support, wiki-style linking, full-text search, and seamless integration with email, calendar, and mobile workflows.

## Features

### 🗒️ Core Note Management
- **Markdown Support**: Full CommonMark-compliant parsing with frontmatter
- **Wiki Linking**: Bidirectional `[[note title]]` linking between notes
- **File Organization**: Flexible directory-based organization
- **Auto-Detection**: Automatic discovery of existing markdown files

### 🔍 Advanced Search
- **Full-Text Search**: Fast SQLite FTS5 search across all content
- **Filtering**: Search by tags, date ranges, file types, and more
- **Ranking**: Intelligent relevance scoring with TF-IDF
- **Categorized Search**: Target specific fields (title, content, tags)

### 🖥️ Terminal Interface (TUI)
- **Browse Mode**: Navigate notes with vim-style keybindings
- **Edit Mode**: Built-in markdown editor with syntax highlighting
- **Search Mode**: Interactive search with live filtering
- **Create Mode**: Quick note creation with templates

### 📧 Email Integration
- **Email to Notes**: Convert emails to markdown notes
- **Contact Linking**: Link notes to email contacts and threads
- **Thread Management**: Organize related email conversations
- **Smart Templates**: Automatic note generation from email content

### 📱 Mobile Integration
- **SMS to Notes**: Convert SMS messages to notes via KDE Connect
- **Mobile Sync**: Bidirectional synchronization with mobile devices
- **Notification Bridge**: Mobile notifications for note updates

### 📅 Calendar Integration
- **Meeting Notes**: Automatic note creation for calendar events
- **Event Linking**: Link notes to specific meetings and appointments
- **Action Items**: Track meeting action items and follow-ups
- **Attendee Management**: Notes linked to meeting participants

## Installation

The notes plugin is built into Comunicado and can be enabled through the plugin system:

1. **Enable the Plugin**:
   ```bash
   # Through the TUI settings
   Ctrl+, -> Plugins -> Notes -> Enable
   
   # Or via configuration file
   echo '{"notes": {"enabled": true}}' > ~/.config/comunicado/plugins.json
   ```

2. **Configure Note Directory**:
   ```toml
   # ~/.config/comunicado/config.toml
   [plugins.notes]
   default_directory = "~/Documents/Notes"
   auto_index = true
   vim_mode = true
   ```

## Quick Start

### Creating Your First Note

1. **Launch TUI Interface**:
   ```
   # In Comunicado, press Ctrl+N to open notes TUI
   ```

2. **Create a Note**:
   ```
   # In notes TUI:
   - Press 'n' to create new note
   - Enter title: "My First Note"
   - Start writing in markdown
   ```

3. **Wiki Linking**:
   ```markdown
   # My First Note
   
   This links to [[Another Note]] which will be created automatically.
   
   See also: [[Project Ideas]] and [[Meeting Notes/2025-01-15]]
   ```

### Basic Navigation

| Key | Action |
|-----|--------|
| `j/k` | Navigate up/down |
| `Enter` | Open selected note |
| `n` | Create new note |
| `e` | Edit current note |
| `/` | Search notes |
| `t` | Filter by tags |
| `q` | Quit/Back |

## Configuration

### Plugin Configuration

```toml
# ~/.config/comunicado/config.toml
[plugins.notes]
# Note storage directory
default_directory = "~/Documents/Notes"

# Automatically index new notes
auto_index = true

# Enable vim-style keybindings
vim_mode = true

# Maximum search results
max_search_results = 100

# Enable integrations
enable_email_integration = true
enable_mobile_integration = true
enable_calendar_integration = true

# TUI theme (default, dark, light)
tui_theme = "default"

# Auto-save interval in seconds
auto_save_interval = 30
```

### Search Configuration

```toml
[plugins.notes.search]
# Enable fuzzy matching
fuzzy_search = true

# Search ranking weights
title_weight = 3.0
content_weight = 1.0
tag_weight = 2.0
filename_weight = 1.5

# Recency boost for new notes
recency_boost = 0.1
recency_days = 30
```

### Email Integration

```toml
[plugins.notes.email]
# Automatically create notes for important emails
auto_create_threshold = "important"

# Email note template
template = """
# Email: {{subject}}

**From:** {{from}}
**Date:** {{date}}
**Thread:** [[{{thread_id}}]]

## Content

{{content}}

## Action Items

- [ ] 

## Notes

"""
```

## Advanced Usage

### Frontmatter Support

Notes support YAML frontmatter for metadata:

```markdown
---
title: "Project Planning"
tags: [work, planning, project]
created: 2025-01-15
priority: high
status: active
---

# Project Planning Notes

Content goes here...
```

### Wiki Linking Patterns

```markdown
# Different linking styles
[[Simple Link]]                    # Links to "Simple Link.md"
[[Custom Title|actual-filename]]   # Custom display text
[[Folder/Subfolder/Note]]         # Hierarchical organization
[[#Section Header]]                # Link to section in current note
[[Note#Section]]                   # Link to section in other note
```

### Search Queries

```
# Basic search
meeting notes

# Tag filtering
tag:work tag:important

# Date range
created:2025-01-01..2025-01-31

# Content type
type:markdown has:frontmatter

# Complex queries
(meeting OR discussion) AND tag:work created:>2025-01-01
```

### Email Integration Workflows

1. **Email to Note**:
   ```
   # In email view, press Ctrl+N
   # Select "Create Note from Email"
   # Choose template and customize
   ```

2. **Link Existing Note**:
   ```
   # In email view, press Ctrl+L
   # Search for existing note
   # Creates bidirectional link
   ```

3. **Thread Notes**:
   ```
   # Automatically groups emails by thread
   # Creates master note for email conversation
   # Links individual emails as sub-notes
   ```

## Keyboard Shortcuts

### Global Shortcuts
- `Ctrl+N` - Open notes TUI
- `Ctrl+Shift+N` - Quick note creation
- `Ctrl+Shift+F` - Global note search

### TUI Navigation
- `j/k` - Move up/down
- `h/l` - Navigate directories
- `Enter` - Open/select
- `Esc/q` - Back/quit
- `g/G` - Top/bottom
- `Ctrl+D/U` - Page down/up

### TUI Actions
- `n` - New note
- `e` - Edit note
- `d` - Delete note
- `r` - Rename note
- `y` - Copy note
- `p` - Paste/move note
- `/` - Search
- `t` - Filter by tags
- `s` - Sort options
- `?` - Help

### Editor Mode
- `Ctrl+S` - Save
- `Ctrl+Q` - Quit without saving
- `Ctrl+Z` - Undo
- `Ctrl+Y` - Redo
- `Ctrl+F` - Find in note
- `Ctrl+R` - Find and replace

## Integration Examples

### Meeting Note Template

```markdown
---
title: "{{meeting_title}}"
tags: [meeting, {{project_tag}}]
date: {{meeting_date}}
attendees: {{attendee_list}}
calendar_event: {{event_id}}
---

# {{meeting_title}}

**Date:** {{meeting_date}}
**Attendees:** {{attendees}}
**Duration:** {{duration}}

## Agenda

{{agenda_items}}

## Discussion

## Decisions

## Action Items

{{#each action_items}}
- [ ] {{description}} (@{{assignee}} due: {{due_date}})
{{/each}}

## Next Meeting

**Date:** {{next_meeting_date}}
**Focus:** 
```

### Email Note Template

```markdown
---
title: "Email: {{subject}}"
tags: [email, {{auto_tags}}]
email_id: {{message_id}}
thread_id: {{thread_id}}
from: {{sender_email}}
---

# Email: {{subject}}

**From:** {{sender_name}} <{{sender_email}}>
**Date:** {{date}}
**Thread:** [[Email Thread: {{thread_subject}}]]

## Summary

{{ai_summary}}

## Original Content

{{content}}

## Follow-up Actions

- [ ] 

## Related Notes

{{#each related_notes}}
- [[{{title}}]]
{{/each}}
```

## Troubleshooting

### Common Issues

1. **Notes not appearing in search**:
   ```bash
   # Rebuild search index
   comunicado --plugin notes --reindex
   ```

2. **TUI not responsive**:
   ```bash
   # Check terminal compatibility
   echo $TERM
   # Should be: xterm-256color, screen-256color, or similar
   ```

3. **File watching not working**:
   ```bash
   # Check inotify limits on Linux
   cat /proc/sys/fs/inotify/max_user_watches
   # Increase if needed:
   echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf
   ```

4. **Mobile integration not connecting**:
   ```bash
   # Check KDE Connect status
   kdeconnect-cli --list-available
   # Pair device if needed:
   kdeconnect-cli --pair --device <device-id>
   ```

### Debug Mode

Enable debug logging for troubleshooting:

```toml
[plugins.notes]
log_level = "debug"

[plugins.notes.debug]
log_file_operations = true
log_search_queries = true
log_integration_events = true
```

## Plugin Development

### Extending the Plugin

The notes plugin supports extensions through hooks:

```rust
// Example: Custom note processor
pub fn register_note_processor(processor: Box<dyn NoteProcessor>) {
    // Implementation
}

// Example: Custom search provider
pub fn register_search_provider(provider: Box<dyn SearchProvider>) {
    // Implementation
}
```

### API Reference

See the full API documentation at [docs.rs/comunicado](https://docs.rs/comunicado) for detailed interface documentation.

## License

The notes plugin is part of Comunicado and is licensed under the same terms as the main application.