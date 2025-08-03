# Notes Plugin Configuration Examples

This document provides comprehensive configuration examples for the Comunicado Notes Plugin.

## Basic Configuration

### Minimal Setup

```toml
# ~/.config/comunicado/config.toml
[plugins.notes]
default_directory = "~/Documents/Notes"
enabled = true
```

### Recommended Setup

```toml
# ~/.config/comunicado/config.toml
[plugins.notes]
# Core settings
default_directory = "~/Documents/Notes"
enabled = true
auto_index = true
max_search_results = 100

# TUI preferences  
vim_mode = true
tui_theme = "default"
auto_save_interval = 30

# Enable integrations
enable_email_integration = true
enable_mobile_integration = true
enable_calendar_integration = true

# File watching
watch_directories = [
    "~/Documents/Notes",
    "~/Projects/*/docs",
    "~/Sync/Notes"
]
```

## Advanced Search Configuration

### Search Engine Tuning

```toml
[plugins.notes.search]
# Enable advanced features
fuzzy_search = true
highlight_results = true
max_snippet_length = 200

# Ranking weights (higher = more important)
title_weight = 3.0
content_weight = 1.0
tag_weight = 2.5
filename_weight = 1.5

# Recency boost for newer notes
recency_boost = 0.15
recency_days = 30

# Link popularity boost
link_popularity_boost = 0.2

# Performance settings
cache_size_mb = 50
cache_ttl_minutes = 15
max_concurrent_searches = 5

# Full-text search configuration
fts_tokenizer = "unicode61"
fts_remove_diacritics = true
fts_categories = ["title", "content", "tags"]
```

### Search Filters

```toml
[plugins.notes.search.filters]
# Default filters for different contexts
work = { tags = ["work"], exclude_tags = ["personal"] }
personal = { tags = ["personal"], exclude_tags = ["work"] }
recent = { date_range = "7d" }
important = { tags = ["important", "urgent"] }

# File type filters
markdown_only = { file_extensions = [".md", ".markdown"] }
with_frontmatter = { has_frontmatter = true }
linked_notes = { has_wiki_links = true }
```

## Email Integration

### Basic Email Settings

```toml
[plugins.notes.email]
# Enable automatic note creation
auto_create_enabled = true
auto_create_threshold = "important"  # none, important, all

# Email note organization
organize_by_thread = true
organize_by_sender = false
organize_by_date = false

# Note naming pattern
note_title_pattern = "Email: {{subject}}"
note_filename_pattern = "{{date}}-email-{{subject_slug}}.md"

# Content processing
include_headers = false
include_attachments = true
convert_html = true
preserve_formatting = true
```

### Email Templates

```toml
[plugins.notes.email.templates]
# Default email note template
default = """
---
title: "{{title}}"
tags: [email, {{auto_tags}}]
email_id: "{{message_id}}"
thread_id: "{{thread_id}}"
from: "{{sender_email}}"
date: "{{date}}"
---

# {{subject}}

**From:** {{sender_name}} <{{sender_email}}>
**To:** {{recipients}}
**Date:** {{date}}
**Thread:** [[{{thread_title}}]]

## Summary

{{ai_summary}}

## Content

{{content}}

## Action Items

- [ ] 

## Follow-up

## Related Notes

{{related_notes}}
"""

# Meeting request template
meeting = """
---
title: "Meeting: {{subject}}"
tags: [meeting, email, {{project_tags}}]
meeting_date: "{{meeting_date}}"
meeting_time: "{{meeting_time}}"
attendees: {{attendees}}
---

# Meeting: {{subject}}

**Organizer:** {{organizer}}
**Date:** {{meeting_date}} at {{meeting_time}}
**Duration:** {{duration}}
**Location:** {{location}}

## Attendees

{{#each attendees}}
- {{name}} <{{email}}>
{{/each}}

## Agenda

{{agenda}}

## Preparation

- [ ] 

## Notes

## Action Items

- [ ] 

## Follow-up Meeting

Date: 
Agenda: 
"""

# Task assignment template
task = """
---
title: "Task: {{subject}}"
tags: [task, email, {{priority}}]
assigned_to: "{{assigned_to}}"
due_date: "{{due_date}}"
status: "pending"
---

# Task: {{subject}}

**Assigned by:** {{sender_name}}
**Assigned to:** {{assigned_to}}
**Due Date:** {{due_date}}
**Priority:** {{priority}}

## Description

{{content}}

## Requirements

## Acceptance Criteria

- [ ] 

## Progress

## Notes
"""
```

### Contact Management

```toml
[plugins.notes.email.contacts]
# Automatic contact note creation
create_contact_notes = true
update_contact_notes = true

# Contact note template
contact_template = """
---
title: "Contact: {{name}}"
tags: [contact, {{company_slug}}]
email: "{{email}}"
company: "{{company}}"
last_contact: "{{last_email_date}}"
---

# {{name}}

**Email:** {{email}}
**Company:** {{company}}
**Title:** {{title}}
**Phone:** {{phone}}

## Background

## Recent Conversations

{{recent_emails}}

## Notes

## Next Steps

- [ ] 
"""
```

## Mobile Integration

### KDE Connect Setup

```toml
[plugins.notes.mobile]
# Enable mobile integration
enabled = true
auto_discover_devices = true

# SMS to notes conversion
sms_to_notes = true
sms_filters = [
    { from = "+1234567890", action = "create_note" },
    { contains = ["TODO", "REMINDER"], action = "create_note" },
    { from_contacts = true, min_length = 50, action = "create_note" }
]

# Note synchronization
sync_notes_to_mobile = true
sync_frequency = "15m"  # 15 minutes
sync_conflicts = "prefer_mobile"  # prefer_mobile, prefer_desktop, manual

# Notification settings
notify_new_notes = true
notify_sync_status = false
```

### SMS Note Templates

```toml
[plugins.notes.mobile.templates]
# SMS note template
sms = """
---
title: "SMS: {{preview}}"
tags: [sms, mobile, {{contact_name}}]
from: "{{phone_number}}"
contact: "{{contact_name}}"
date: "{{timestamp}}"
---

# SMS from {{contact_name}}

**From:** {{contact_name}} ({{phone_number}})
**Date:** {{timestamp}}

## Message

{{content}}

## Context

## Action Items

- [ ] 

## Response

"""

# Voice note template (if supported)
voice = """
---
title: "Voice Note: {{timestamp}}"
tags: [voice, mobile]
duration: "{{duration}}"
transcription_confidence: "{{confidence}}"
---

# Voice Note

**Date:** {{timestamp}}
**Duration:** {{duration}}
**Quality:** {{quality}}

## Transcription

{{transcription}}

## Summary

{{ai_summary}}

## Action Items

- [ ] 
"""
```

## Calendar Integration

### Calendar Settings

```toml
[plugins.notes.calendar]
# Enable calendar integration
enabled = true
auto_create_meeting_notes = true
auto_create_threshold = "invited"  # all, invited, organizer, none

# Meeting note timing
create_timing = "before"  # before, during, after
create_offset_minutes = 15

# Note organization
organize_by_project = true
organize_by_calendar = false
organize_by_date = true

# Calendar sources
calendars = [
    { name = "work", url = "https://calendar.example.com/work.ics" },
    { name = "personal", url = "https://calendar.example.com/personal.ics" }
]
```

### Meeting Note Templates

```toml
[plugins.notes.calendar.templates]
# Standard meeting template
meeting = """
---
title: "{{title}}"
tags: [meeting, {{project_tags}}]
date: "{{date}}"
time: "{{start_time}} - {{end_time}}"
attendees: {{attendees}}
calendar_event: "{{event_id}}"
meeting_type: "{{meeting_type}}"
---

# {{title}}

**Date:** {{formatted_date}}
**Time:** {{start_time}} - {{end_time}} ({{duration}})
**Location:** {{location}}
**Meeting Type:** {{meeting_type}}

## Attendees

{{#each attendees}}
- {{name}} <{{email}}> {{#if organizer}}(Organizer){{/if}}
{{/each}}

## Agenda

{{agenda}}

## Pre-meeting Notes

## Discussion

### {{topic_1}}

### {{topic_2}}

## Decisions

## Action Items

{{#each action_items}}
- [ ] {{description}} (@{{assignee}} due: {{due_date}})
{{/each}}

## Next Steps

## Follow-up Meeting

**Date:** 
**Agenda:** 
"""

# Standup meeting template
standup = """
---
title: "Standup: {{date}}"
tags: [standup, meeting, {{team_tag}}]
date: "{{date}}"
team: "{{team_name}}"
---

# Daily Standup - {{formatted_date}}

**Team:** {{team_name}}
**Date:** {{formatted_date}}

## Attendees

{{attendees_list}}

## Yesterday's Accomplishments

{{#each attendees}}
### {{name}}
- 

{{/each}}

## Today's Goals

{{#each attendees}}
### {{name}}
- 

{{/each}}

## Blockers & Issues

{{#each attendees}}
### {{name}}
- 

{{/each}}

## Team Updates

## Action Items

- [ ] 
"""

# Retrospective template
retrospective = """
---
title: "Retrospective: {{sprint_name}}"
tags: [retrospective, meeting, {{team_tag}}]
sprint: "{{sprint_name}}"
date: "{{date}}"
---

# Sprint Retrospective - {{sprint_name}}

**Date:** {{formatted_date}}
**Sprint:** {{sprint_name}}
**Duration:** {{sprint_duration}}

## Attendees

{{attendees_list}}

## Sprint Summary

**Goals:** {{sprint_goals}}
**Completed:** {{completed_stories}}
**Velocity:** {{velocity}}

## What Went Well? 😊

- 

## What Could Be Improved? 🤔

- 

## What Should We Stop Doing? ❌

- 

## What Should We Start Doing? ✅

- 

## Action Items

- [ ] 

## Next Sprint Focus

"""
```

## TUI Customization

### Theme Configuration

```toml
[plugins.notes.tui.themes.dark]
# Dark theme colors
background = "#1e1e1e"
foreground = "#d4d4d4"
border = "#404040"
selected = "#264f78"
highlight = "#569cd6"
error = "#f14c4c"
warning = "#ffcc02"
success = "#73c991"

# Syntax highlighting
markdown_header = "#569cd6"
markdown_bold = "#d7ba7d"
markdown_italic = "#ce9178"
markdown_code = "#d4d4d4"
markdown_link = "#4ec9b0"

[plugins.notes.tui.themes.light]
# Light theme colors
background = "#ffffff"
foreground = "#000000"
border = "#cccccc"
selected = "#e3f2fd"
highlight = "#2196f3"
error = "#f44336"
warning = "#ff9800"
success = "#4caf50"
```

### Key Bindings

```toml
[plugins.notes.tui.keybindings]
# Global shortcuts
quit = ["q", "Ctrl+c"]
help = ["?", "F1"]
refresh = ["F5", "Ctrl+r"]

# Browse mode
[plugins.notes.tui.keybindings.browse]
up = ["k", "Up"]
down = ["j", "Down"]
page_up = ["Ctrl+u", "PageUp"]
page_down = ["Ctrl+d", "PageDown"]
top = ["g"]
bottom = ["G"]
select = ["Enter", "l", "Right"]
back = ["h", "Left", "Esc"]
new_note = ["n"]
edit_note = ["e"]
delete_note = ["d"]
rename_note = ["r"]
copy_note = ["y"]
paste_note = ["p"]
search = ["/"]
filter_tags = ["t"]
sort = ["s"]

# Edit mode
[plugins.notes.tui.keybindings.edit]
save = ["Ctrl+s"]
save_and_quit = ["Ctrl+x"]
quit_without_save = ["Ctrl+q"]
undo = ["Ctrl+z"]
redo = ["Ctrl+y"]
find = ["Ctrl+f"]
replace = ["Ctrl+h"]
goto_line = ["Ctrl+g"]

# Search mode
[plugins.notes.tui.keybindings.search]
search = ["Enter"]
clear = ["Ctrl+u"]
next_result = ["Ctrl+n"]
prev_result = ["Ctrl+p"]
filter_toggle = ["Tab"]
```

### Layout Configuration

```toml
[plugins.notes.tui.layout]
# Main layout
show_sidebar = true
sidebar_width = 25
show_status_bar = true
show_search_bar = true

# Note list
show_preview = true
preview_lines = 3
show_tags = true
show_dates = true
show_word_count = false

# Editor
show_line_numbers = true
word_wrap = true
tab_size = 4
auto_indent = true
syntax_highlighting = true

# Search
highlight_matches = true
show_snippets = true
snippet_length = 100
max_results_per_page = 20
```

## Workflow Configurations

### Academic Research Workflow

```toml
[plugins.notes.workflows.academic]
# Directory structure
base_directory = "~/Research/Notes"
subdirectories = [
    "Papers",
    "Ideas", 
    "Literature_Reviews",
    "Methodology",
    "Data",
    "Drafts"
]

# Templates
[plugins.notes.workflows.academic.templates]
paper_review = """
---
title: "Review: {{paper_title}}"
tags: [paper, review, {{field}}, {{year}}]
authors: {{authors}}
journal: "{{journal}}"
year: {{year}}
doi: "{{doi}}"
status: "reading"
---

# {{paper_title}}

**Authors:** {{authors}}
**Journal:** {{journal}} ({{year}})
**DOI:** {{doi}}

## Summary

## Key Contributions

1. 

## Methodology

## Results

## Strengths

- 

## Weaknesses

- 

## Related Work

- [[]]

## Questions/Future Work

- 

## Citations

{{citations}}
"""

research_idea = """
---
title: "Idea: {{title}}"
tags: [idea, {{field}}, {{priority}}]
status: "draft"
confidence: "{{confidence}}"
---

# {{title}}

## Problem Statement

## Hypothesis

## Methodology

## Expected Outcomes

## Related Work

- [[]]

## Resources Needed

- 

## Next Steps

- [ ] 
"""
```

### Project Management Workflow

```toml
[plugins.notes.workflows.project]
# Project structure
base_directory = "~/Projects"
template_directory = "~/Projects/Templates"

# Auto-create project structure
auto_create_structure = true
project_subdirectories = [
    "planning",
    "meetings", 
    "specs",
    "notes",
    "reviews"
]

[plugins.notes.workflows.project.templates]
project_overview = """
---
title: "Project: {{project_name}}"
tags: [project, {{status}}, {{priority}}]
status: "{{status}}"
start_date: "{{start_date}}"
end_date: "{{end_date}}"
team: {{team_members}}
---

# {{project_name}}

**Status:** {{status}}
**Timeline:** {{start_date}} - {{end_date}}
**Team:** {{team_list}}

## Objective

## Success Criteria

- [ ] 

## Milestones

- [ ] {{milestone_1}} ({{date_1}})
- [ ] {{milestone_2}} ({{date_2}})

## Resources

## Risks

## Related Projects

- [[]]

## Documentation

- [[Project Plan|{{project_name}}/planning/project-plan]]
- [[Technical Spec|{{project_name}}/specs/technical-spec]]
- [[Meeting Notes|{{project_name}}/meetings/]]
"""
```

This configuration guide provides comprehensive examples for customizing the Notes Plugin to fit various workflows and preferences. Adjust these configurations based on your specific needs and organizational requirements.