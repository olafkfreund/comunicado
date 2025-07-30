# Database Schema

This is the database schema implementation for the spec detailed in @.agent-os/specs/2025-07-25-modern-ui-enhancement/spec.md

> Created: 2025-07-25
> Version: 1.0.0

## Schema Changes

### New Tables

#### calendars
```sql
CREATE TABLE calendars (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    color TEXT NOT NULL DEFAULT '#3788d8',
    caldav_url TEXT,
    caldav_username TEXT,
    caldav_password_encrypted TEXT,
    sync_enabled BOOLEAN NOT NULL DEFAULT true,
    last_sync DATETIME,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_calendars_sync ON calendars(sync_enabled, last_sync);
```

#### calendar_events
```sql
CREATE TABLE calendar_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    calendar_id INTEGER NOT NULL,
    event_uid TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    start_datetime DATETIME NOT NULL,
    end_datetime DATETIME NOT NULL,
    all_day BOOLEAN NOT NULL DEFAULT false,
    recurrence_rule TEXT,
    recurrence_exceptions TEXT,
    status TEXT NOT NULL DEFAULT 'confirmed',
    organizer_email TEXT,
    attendees TEXT, -- JSON array of attendee objects
    last_modified DATETIME DEFAULT CURRENT_TIMESTAMP,
    etag TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE
);

CREATE INDEX idx_events_calendar ON calendar_events(calendar_id);
CREATE INDEX idx_events_datetime ON calendar_events(start_datetime, end_datetime);
CREATE INDEX idx_events_uid ON calendar_events(event_uid);
```

#### meeting_invitations
```sql
CREATE TABLE meeting_invitations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_message_id TEXT NOT NULL,
    event_uid TEXT NOT NULL,
    method TEXT NOT NULL, -- REQUEST, REPLY, CANCEL
    sequence_number INTEGER NOT NULL DEFAULT 0,
    response_status TEXT, -- ACCEPTED, DECLINED, TENTATIVE, NEEDS-ACTION
    processed BOOLEAN NOT NULL DEFAULT false,
    ical_data TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (email_message_id) REFERENCES emails(message_id) ON DELETE CASCADE
);

CREATE INDEX idx_invitations_email ON meeting_invitations(email_message_id);
CREATE INDEX idx_invitations_event ON meeting_invitations(event_uid);
```

#### plugins
```sql
CREATE TABLE plugins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    author TEXT,
    plugin_type TEXT NOT NULL, -- 'wasm', 'native'
    file_path TEXT NOT NULL,
    config_schema TEXT, -- JSON schema for plugin configuration
    enabled BOOLEAN NOT NULL DEFAULT true,
    auto_start BOOLEAN NOT NULL DEFAULT false,
    permissions TEXT, -- JSON array of required permissions
    dependencies TEXT, -- JSON array of plugin dependencies
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_plugins_enabled ON plugins(enabled);
CREATE INDEX idx_plugins_type ON plugins(plugin_type);
```

#### plugin_configs
```sql
CREATE TABLE plugin_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id INTEGER NOT NULL,
    config_key TEXT NOT NULL,
    config_value TEXT NOT NULL,
    encrypted BOOLEAN NOT NULL DEFAULT false,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    UNIQUE(plugin_id, config_key)
);

CREATE INDEX idx_plugin_configs_plugin ON plugin_configs(plugin_id);
```

#### plugin_data
```sql
CREATE TABLE plugin_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id INTEGER NOT NULL,
    data_key TEXT NOT NULL,
    data_value TEXT NOT NULL,
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    UNIQUE(plugin_id, data_key)
);

CREATE INDEX idx_plugin_data_plugin ON plugin_data(plugin_id);
CREATE INDEX idx_plugin_data_expires ON plugin_data(expires_at);
```

#### ai_conversations
```sql
CREATE TABLE ai_conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT UNIQUE NOT NULL,
    provider TEXT NOT NULL, -- 'openai', 'gemini', 'ollama'
    context_type TEXT NOT NULL, -- 'email_summary', 'response_draft', 'general'
    context_data TEXT, -- JSON data related to context
    conversation_history TEXT NOT NULL, -- JSON array of messages
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_ai_conversations_session ON ai_conversations(session_id);
CREATE INDEX idx_ai_conversations_type ON ai_conversations(context_type);
```

#### email_ai_analysis
```sql
CREATE TABLE email_ai_analysis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_message_id TEXT UNIQUE NOT NULL,
    summary TEXT,
    sentiment TEXT,
    priority_score INTEGER DEFAULT 0,
    contains_tasks BOOLEAN DEFAULT false,
    contains_meetings BOOLEAN DEFAULT false,
    spam_score REAL DEFAULT 0.0,
    ai_provider TEXT NOT NULL,
    analysis_version TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (email_message_id) REFERENCES emails(message_id) ON DELETE CASCADE
);

CREATE INDEX idx_email_analysis_message ON email_ai_analysis(email_message_id);
CREATE INDEX idx_email_analysis_priority ON email_ai_analysis(priority_score);
CREATE INDEX idx_email_analysis_spam ON email_ai_analysis(spam_score);
```

#### rss_feeds
```sql
CREATE TABLE rss_feeds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT UNIQUE NOT NULL,
    title TEXT,
    description TEXT,
    category TEXT,
    feed_type TEXT NOT NULL, -- 'rss', 'atom', 'youtube', 'podcast'
    last_fetched DATETIME,
    fetch_interval INTEGER DEFAULT 3600, -- seconds
    enabled BOOLEAN DEFAULT true,
    error_count INTEGER DEFAULT 0,
    last_error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_rss_feeds_enabled ON rss_feeds(enabled);
CREATE INDEX idx_rss_feeds_category ON rss_feeds(category);
CREATE INDEX idx_rss_feeds_fetch ON rss_feeds(last_fetched);
```

#### rss_items
```sql
CREATE TABLE rss_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_id INTEGER NOT NULL,
    item_guid TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    content TEXT,
    url TEXT,
    author TEXT,
    published_at DATETIME,
    ai_summary TEXT,
    ai_relevance_score REAL DEFAULT 0.0,
    read_status BOOLEAN DEFAULT false,
    bookmarked BOOLEAN DEFAULT false,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (feed_id) REFERENCES rss_feeds(id) ON DELETE CASCADE,
    UNIQUE(feed_id, item_guid)
);

CREATE INDEX idx_rss_items_feed ON rss_items(feed_id);
CREATE INDEX idx_rss_items_published ON rss_items(published_at);
CREATE INDEX idx_rss_items_read ON rss_items(read_status);
CREATE INDEX idx_rss_items_relevance ON rss_items(ai_relevance_score);
```

#### voice_commands
```sql
CREATE TABLE voice_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command_text TEXT NOT NULL,
    parsed_intent TEXT NOT NULL,
    executed_action TEXT,
    success BOOLEAN DEFAULT false,
    error_message TEXT,
    execution_time_ms INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_voice_commands_intent ON voice_commands(parsed_intent);
CREATE INDEX idx_voice_commands_success ON voice_commands(success);
```

#### account_profiles
```sql
CREATE TABLE account_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_name TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    email_address TEXT NOT NULL,
    profile_color TEXT NOT NULL, -- hex color for visual identity
    footer_template TEXT, -- custom footer template
    signature TEXT,
    is_default BOOLEAN DEFAULT false,
    ai_writing_style TEXT, -- JSON configuration for AI writing preferences
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_account_profiles_default ON account_profiles(is_default);
```

#### status_bar_config
```sql
CREATE TABLE status_bar_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    segment_name TEXT UNIQUE NOT NULL,
    display_order INTEGER NOT NULL,
    enabled BOOLEAN DEFAULT true,
    format_template TEXT NOT NULL,
    update_interval INTEGER DEFAULT 5, -- seconds
    color_scheme TEXT, -- JSON color configuration
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_status_config_order ON status_bar_config(display_order);
CREATE INDEX idx_status_config_enabled ON status_bar_config(enabled);
```

#### email_threads
```sql
CREATE TABLE email_threads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    root_message_id TEXT NOT NULL,
    subject_normalized TEXT NOT NULL,
    thread_hash TEXT UNIQUE NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_threads_hash ON email_threads(thread_hash);
CREATE INDEX idx_threads_subject ON email_threads(subject_normalized);
```

#### email_thread_messages
```sql
CREATE TABLE email_thread_messages (
    thread_id INTEGER NOT NULL,
    message_id TEXT NOT NULL,
    parent_message_id TEXT,
    level INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (thread_id, message_id),
    FOREIGN KEY (thread_id) REFERENCES email_threads(id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES emails(message_id) ON DELETE CASCADE
);

CREATE INDEX idx_thread_messages_thread ON email_thread_messages(thread_id);
CREATE INDEX idx_thread_messages_level ON email_thread_messages(thread_id, level);
CREATE INDEX idx_thread_messages_position ON email_thread_messages(thread_id, position);
```

### Modified Tables

#### emails (add threading columns)
```sql
ALTER TABLE emails ADD COLUMN thread_id INTEGER;
ALTER TABLE emails ADD COLUMN in_reply_to TEXT;
ALTER TABLE emails ADD COLUMN references TEXT;

CREATE INDEX idx_emails_thread_id ON emails(thread_id);
CREATE INDEX idx_emails_in_reply_to ON emails(in_reply_to);
```

#### user_preferences (new table for UI customization)
```sql
CREATE TABLE user_preferences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,
    value TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_preferences_category ON user_preferences(category);
CREATE INDEX idx_preferences_key ON user_preferences(key);
```

## Migration Scripts

### Migration 001: Add Threading Support
```sql
-- Add threading columns to emails table
ALTER TABLE emails ADD COLUMN thread_id INTEGER;
ALTER TABLE emails ADD COLUMN in_reply_to TEXT;
ALTER TABLE emails ADD COLUMN references TEXT;

-- Create indexes for performance
CREATE INDEX idx_emails_thread_id ON emails(thread_id);
CREATE INDEX idx_emails_in_reply_to ON emails(in_reply_to);

-- Create email_threads table
CREATE TABLE email_threads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    root_message_id TEXT NOT NULL,
    subject_normalized TEXT NOT NULL,
    thread_hash TEXT UNIQUE NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_threads_hash ON email_threads(thread_hash);
CREATE INDEX idx_threads_subject ON email_threads(subject_normalized);

-- Create email_thread_messages table
CREATE TABLE email_thread_messages (
    thread_id INTEGER NOT NULL,
    message_id TEXT NOT NULL,
    parent_message_id TEXT,
    level INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (thread_id, message_id),
    FOREIGN KEY (thread_id) REFERENCES email_threads(id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES emails(message_id) ON DELETE CASCADE
);

CREATE INDEX idx_thread_messages_thread ON email_thread_messages(thread_id);
CREATE INDEX idx_thread_messages_level ON email_thread_messages(thread_id, level);
CREATE INDEX idx_thread_messages_position ON email_thread_messages(thread_id, position);
```

### Migration 002: Add User Preferences
```sql
-- Create user preferences table for UI customization
CREATE TABLE user_preferences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,
    value TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_preferences_category ON user_preferences(category);
CREATE INDEX idx_preferences_key ON user_preferences(key);

-- Insert default UI preferences
INSERT INTO user_preferences (key, value, category) VALUES
    ('ui.show_sidebar', 'true', 'interface'),
    ('ui.show_preview', 'true', 'interface'),
    ('ui.show_status_bar', 'true', 'interface'),
    ('ui.show_calendar_panel', 'true', 'interface'),
    ('ui.email_list_columns', '["sender","subject","date"]', 'interface'),
    ('sorting.primary_key', 'date', 'email'),
    ('sorting.primary_order', 'desc', 'email'),
    ('threading.enabled', 'true', 'email'),
    ('threading.auto_expand', 'false', 'email'),
    ('calendar.default_view', 'week', 'calendar'),
    ('calendar.work_hours_start', '09:00', 'calendar'),
    ('calendar.work_hours_end', '17:00', 'calendar'),
    ('calendar.sync_interval', '300', 'calendar'),
    ('plugins.auto_load', 'true', 'plugins'),
    ('plugins.sandbox_enabled', 'true', 'plugins');
```

### Migration 003: Add Calendar System
```sql
-- Create calendars table
CREATE TABLE calendars (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    color TEXT NOT NULL DEFAULT '#3788d8',
    caldav_url TEXT,
    caldav_username TEXT,
    caldav_password_encrypted TEXT,
    sync_enabled BOOLEAN NOT NULL DEFAULT true,
    last_sync DATETIME,
    timezone TEXT NOT NULL DEFAULT 'UTC',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_calendars_sync ON calendars(sync_enabled, last_sync);

-- Create calendar_events table
CREATE TABLE calendar_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    calendar_id INTEGER NOT NULL,
    event_uid TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    start_datetime DATETIME NOT NULL,
    end_datetime DATETIME NOT NULL,
    all_day BOOLEAN NOT NULL DEFAULT false,
    recurrence_rule TEXT,
    recurrence_exceptions TEXT,
    status TEXT NOT NULL DEFAULT 'confirmed',
    organizer_email TEXT,
    attendees TEXT,
    last_modified DATETIME DEFAULT CURRENT_TIMESTAMP,
    etag TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE
);

CREATE INDEX idx_events_calendar ON calendar_events(calendar_id);
CREATE INDEX idx_events_datetime ON calendar_events(start_datetime, end_datetime);
CREATE INDEX idx_events_uid ON calendar_events(event_uid);

-- Create meeting_invitations table
CREATE TABLE meeting_invitations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_message_id TEXT NOT NULL,
    event_uid TEXT NOT NULL,
    method TEXT NOT NULL,
    sequence_number INTEGER NOT NULL DEFAULT 0,
    response_status TEXT,
    processed BOOLEAN NOT NULL DEFAULT false,
    ical_data TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (email_message_id) REFERENCES emails(message_id) ON DELETE CASCADE
);

CREATE INDEX idx_invitations_email ON meeting_invitations(email_message_id);
CREATE INDEX idx_invitations_event ON meeting_invitations(event_uid);

-- Insert default calendar
INSERT INTO calendars (name, description, color) VALUES 
    ('Personal', 'Default personal calendar', '#3788d8');
```

### Migration 004: Add Plugin System
```sql
-- Create plugins table
CREATE TABLE plugins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    author TEXT,
    plugin_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    config_schema TEXT,
    enabled BOOLEAN NOT NULL DEFAULT true,
    auto_start BOOLEAN NOT NULL DEFAULT false,
    permissions TEXT,
    dependencies TEXT,
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_updated DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_plugins_enabled ON plugins(enabled);
CREATE INDEX idx_plugins_type ON plugins(plugin_type);

-- Create plugin_configs table
CREATE TABLE plugin_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id INTEGER NOT NULL,
    config_key TEXT NOT NULL,
    config_value TEXT NOT NULL,
    encrypted BOOLEAN NOT NULL DEFAULT false,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    UNIQUE(plugin_id, config_key)
);

CREATE INDEX idx_plugin_configs_plugin ON plugin_configs(plugin_id);

-- Create plugin_data table
CREATE TABLE plugin_data (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id INTEGER NOT NULL,
    data_key TEXT NOT NULL,
    data_value TEXT NOT NULL,
    expires_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    UNIQUE(plugin_id, data_key)
);

CREATE INDEX idx_plugin_data_plugin ON plugin_data(plugin_id);
CREATE INDEX idx_plugin_data_expires ON plugin_data(expires_at);
```

### Migration 005: Add AI System
```sql
-- Create ai_conversations table
CREATE TABLE ai_conversations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT UNIQUE NOT NULL,
    provider TEXT NOT NULL,
    context_type TEXT NOT NULL,
    context_data TEXT,
    conversation_history TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_ai_conversations_session ON ai_conversations(session_id);
CREATE INDEX idx_ai_conversations_type ON ai_conversations(context_type);

-- Create email_ai_analysis table
CREATE TABLE email_ai_analysis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_message_id TEXT UNIQUE NOT NULL,
    summary TEXT,
    sentiment TEXT,
    priority_score INTEGER DEFAULT 0,
    contains_tasks BOOLEAN DEFAULT false,
    contains_meetings BOOLEAN DEFAULT false,
    spam_score REAL DEFAULT 0.0,
    ai_provider TEXT NOT NULL,
    analysis_version TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (email_message_id) REFERENCES emails(message_id) ON DELETE CASCADE
);

CREATE INDEX idx_email_analysis_message ON email_ai_analysis(email_message_id);
CREATE INDEX idx_email_analysis_priority ON email_ai_analysis(priority_score);
CREATE INDEX idx_email_analysis_spam ON email_ai_analysis(spam_score);

-- Create voice_commands table
CREATE TABLE voice_commands (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command_text TEXT NOT NULL,
    parsed_intent TEXT NOT NULL,
    executed_action TEXT,
    success BOOLEAN DEFAULT false,
    error_message TEXT,
    execution_time_ms INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_voice_commands_intent ON voice_commands(parsed_intent);
CREATE INDEX idx_voice_commands_success ON voice_commands(success);
```

### Migration 006: Add RSS Content System
```sql
-- Create rss_feeds table
CREATE TABLE rss_feeds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT UNIQUE NOT NULL,
    title TEXT,
    description TEXT,
    category TEXT,
    feed_type TEXT NOT NULL,
    last_fetched DATETIME,
    fetch_interval INTEGER DEFAULT 3600,
    enabled BOOLEAN DEFAULT true,
    error_count INTEGER DEFAULT 0,
    last_error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_rss_feeds_enabled ON rss_feeds(enabled);
CREATE INDEX idx_rss_feeds_category ON rss_feeds(category);
CREATE INDEX idx_rss_feeds_fetch ON rss_feeds(last_fetched);

-- Create rss_items table
CREATE TABLE rss_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_id INTEGER NOT NULL,
    item_guid TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    content TEXT,
    url TEXT,
    author TEXT,
    published_at DATETIME,
    ai_summary TEXT,
    ai_relevance_score REAL DEFAULT 0.0,
    read_status BOOLEAN DEFAULT false,
    bookmarked BOOLEAN DEFAULT false,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (feed_id) REFERENCES rss_feeds(id) ON DELETE CASCADE,
    UNIQUE(feed_id, item_guid)
);

CREATE INDEX idx_rss_items_feed ON rss_items(feed_id);
CREATE INDEX idx_rss_items_published ON rss_items(published_at);
CREATE INDEX idx_rss_items_read ON rss_items(read_status);
CREATE INDEX idx_rss_items_relevance ON rss_items(ai_relevance_score);
```

### Migration 007: Add Multi-Account and Status Bar System
```sql
-- Create account_profiles table
CREATE TABLE account_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_name TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    email_address TEXT NOT NULL,
    profile_color TEXT NOT NULL,
    footer_template TEXT,
    signature TEXT,
    is_default BOOLEAN DEFAULT false,
    ai_writing_style TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_account_profiles_default ON account_profiles(is_default);

-- Create status_bar_config table
CREATE TABLE status_bar_config (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    segment_name TEXT UNIQUE NOT NULL,
    display_order INTEGER NOT NULL,
    enabled BOOLEAN DEFAULT true,
    format_template TEXT NOT NULL,
    update_interval INTEGER DEFAULT 5,
    color_scheme TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_status_config_order ON status_bar_config(display_order);
CREATE INDEX idx_status_config_enabled ON status_bar_config(enabled);

-- Insert default status bar segments
INSERT INTO status_bar_config (segment_name, display_order, format_template, color_scheme) VALUES
    ('account', 1, '{{profile_name}}', '{"bg": "blue", "fg": "white"}'),
    ('email_count', 2, '✉ {{unread}}/{{total}}', '{"bg": "yellow", "fg": "black"}'),
    ('calendar_events', 3, '📅 {{today_events}}', '{"bg": "green", "fg": "white"}'),
    ('sync_status', 4, '{{sync_icon}}', '{"bg": "cyan", "fg": "black"}'),
    ('ai_status', 5, '🤖 {{ai_provider}}', '{"bg": "purple", "fg": "white"}'),
    ('time', 6, '{{time}}', '{"bg": "gray", "fg": "white"}');

-- Insert default account profile
INSERT INTO account_profiles (account_name, display_name, email_address, profile_color, is_default) VALUES
    ('default', 'Default Account', 'user@example.com', '#3788d8', true);
```

## Rationale

The expanded schema supports the transformation of Comunicado into a comprehensive communication and productivity hub:

**Threading System**: Separate tables efficiently store conversation relationships without denormalizing the main emails table, supporting fast hierarchical queries.

**Calendar System**: Full CalDAV compliance with proper event storage, timezone handling, and meeting invitation tracking. The calendar schema supports synchronization with external providers while maintaining local performance.

**Plugin System**: Secure plugin management with configuration isolation, data persistence, and permission tracking. The three-table plugin architecture (plugins, plugin_configs, plugin_data) provides flexibility for different plugin types while maintaining security boundaries.

**AI Integration**: Comprehensive AI support with conversation tracking, email analysis, and voice command logging. The ai_conversations table maintains context across sessions while email_ai_analysis provides cached intelligence to improve performance and enable offline AI features.

**RSS Content System**: Full RSS aggregation with AI-enhanced content processing. The two-table structure (rss_feeds, rss_items) supports various feed types including YouTube channels while maintaining performance through proper indexing and relevance scoring.

**Multi-Account Management**: Color-coded profile system with account_profiles table supporting custom branding, signatures, and AI writing styles. Each profile maintains its visual identity and configuration preferences.

**Status Bar System**: Configurable powerline-style status bar with status_bar_config table allowing users to customize information display, update intervals, and visual styling to match their workflow preferences.

**Voice Control**: Complete voice command tracking with voice_commands table logging all interactions for performance analysis and command improvement.

**Integration Layer**: The expanded user preferences system now supports all major components (email, calendar, plugins, AI, RSS) with categorized configuration management and cross-system integration.

The schema design supports:
- Fast thread lookup via indexed thread_hash
- Efficient hierarchical queries using level and position  
- Calendar event queries optimized for date ranges and calendar filtering
- Plugin lifecycle management with proper isolation
- AI conversation context persistence and email analysis caching
- RSS content aggregation with relevance scoring and categorization
- Multi-account profile management with visual identity systems
- Configurable status bar with real-time information display
- Voice command tracking and performance analysis
- Cross-system integration through foreign key relationships
- Flexible user preferences with categorization across all components
- Foreign key constraints maintaining data integrity throughout the expanded system