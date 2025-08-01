#!/bin/bash

echo "🧪 Comunicado Runtime Debug Test"
echo "=================================="
echo

# Check if binary exists
if [ ! -f "./target/debug/comunicado" ]; then
    echo "❌ Binary not found. Building first..."
    cargo build
    if [ $? -ne 0 ]; then
        echo "❌ Build failed!"
        exit 1
    fi
fi

echo "📋 Test Plan:"
echo "1. Check account status (token expiration)"
echo "2. Test keyboard shortcuts (V, A, Enter, etc.)"
echo "3. Verify folder navigation"
echo

echo "🔍 Step 1: Check Account Status"
echo "------------------------------"
echo "ℹ️  We'll check if tokens are expired using the CLI"
echo

# Test account status
echo "📊 Account diagnostics:"
./target/debug/comunicado accounts 2>/dev/null || echo "ℹ️  Accounts command may not be available"

echo
echo "🔑 Token diagnostics:"
./target/debug/comunicado --version >/dev/null 2>&1 && echo "✅ Binary works" || echo "❌ Binary has issues"

echo
echo "🧪 Step 2: Keyboard Shortcut Runtime Test"
echo "----------------------------------------"
echo "ℹ️  Now we'll test the keyboard shortcuts in the actual app"
echo
echo "📝 Testing Instructions:"
echo "   1. Run: ./target/debug/comunicado"
echo "   2. Check if account indicator is red or green"
echo "   3. Try these shortcuts:"
echo "      • V (uppercase) - should open email viewer"
echo "      • A (uppercase) - should view attachment"  
echo "      • Enter on folder - should load folder"
echo "      • ? - should show help popup"
echo "      • q - should quit"
echo
echo "🔍 Debug logging will show:"
echo "   • 'Found keyboard action for' - confirms shortcut registered"
echo "   • 'OpenEmailViewer action triggered' - confirms V shortcut works"
echo "   • 'Folder selection: returning FolderSelect' - confirms Enter works"
echo

echo "💡 Expected Results:"
echo "   ✅ Account should be green (if tokens are valid)"
echo "   ✅ V should trigger debug message and open email viewer"
echo "   ✅ Enter should load folder contents"
echo "   ✅ ? should show updated help popup with proper categories"
echo

echo "🚀 Starting Comunicado with debug logging..."
echo "   (Press Ctrl+C to stop, look for debug messages)"
echo

# Set debug logging level
export RUST_LOG=debug,comunicado=debug

# Run the app
echo "⚡ Launching: RUST_LOG=debug ./target/debug/comunicado"
echo
./target/debug/comunicado