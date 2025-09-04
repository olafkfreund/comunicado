#!/usr/bin/env python3

import re
import os

def fix_file_imports(filepath, fixes):
    """Apply import fixes to a file"""
    if not os.path.exists(filepath):
        return False
        
    with open(filepath, 'r') as f:
        content = f.read()
    
    modified = False
    for pattern, replacement in fixes:
        if re.search(pattern, content):
            content = re.sub(pattern, replacement, content)
            modified = True
    
    if modified:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"Fixed {filepath}")
        return True
    return False

# Define fixes for each file
fixes = {
    "src/backup/backup_ui.rs": [
        (r"use ratatui::\{[^}]+layout::\{Alignment,[^}]+\};", 
         "use ratatui::{\n    layout::{Constraint, Direction, Layout, Rect},\n    style::{Modifier, Style},\n    text::{Line, Span},\n    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},\n    Frame,\n};")
    ],
    
    "src/backup/sync_engine.rs": [
        (r"use std::path::\{Path, PathBuf\};", "use std::path::{Path}; // PathBuf")
    ],
    
    "src/backup/mod.rs": [
        (r"use chrono::\{DateTime, Utc\};", "use chrono::{DateTime}; // Utc"),
        (r"^use std::collections::HashMap;$", "// use std::collections::HashMap;"),
        (r"^use uuid::Uuid;$", "// use uuid::Uuid;")
    ],
    
    "src/calendar/sharing_ui.rs": [
        (r"BackupService, SharedUser, SharingResult", "BackupService, // SharedUser, SharingResult"),
        (r"use ratatui::\{[^}]+widgets::\{[^}]+Cell, Row[^}]+\};", 
         "use ratatui::{\n    layout::{Constraint, Direction, Layout, Rect},\n    style::{Modifier, Style},\n    text::{Line, Span},\n    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},\n    Frame,\n};")
    ],
    
    "src/cloud_sync/offline_storage.rs": [
        (r"use std::path::\{Path, PathBuf\};", "use std::path::{PathBuf}; // Path")
    ],
    
    "src/cloud_sync/mod.rs": [
        (r"^use std::path::PathBuf;$", "// use std::path::PathBuf;")
    ],
    
    "src/config/advanced_ui.rs": [
        (r"AdvancedConfigResult, ValidationSeverity, ConfigurationError", "ConfigurationError, // AdvancedConfigResult, ValidationSeverity"),
        (r"use ratatui::\{[^}]+Cell, Row, Table[^}]+\};", 
         "use ratatui::{\n    layout::{Constraint, Direction, Layout, Rect},\n    style::{Modifier, Style},\n    text::{Line, Span},\n    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},\n    Frame,\n};")
    ],
    
    "src/deployment/packaging.rs": [
        (r"DeploymentTarget, Architecture", "DeploymentTarget, // Architecture")
    ],
    
    "src/deployment/distributions.rs": [
        (r"DeploymentTarget, Architecture", "DeploymentTarget, // Architecture")
    ],
    
    "src/multiplexer/mod.rs": [
        (r"^use std::collections::HashMap;$", "// use std::collections::HashMap;")
    ]
}

print("Applying final warning fixes...")

total_fixed = 0
for filepath, file_fixes in fixes.items():
    if fix_file_imports(filepath, file_fixes):
        total_fixed += 1

print(f"Fixed {total_fixed} files")