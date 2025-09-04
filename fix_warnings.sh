#!/bin/bash

# Script to fix common unused import warnings

echo "Fixing unused import warnings..."

# Fix uuid::Uuid imports
echo "Fixing uuid::Uuid imports..."
files=(
    "src/performance/database.rs"
    "src/performance/cache.rs"
    "src/multiplexer/tmux.rs"
    "src/multiplexer/mod.rs"
)

for file in "${files[@]}"; do
    if [ -f "$file" ]; then
        sed -i 's/^use uuid::Uuid;$/\/\/ use uuid::Uuid;/' "$file"
        echo "Fixed $file"
    fi
done

# Fix MultiplexerError imports
echo "Fixing MultiplexerError imports..."
multiplexer_files=(
    "src/multiplexer/status_integration.rs"
    "src/multiplexer/remote_session.rs"
    "src/multiplexer/clipboard_sync.rs"
)

for file in "${multiplexer_files[@]}"; do
    if [ -f "$file" ]; then
        sed -i 's/use super::{MultiplexerError, MultiplexerResult};/use super::{MultiplexerResult}; \/\/ MultiplexerError/' "$file"
        echo "Fixed $file"
    fi
done

# Fix HashMap imports
echo "Fixing HashMap imports..."
sed -i 's/^use std::collections::HashMap;$/\/\/ use std::collections::HashMap;/' "src/multiplexer/remote_session.rs"

# Fix PerformanceError import
echo "Fixing PerformanceError import..."
if [ -f "src/performance/cache.rs" ]; then
    sed -i 's/use crate::performance::{PerformanceResult, PerformanceError};/use crate::performance::{PerformanceResult}; \/\/ PerformanceError/' "src/performance/cache.rs"
fi

# Fix UI module warnings
echo "Fixing UI module warnings..."
if [ -f "src/ui/status_bar.rs" ]; then
    sed -i 's/, InformationDensity as TypographyInformationDensity/\/\/, InformationDensity as TypographyInformationDensity/' "src/ui/status_bar.rs"
fi

if [ -f "src/ui/mod.rs" ]; then
    sed -i 's/use crate::ui::unified_feedback::{FeedbackLevel, FeedbackContext};/use crate::ui::unified_feedback::{FeedbackContext}; \/\/ FeedbackLevel/' "src/ui/mod.rs"
fi

# Fix cloud sync warnings
echo "Fixing cloud sync warnings..."
if [ -f "src/cloud_sync/sync_engine.rs" ]; then
    sed -i 's/use super::{CloudSyncError, CloudSyncResult};/use super::{CloudSyncResult}; \/\/ CloudSyncError/' "src/cloud_sync/sync_engine.rs"
fi

# Fix deployment module warnings
echo "Fixing deployment module warnings..."
deployment_files=(
    "src/deployment/packaging.rs"
    "src/deployment/distributions.rs"
)

for file in "${deployment_files[@]}"; do
    if [ -f "$file" ]; then
        sed -i 's/use std::path::{Path, PathBuf};/use std::path::{PathBuf}; \/\/ Path/' "$file"
        sed -i 's/use crate::deployment::{DeploymentTarget, Architecture};/use crate::deployment::{DeploymentTarget}; \/\/ Architecture/' "$file"
        echo "Fixed $file"
    fi
done

# Fix config module warnings
echo "Fixing config module warnings..."
if [ -f "src/config/advanced_ui.rs" ]; then
    sed -i 's/use super::{AdvancedConfigResult, ValidationSeverity, ConfigurationError};/use super::{ConfigurationError}; \/\/ AdvancedConfigResult, ValidationSeverity/' "src/config/advanced_ui.rs"
    # Also fix the ratatui imports
    sed -i 's/use ratatui::{prelude::*, widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap}, layout::{Alignment, Constraint, Direction, Layout, Margin}, style::{Color, Modifier, Style}, text::{Line, Span}, widgets::{Cell, Row, Table}};/use ratatui::{prelude::*, widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap}, layout::{Constraint, Direction, Layout, Margin}};/' "src/config/advanced_ui.rs"
fi

# Fix iOS casing warning
echo "Fixing iOS casing warning..."
if [ -f "src/mobile/device_manager.rs" ]; then
    sed -i 's/iOS,/IOs,/' "src/mobile/device_manager.rs"
fi

echo "Warning fixes applied!"
echo "Running build to check remaining warnings..."