#!/bin/bash

echo "Fixing remaining compilation warnings..."

# Fix backup module warnings
echo "Fixing backup module warnings..."
if [ -f "src/backup/backup_ui.rs" ]; then
    sed -i 's/use super::{BackupService, BackupTarget, BackupType, BackupResult};/use super::{BackupService, BackupResult}; \/\/ BackupTarget, BackupType/' "src/backup/backup_ui.rs"
    sed -i 's/use ratatui::{prelude::*, widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap}, layout::{Alignment, Constraint, Direction, Layout, Margin}, style::{Color, Modifier, Style}, text::{Line, Span}};/use ratatui::{prelude::*, widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap}, layout::{Constraint, Direction, Layout, Margin}};/' "src/backup/backup_ui.rs"
fi

if [ -f "src/backup/sync_engine.rs" ]; then
    sed -i 's/use std::path::{Path, PathBuf};/use std::path::{Path}; \/\/ PathBuf/' "src/backup/sync_engine.rs"
fi

if [ -f "src/backup/mod.rs" ]; then
    sed -i 's/use chrono::{DateTime, Utc};/use chrono::{Utc}; \/\/ DateTime/' "src/backup/mod.rs"
    sed -i 's/^use std::collections::HashMap;$/\/\/ use std::collections::HashMap;/' "src/backup/mod.rs"
    sed -i 's/^use uuid::Uuid;$/\/\/ use uuid::Uuid;/' "src/backup/mod.rs"
fi

# Fix calendar module warnings  
echo "Fixing calendar module warnings..."
if [ -f "src/calendar/sharing_ui.rs" ]; then
    sed -i 's/use super::{CalendarService, SharedUser, SharingResult};/use super::{CalendarService}; \/\/ SharedUser, SharingResult/' "src/calendar/sharing_ui.rs"
    sed -i 's/use ratatui::{prelude::*, widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Table, Wrap}, layout::{Alignment, Constraint, Direction, Layout, Margin}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Cell, Row}};/use ratatui::{prelude::*, widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap}, layout::{Constraint, Direction, Layout, Margin}};/' "src/calendar/sharing_ui.rs"
fi

# Fix cloud sync warnings
echo "Fixing cloud sync warnings..."
if [ -f "src/cloud_sync/providers.rs" ]; then
    sed -i 's/^use std::collections::HashMap;$/\/\/ use std::collections::HashMap;/' "src/cloud_sync/providers.rs"
fi

# Fix pwa module warnings
echo "Fixing PWA module warnings..."  
if [ -f "src/pwa/manifest.rs" ]; then
    sed -i 's/^use std::path::PathBuf;$/\/\/ use std::path::PathBuf;/' "src/pwa/manifest.rs"
fi

if [ -f "src/pwa/service_worker.rs" ]; then
    sed -i 's/^use std::path::Path;$/\/\/ use std::path::Path;/' "src/pwa/service_worker.rs"
fi

# Fix pwa/wasm warnings
if [ -f "src/pwa/wasm/email_preview.rs" ]; then
    sed -i 's/^use wasm_bindgen::prelude::*;$/\/\/ use wasm_bindgen::prelude::*;/' "src/pwa/wasm/email_preview.rs"
    sed -i 's/^use web_sys::{console, HtmlElement};$/\/\/ use web_sys::{console, HtmlElement};/' "src/pwa/wasm/email_preview.rs"
fi

if [ -f "src/pwa/wasm/notifications.rs" ]; then
    sed -i 's/^use wasm_bindgen::prelude::*;$/\/\/ use wasm_bindgen::prelude::*;/' "src/pwa/wasm/notifications.rs"
    sed -i 's/^use web_sys::{console, Notification, NotificationOptions};$/\/\/ use web_sys::{console, Notification, NotificationOptions};/' "src/pwa/wasm/notifications.rs"
fi

if [ -f "src/pwa/wasm/offline_storage.rs" ]; then
    sed -i 's/^use wasm_bindgen::prelude::*;$/\/\/ use wasm_bindgen::prelude::*;/' "src/pwa/wasm/offline_storage.rs"
    sed -i 's/^use web_sys::{console, Storage, Window};$/\/\/ use web_sys::{console, Storage, Window};/' "src/pwa/wasm/offline_storage.rs"
fi

echo "Additional warning fixes applied!"