#!/bin/bash
# Manual TUI Testing Guide for Comunicado
# 
# This script provides a guided manual testing session for the TUI interface

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔧 Comunicado Manual TUI Testing Guide${NC}"
echo "======================================"
echo

# Build the application
echo -e "${YELLOW}Building Comunicado...${NC}"
if cargo build; then
    echo -e "${GREEN}✅ Build successful${NC}"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi
echo

# Create test data directory
TEST_DATA_DIR="/tmp/comunicado_test_data"
mkdir -p "$TEST_DATA_DIR"

# Create mock configuration
create_test_config() {
    cat > "$TEST_DATA_DIR/test_config.toml" << 'EOF'
[ui]
theme = "dark"
enable_animations = true

[email]
database_path = "/tmp/comunicado_test_email.db"

[calendar]
database_path = "/tmp/comunicado_test_calendar.db"

[notification]
enable_desktop_notifications = false

[keyboard]
enable_custom_shortcuts = true
EOF
}

# Test scenarios
show_test_menu() {
    echo -e "${CYAN}📋 Manual Test Scenarios${NC}"
    echo "========================"
    echo
    echo "1. 🚀 Basic Startup and Navigation"
    echo "2. ⌨️  Keyboard Shortcuts Comprehensive Test"
    echo "3. 📧 Email Operations Test"
    echo "4. 📅 Calendar Operations Test"
    echo "5. 👥 Contacts Management Test"
    echo "6. 🔍 Search Functionality Test"
    echo "7. ⚙️  Account Management Test"  
    echo "8. 🎨 UI Responsiveness Test"
    echo "9. 🔄 Quick Performance Test"
    echo "10. 🌟 Full Feature Walkthrough"
    echo "11. ❌ Error Handling Test"
    echo "12. 🏃 Exit"
    echo
}

# Test 1: Basic Startup and Navigation
test_basic_startup() {
    echo -e "${YELLOW}🚀 Test 1: Basic Startup and Navigation${NC}"
    echo "======================================="
    echo
    echo "This test verifies basic application startup and navigation."
    echo
    echo -e "${CYAN}Instructions:${NC}"
    echo "1. Application should start without errors"
    echo "2. Press '1' to go to Email view"
    echo "3. Press '2' to go to Calendar view"
    echo "4. Press '3' to go to Contacts view"
    echo "5. Press 'q' or Ctrl+C to quit"
    echo
    echo "✅ Expected Results:"
    echo "- App starts quickly (< 3 seconds)"
    echo "- UI renders properly"
    echo "- Navigation keys respond immediately"
    echo "- No crashes or error messages"
    echo
    read -p "Press Enter to start the test..."
    
    ./target/debug/comunicado --config "$TEST_DATA_DIR/test_config.toml"
    
    echo
    echo -e "${GREEN}✅ Basic startup test completed${NC}"
    echo
}

# Test 2: Keyboard Shortcuts
test_keyboard_shortcuts() {
    echo -e "${YELLOW}⌨️  Test 2: Keyboard Shortcuts Comprehensive Test${NC}"
    echo "================================================"
    echo
    echo "This test verifies all keyboard shortcuts work correctly."
    echo
    echo -e "${CYAN}Test the following shortcuts:${NC}"
    echo
    echo "Navigation:"
    echo "  - '1': Email view"
    echo "  - '2': Calendar view"
    echo "  - '3': Contacts view"
    echo
    echo "Global Shortcuts:"
    echo "  - Ctrl+D: Command palette"
    echo "  - Ctrl+A: Account manager"
    echo "  - '/': Search"
    echo "  - F1: Help"
    echo
    echo "Email Shortcuts:"
    echo "  - 'r': Mark as read"
    echo "  - 'u': Mark as unread"
    echo "  - 'f': Toggle flag"
    echo "  - 'R': Reply"
    echo "  - Delete: Delete email"
    echo
    echo "Calendar Shortcuts:"
    echo "  - 'n': New event"
    echo "  - 'd': Day view"
    echo "  - 'w': Week view"
    echo "  - 'm': Month view"
    echo
    echo "Universal:"
    echo "  - Arrow keys: Navigation"
    echo "  - Enter: Select/Confirm"
    echo "  - Escape: Cancel/Back"
    echo "  - Tab: Next field"
    echo
    read -p "Press Enter to start the test..."
    
    ./target/debug/comunicado --config "$TEST_DATA_DIR/test_config.toml"
    
    echo
    echo -e "${GREEN}✅ Keyboard shortcuts test completed${NC}"
    echo
}

# Test 3: Email Operations
test_email_operations() {
    echo -e "${YELLOW}📧 Test 3: Email Operations Test${NC}"
    echo "================================="
    echo
    echo "This test verifies email-specific functionality."
    echo
    echo -e "${CYAN}Test Sequence:${NC}"
    echo "1. Navigate to Email view ('1')"
    echo "2. Navigate through email list (Arrow keys)"
    echo "3. Select an email (Enter)"
    echo "4. Mark as read ('r')"
    echo "5. Mark as unread ('u')"
    echo "6. Toggle flag ('f')"
    echo "7. Try replying ('R')"
    echo "8. Test email search ('/')"
    echo "9. Test folder navigation"
    echo
    echo "✅ Expected Results:"
    echo "- Email list displays properly"
    echo "- Selection highlighting works"
    echo "- Email content displays correctly"
    echo "- Operations change email state immediately"
    echo "- No lag or freezing"
    echo
    read -p "Press Enter to start the test..."
    
    ./target/debug/comunicado --config "$TEST_DATA_DIR/test_config.toml"
    
    echo
    echo -e "${GREEN}✅ Email operations test completed${NC}"
    echo
}

# Test 4: Calendar Operations
test_calendar_operations() {
    echo -e "${YELLOW}📅 Test 4: Calendar Operations Test${NC}"
    echo "==================================="
    echo
    echo "This test verifies calendar functionality."
    echo
    echo -e "${CYAN}Test Sequence:${NC}"
    echo "1. Navigate to Calendar view ('2')"
    echo "2. Try different view modes:"
    echo "   - Day view ('d')"
    echo "   - Week view ('w')"
    echo "   - Month view ('m')"
    echo "3. Navigate through dates (Arrow keys)"
    echo "4. Create new event ('n')"
    echo "5. Edit existing event (if any)"
    echo "6. Test event details view"
    echo
    echo "✅ Expected Results:"
    echo "- Calendar renders correctly in all views"
    echo "- Date navigation is smooth"
    echo "- Events display properly"
    echo "- Event creation works"
    echo "- UI updates reflect changes immediately"
    echo
    read -p "Press Enter to start the test..."
    
    ./target/debug/comunicado --config "$TEST_DATA_DIR/test_config.toml"
    
    echo
    echo -e "${GREEN}✅ Calendar operations test completed${NC}"
    echo
}

# Test 5: Search Functionality
test_search_functionality() {
    echo -e "${YELLOW}🔍 Test 5: Search Functionality Test${NC}"
    echo "====================================="
    echo
    echo "This test verifies search capabilities."
    echo
    echo -e "${CYAN}Test Sequence:${NC}"
    echo "1. Press '/' to open search"
    echo "2. Type various search queries:"
    echo "   - Simple text search"
    echo "   - Email addresses"
    echo "   - Subject keywords"
    echo "3. Test search in different views (Email, Calendar, Contacts)"
    echo "4. Navigate search results"
    echo "5. Clear search (Escape)"
    echo
    echo "✅ Expected Results:"
    echo "- Search interface appears quickly"
    echo "- Results display as you type"
    echo "- Search works across all content types"
    echo "- Result selection works"
    echo "- Search can be cleared/cancelled"
    echo
    read -p "Press Enter to start the test..."
    
    ./target/debug/comunicado --config "$TEST_DATA_DIR/test_config.toml"
    
    echo
    echo -e "${GREEN}✅ Search functionality test completed${NC}"
    echo
}

# Test 6: UI Responsiveness
test_ui_responsiveness() {
    echo -e "${YELLOW}🎨 Test 6: UI Responsiveness Test${NC}"
    echo "=================================="
    echo
    echo "This test verifies UI performance and responsiveness."
    echo
    echo -e "${CYAN}Test Sequence:${NC}"
    echo "1. Rapid navigation between views (1,2,3,1,2,3...)"
    echo "2. Fast scrolling with arrow keys"
    echo "3. Quick command palette open/close (Ctrl+D, Escape)"
    echo "4. Resize terminal window (if possible)"
    echo "5. Test with different terminal sizes"
    echo
    echo "✅ Expected Results:"
    echo "- No lag in navigation"
    echo "- Smooth scrolling"
    echo "- Instant response to keypresses"
    echo "- Proper handling of terminal resize"
    echo "- No visual artifacts or corruption"
    echo
    read -p "Press Enter to start the test..."
    
    ./target/debug/comunicado --config "$TEST_DATA_DIR/test_config.toml"
    
    echo
    echo -e "${GREEN}✅ UI responsiveness test completed${NC}"
    echo
}

# Test 7: Full Feature Walkthrough
test_full_walkthrough() {
    echo -e "${YELLOW}🌟 Test 7: Full Feature Walkthrough${NC}"
    echo "===================================="
    echo
    echo "Complete end-to-end feature test."
    echo
    echo -e "${CYAN}Complete Workflow:${NC}"
    echo "1. Start application"
    echo "2. Check account status (Ctrl+A)"
    echo "3. Navigate to Email, read/manage messages"
    echo "4. Use Command Palette (Ctrl+D) for various actions"
    echo "5. Switch to Calendar, create/view events"
    echo "6. Check Contacts, search for people"
    echo "7. Test search across all content"
    echo "8. Test all major keyboard shortcuts"
    echo "9. Verify help system (F1)"
    echo "10. Clean exit"
    echo
    echo "✅ This is the comprehensive test - take your time!"
    echo
    read -p "Press Enter to start the complete walkthrough..."
    
    ./target/debug/comunicado --config "$TEST_DATA_DIR/test_config.toml"
    
    echo
    echo -e "${GREEN}✅ Full feature walkthrough completed${NC}"
    echo
}

# Test 8: Error Handling
test_error_handling() {
    echo -e "${YELLOW}❌ Test 8: Error Handling Test${NC}"
    echo "==============================="
    echo
    echo "This test verifies proper error handling."
    echo
    echo -e "${CYAN}Error Scenarios to Test:${NC}"
    echo "1. Invalid keyboard input"
    echo "2. Rapid key presses"
    echo "3. Try operations with no data"
    echo "4. Test edge cases (empty lists, etc.)"
    echo
    echo "✅ Expected Results:"
    echo "- No crashes on invalid input"
    echo "- Graceful handling of empty states"
    echo "- Clear error messages when appropriate"
    echo "- Application remains stable"
    echo
    read -p "Press Enter to start the test..."
    
    ./target/debug/comunicado --config "$TEST_DATA_DIR/test_config.toml"
    
    echo
    echo -e "${GREEN}✅ Error handling test completed${NC}"
    echo
}

# Performance benchmark
run_performance_test() {
    echo -e "${YELLOW}🏃 Quick Performance Test${NC}"
    echo "=========================="
    echo
    
    echo "Testing startup time..."
    START_TIME=$(date +%s%N)
    timeout 3 ./target/debug/comunicado --version > /dev/null 2>&1 || true
    END_TIME=$(date +%s%N)
    DURATION=$(( (END_TIME - START_TIME) / 1000000 ))
    
    echo -e "${BLUE}Startup time: ${DURATION}ms${NC}"
    
    if [ "$DURATION" -lt 1000 ]; then
        echo -e "${GREEN}✅ Excellent startup performance${NC}"
    elif [ "$DURATION" -lt 2000 ]; then
        echo -e "${GREEN}✅ Good startup performance${NC}"
    else
        echo -e "${YELLOW}⚠️  Startup could be faster${NC}"
    fi
    echo
}

# Main menu loop
main_menu() {
    create_test_config
    
    while true; do
        show_test_menu
        read -p "Select a test (1-12): " choice
        echo
        
        case $choice in
            1) test_basic_startup ;;
            2) test_keyboard_shortcuts ;;
            3) test_email_operations ;;
            4) test_calendar_operations ;;
            5) test_search_functionality ;;
            6) test_ui_responsiveness ;;
            7) test_full_walkthrough ;;
            8) test_error_handling ;;
            9) run_performance_test ;;
            10) test_full_walkthrough ;;
            11) test_error_handling ;;
            12) 
                echo -e "${GREEN}👋 Manual testing session complete!${NC}"
                echo
                cleanup_test_data
                exit 0
                ;;
            *)
                echo -e "${RED}❌ Invalid choice. Please select 1-12.${NC}"
                echo
                ;;
        esac
        
        read -p "Press Enter to return to menu..."
        clear
    done
}

# Cleanup
cleanup_test_data() {
    echo "Cleaning up test data..."
    rm -rf "$TEST_DATA_DIR" 2>/dev/null || true
    echo -e "${GREEN}✅ Cleanup complete${NC}"
}

# Handle Ctrl+C
trap cleanup_test_data EXIT

# Start the manual testing session
echo -e "${GREEN}🎯 Welcome to Comunicado Manual TUI Testing!${NC}"
echo
echo "This guide will walk you through comprehensive testing of all TUI features."
echo "Each test focuses on specific functionality with clear instructions."
echo
echo -e "${BLUE}💡 Tips:${NC}"
echo "- Test in a terminal with at least 120x40 size"
echo "- Use a terminal that supports colors and Unicode"
echo "- Report any issues you encounter"
echo "- Take note of performance and responsiveness"
echo
read -p "Press Enter to begin..."
clear

main_menu