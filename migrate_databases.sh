#!/bin/bash

# Script to migrate databases to consolidated ~/.config/comunicado/ structure

CONFIG_DIR="$HOME/.config/comunicado"
OLD_DB_DIR="$CONFIG_DIR/databases"
OLD_DATA_DIR="$HOME/.local/share/comunicado"

echo "🔄 Migrating Comunicado databases to unified structure..."

# Create the config directory if it doesn't exist
mkdir -p "$CONFIG_DIR"

# Function to copy database if source exists and target doesn't
copy_if_needed() {
    local source="$1"  
    local target="$2"
    local name="$3"
    
    if [[ -f "$source" ]]; then
        if [[ -f "$target" ]]; then
            echo "⚠️  $name: Target already exists at $target"
            echo "   Consider backing up before overwriting"
        else
            echo "📦 $name: Copying from $source to $target"
            cp "$source" "$target"
        fi
    else
        echo "❌ $name: Source not found at $source"
    fi
}

# Migrate calendar database
if [[ -f "$OLD_DB_DIR/calendar.db" ]]; then
    copy_if_needed "$OLD_DB_DIR/calendar.db" "$CONFIG_DIR/calendar.db" "Calendar"
elif [[ -f "$CONFIG_DIR/calendar.db" ]]; then
    echo "✅ Calendar: Already in correct location"
else
    echo "⚠️  Calendar: No existing database found"
fi

# Migrate contacts database  
if [[ -f "$OLD_DB_DIR/contacts.db" ]]; then
    copy_if_needed "$OLD_DB_DIR/contacts.db" "$CONFIG_DIR/contacts.db" "Contacts"
elif [[ -f "$OLD_DATA_DIR/contacts.db" ]]; then
    copy_if_needed "$OLD_DATA_DIR/contacts.db" "$CONFIG_DIR/contacts.db" "Contacts"
elif [[ -f "$CONFIG_DIR/contacts.db" ]]; then
    echo "✅ Contacts: Already in correct location"
else
    echo "⚠️  Contacts: No existing database found"
fi

# Migrate email database
if [[ -f "$OLD_DB_DIR/email.db" ]]; then
    copy_if_needed "$OLD_DB_DIR/email.db" "$CONFIG_DIR/email.db" "Email"
elif [[ -f "$OLD_DATA_DIR/comunicado.db" ]]; then
    copy_if_needed "$OLD_DATA_DIR/comunicado.db" "$CONFIG_DIR/email.db" "Email"
elif [[ -f "$CONFIG_DIR/email.db" ]]; then
    echo "✅ Email: Already in correct location"
else
    echo "⚠️  Email: No existing database found"
fi

echo ""
echo "📊 Final database structure:"
ls -la "$CONFIG_DIR"/*.db 2>/dev/null || echo "   No databases found in $CONFIG_DIR"

echo ""
echo "✅ Migration complete!"
echo "   All databases should now be in: $CONFIG_DIR"
echo ""
echo "🗑️  After verifying everything works, you can clean up old locations:"
echo "   rm -rf '$OLD_DB_DIR'"
echo "   rm -rf '$OLD_DATA_DIR'"