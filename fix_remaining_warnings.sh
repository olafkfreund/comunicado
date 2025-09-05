#!/bin/bash

# Fix specific unused variable warnings by adding underscore prefix

echo "🔧 Fixing remaining unused variable warnings..."

# Fix files_processed in backup_engine.rs
sed -i 's/let mut files_processed = 0;/let mut _files_processed = 0;/' src/backup/backup_engine.rs

# Fix data_type in cloud_sync/mod.rs method parameter
sed -i 's/async fn perform_sync(&mut self, data_type: SyncDataType)/async fn perform_sync(&mut self, _data_type: SyncDataType)/' src/cloud_sync/mod.rs

# Fix provider in real_time.rs
sed -i 's/pub async fn new(provider: &dyn CloudProvider)/pub async fn new(_provider: &dyn CloudProvider)/' src/cloud_sync/real_time.rs

# Fix success in database.rs
sed -i 's/success: bool,/_success: bool,/' src/performance/database.rs

# Fix calendar_id in sharing_ui.rs  
sed -i 's/if let Some(calendar_id) = self.state.selected_share {/if let Some(_calendar_id) = self.state.selected_share {/' src/calendar/sharing_ui.rs

# Fix remote in conflict_resolution.rs
sed -i 's/fn merge(&self, local: &\[u8\], remote: &\[u8\])/fn merge(\&self, local: \&[u8], _remote: \&[u8])/' src/cloud_sync/conflict_resolution.rs

# Fix mutable variable in screen.rs
sed -i 's/let mut chars: Vec<char>/let chars: Vec<char>/' src/multiplexer/screen.rs

# Fix unused parameters in screen.rs methods
sed -i 's/fn find_window_by_name(&self, output: &str, name: &str)/fn find_window_by_name(\&self, _output: \&str, _name: \&str)/' src/multiplexer/screen.rs

echo "✅ Applied targeted unused variable fixes"

# Count remaining warnings
echo "🔍 Checking remaining warnings..."
REMAINING=$(cargo check --release 2>&1 | grep "warning:" | wc -l)
echo "📊 Remaining warnings: $REMAINING"
