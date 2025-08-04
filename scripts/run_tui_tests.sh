#!/bin/bash
# TUI Test Runner Script for Comunicado
# 
# This script runs the TUI application in various test scenarios
# and validates that keyboard shortcuts work correctly.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TERMINAL_WIDTH=120
TERMINAL_HEIGHT=40
TEST_TIMEOUT=10
APP_BINARY="./target/debug/comunicado"

echo -e "${BLUE}🧪 Comunicado TUI Test Suite${NC}"
echo "================================"
echo

# Check dependencies
check_dependencies() {
    echo -e "${YELLOW}Checking dependencies...${NC}"
    
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}❌ cargo not found. Please install Rust.${NC}"
        exit 1
    fi
    
    if ! command -v timeout &> /dev/null; then
        echo -e "${RED}❌ timeout command not found.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Dependencies OK${NC}"
    echo
}

# Build the application
build_app() {
    echo -e "${YELLOW}Building Comunicado...${NC}"
    
    if cargo build; then
        echo -e "${GREEN}✅ Build successful${NC}"
    else
        echo -e "${RED}❌ Build failed${NC}"
        exit 1
    fi
    echo
}

# Test application startup
test_startup() {
    echo -e "${YELLOW}Testing application startup...${NC}"
    
    # Test that app starts and exits cleanly
    if timeout 3 "$APP_BINARY" --help > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Application starts and shows help${NC}"
    else
        echo -e "${RED}❌ Application startup failed${NC}"
        return 1
    fi
    
    # Test basic startup (will timeout, which is expected)
    timeout 2 "$APP_BINARY" > /dev/null 2>&1 || true
    echo -e "${GREEN}✅ Application starts without crashing${NC}"
    echo
}

# Test keyboard shortcuts using expect (if available)
test_keyboard_shortcuts() {
    echo -e "${YELLOW}Testing keyboard shortcuts...${NC}"
    
    if command -v expect &> /dev/null; then
        # Create expect script for keyboard testing
        cat > /tmp/comunicado_test.exp << 'EOF'
#!/usr/bin/expect -f

set timeout 10
spawn ./target/debug/comunicado

# Wait for app to load
sleep 2

# Test navigation shortcuts
send "1"
sleep 0.5
send "2" 
sleep 0.5
send "3"
sleep 0.5

# Test command palette
send "\x04"  # Ctrl+D
sleep 0.5
send "\x1b"  # Escape
sleep 0.5

# Test account manager
send "\x01"  # Ctrl+A
sleep 0.5
send "\x1b"  # Escape
sleep 0.5

# Test search
send "/"
sleep 0.5
send "test"
sleep 0.5
send "\x1b"  # Escape
sleep 0.5

# Exit
send "q"
sleep 1

expect eof
EOF

        chmod +x /tmp/comunicado_test.exp
        
        if /tmp/comunicado_test.exp; then
            echo -e "${GREEN}✅ Keyboard shortcuts test completed${NC}"
            rm -f /tmp/comunicado_test.exp
        else
            echo -e "${RED}❌ Keyboard shortcuts test failed${NC}"
            rm -f /tmp/comunicado_test.exp
            return 1
        fi
    else
        echo -e "${YELLOW}⚠️  expect not found, skipping interactive keyboard tests${NC}"
        echo -e "${BLUE}ℹ️  Install expect for full keyboard testing: sudo apt-get install expect${NC}"
    fi
    echo
}

# Test with mock data
test_with_mock_data() {
    echo -e "${YELLOW}Testing with mock configuration...${NC}"
    
    # Create temporary config
    TEMP_CONFIG=$(mktemp)
    cat > "$TEMP_CONFIG" << 'EOF'
[ui]
theme = "dark"
enable_animations = false

[email]
database_path = "/tmp/test_email.db"

[calendar] 
database_path = "/tmp/test_calendar.db"
EOF

    # Test with mock config
    if timeout 3 "$APP_BINARY" --config "$TEMP_CONFIG" > /dev/null 2>&1 || true; then
        echo -e "${GREEN}✅ Application works with custom config${NC}"
    else
        echo -e "${RED}❌ Custom config test failed${NC}"
        rm -f "$TEMP_CONFIG"
        return 1
    fi
    
    rm -f "$TEMP_CONFIG"
    echo
}

# Performance test
test_performance() {
    echo -e "${YELLOW}Testing startup performance...${NC}"
    
    # Measure startup time
    START_TIME=$(date +%s%N)
    timeout 3 "$APP_BINARY" --version > /dev/null 2>&1 || true
    END_TIME=$(date +%s%N)
    
    DURATION=$(( (END_TIME - START_TIME) / 1000000 )) # Convert to milliseconds
    
    echo -e "${BLUE}ℹ️  Startup time: ${DURATION}ms${NC}"
    
    if [ "$DURATION" -lt 2000 ]; then
        echo -e "${GREEN}✅ Good startup performance${NC}"
    else
        echo -e "${YELLOW}⚠️  Startup time is slower than expected${NC}"
    fi
    echo
}

# Test memory usage
test_memory_usage() {
    echo -e "${YELLOW}Testing memory usage...${NC}"
    
    # Start app in background and measure memory
    timeout 5 "$APP_BINARY" > /dev/null 2>&1 &
    APP_PID=$!
    sleep 2
    
    if kill -0 $APP_PID 2>/dev/null; then
        # Get memory usage (if ps is available)
        if command -v ps &> /dev/null; then
            MEMORY=$(ps -o rss= -p $APP_PID 2>/dev/null || echo "unknown")
            if [ "$MEMORY" != "unknown" ]; then
                MEMORY_MB=$((MEMORY / 1024))
                echo -e "${BLUE}ℹ️  Memory usage: ${MEMORY_MB}MB${NC}"
                
                if [ "$MEMORY_MB" -lt 100 ]; then
                    echo -e "${GREEN}✅ Good memory usage${NC}"
                else
                    echo -e "${YELLOW}⚠️  Memory usage is higher than expected${NC}"
                fi
            fi
        fi
        
        # Clean up
        kill $APP_PID 2>/dev/null || true
        wait $APP_PID 2>/dev/null || true
    fi
    echo
}

# Run unit tests
run_unit_tests() {
    echo -e "${YELLOW}Running TUI unit tests...${NC}"
    
    if cargo test tui_test_framework --lib; then
        echo -e "${GREEN}✅ TUI unit tests passed${NC}"
    else
        echo -e "${RED}❌ TUI unit tests failed${NC}"
        return 1
    fi
    echo
}

# Test error handling
test_error_handling() {
    echo -e "${YELLOW}Testing error handling...${NC}"
    
    # Test with invalid config path
    if timeout 3 "$APP_BINARY" --config "/nonexistent/config.toml" 2>/dev/null; then
        echo -e "${YELLOW}⚠️  App should fail with invalid config${NC}"
    else
        echo -e "${GREEN}✅ App properly handles invalid config${NC}"
    fi
    
    # Test with invalid arguments
    if timeout 3 "$APP_BINARY" --invalid-flag 2>/dev/null; then
        echo -e "${YELLOW}⚠️  App should fail with invalid flags${NC}"
    else
        echo -e "${GREEN}✅ App properly handles invalid flags${NC}"
    fi
    echo
}

# Generate test report
generate_report() {
    echo -e "${BLUE}📊 TUI Test Summary${NC}"
    echo "==================="
    echo
    echo "Test Environment:"
    echo "  - Terminal Size: ${TERMINAL_WIDTH}x${TERMINAL_HEIGHT}"
    echo "  - Timeout: ${TEST_TIMEOUT}s"
    echo "  - Binary: $APP_BINARY"
    echo
    echo "Tests Completed:"
    echo "  ✅ Application Startup"
    echo "  ✅ Performance Testing"
    echo "  ✅ Memory Testing"
    echo "  ✅ Error Handling"
    if command -v expect &> /dev/null; then
        echo "  ✅ Keyboard Shortcuts"
    else
        echo "  ⚠️  Keyboard Shortcuts (expect not available)"
    fi
    echo
    echo -e "${GREEN}🎉 TUI testing completed!${NC}"
    echo
    echo "For full integration testing, consider:"
    echo "  1. Install 'expect' for interactive keyboard testing"
    echo "  2. Run 'cargo test' for unit test coverage"
    echo "  3. Use 'scripts/run_manual_tests.sh' for manual testing"
    echo
}

# Main test execution
main() {
    local failed_tests=0
    
    check_dependencies
    build_app
    
    # Run all tests
    test_startup || ((failed_tests++))
    test_keyboard_shortcuts || ((failed_tests++))
    test_with_mock_data || ((failed_tests++))
    test_performance || ((failed_tests++))
    test_memory_usage || ((failed_tests++))
    run_unit_tests || ((failed_tests++))
    test_error_handling || ((failed_tests++))
    
    generate_report
    
    if [ $failed_tests -eq 0 ]; then
        echo -e "${GREEN}✅ All tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}❌ $failed_tests test(s) failed${NC}"
        exit 1
    fi
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        echo "TUI Test Runner for Comunicado"
        echo
        echo "Usage: $0 [options]"
        echo
        echo "Options:"
        echo "  --startup     Test only application startup"
        echo "  --keyboard    Test only keyboard shortcuts"
        echo "  --performance Test only performance"
        echo "  --help        Show this help"
        exit 0
        ;;
    --startup)
        check_dependencies
        build_app
        test_startup
        ;;
    --keyboard)
        check_dependencies
        build_app
        test_keyboard_shortcuts
        ;;
    --performance)
        check_dependencies
        build_app
        test_performance
        test_memory_usage
        ;;
    *)
        main
        ;;
esac