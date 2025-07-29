# Tests Specification

This is the tests coverage details for the spec detailed in @.agent-os/specs/2025-07-29-startup-progress-bar/spec.md

> Created: 2025-07-29
> Version: 1.0.0

## Test Coverage

### Unit Tests

**StartupProgressManager**
- Test phase initialization with correct timeout values and initial states
- Test phase transition logic from pending to in-progress to completed
- Test timeout detection and phase status updates when timeouts occur
- Test error state handling and error message propagation
- Test overall progress calculation based on completed phases
- Test ETA calculation accuracy with varying phase durations
- Test concurrent phase status updates without race conditions

**StartupPhase Enum and PhaseStatus**
- Test phase status transitions for all valid state changes
- Test invalid state transition rejection and error handling
- Test duration calculation accuracy for completed and timed-out phases
- Test error message storage and retrieval for failed phases

**Progress Calculation Logic**
- Test percentage calculation with different phase completion scenarios
- Test ETA accuracy with historical timing data and current progress
- Test progress calculation when phases fail or timeout
- Test boundary conditions (0% and 100% completion states)

### Integration Tests

**Startup Progress Integration**
- Test complete startup flow with progress tracking enabled
- Test progress updates during actual database initialization phase
- Test progress updates during IMAP manager initialization with real timeouts
- Test progress updates during account setup and service initialization
- Test graceful handling when services fail during startup
- Test UI rendering integration with progress manager state changes
- Test startup completion transition to main application UI

**Error Handling Integration**
- Test startup continuation when database initialization fails
- Test startup continuation when IMAP manager initialization times out
- Test startup continuation when account setup encounters errors
- Test startup continuation when services fail to initialize
- Test user feedback accuracy when services are unavailable due to startup failures

**Performance Integration**
- Test startup time impact with progress tracking enabled vs disabled
- Test memory usage impact of progress tracking components
- Test UI responsiveness during intensive initialization phases
- Test progress update frequency impact on overall startup performance

### Feature Tests

**Visual Progress Display**
- End-to-end test of full startup sequence with visual progress bar
- Test progress bar animation and smooth transitions between phases
- Test phase status icon updates (pending → in-progress → completed/failed)
- Test timeout countdown display accuracy and warning indicators
- Test error state visualization with appropriate user messaging
- Test theme integration and consistent styling throughout startup

**User Experience Scenarios**
- Test normal startup completion with all services successful
- Test startup with mixed success/failure states for different services
- Test startup with multiple service timeouts and graceful continuation
- Test startup interruption and recovery (Ctrl+C handling during startup)
- Test startup progress display in different terminal sizes and configurations

**Error Recovery Workflows**
- Test application functionality when database initialization fails
- Test email functionality when IMAP manager fails to initialize
- Test account management when account setup encounters errors
- Test service availability when background services fail to start
- Test user guidance and error messaging for failed startup components

## Mocking Requirements

**Time-Based Testing**
- **tokio::time::pause()** - Control time progression for timeout testing
- **Mock Instant** - Simulate different startup timing scenarios
- **Duration Mocking** - Test ETA calculations with controlled time advancement

**Service Initialization Mocking**
- **Database Mock** - Simulate database initialization success/failure/timeout scenarios
- **IMAP Manager Mock** - Control IMAP manager initialization behavior for testing
- **Account Setup Mock** - Simulate various account configuration scenarios
- **Service Manager Mock** - Control background service initialization outcomes

**UI Rendering Mocking**
- **Terminal Mock** - Test progress display in different terminal configurations
- **Theme Mock** - Verify styling and color application in progress components
- **Ratatui Test Backend** - Capture and verify rendered progress bar output

**Error Simulation**
- **Network Error Mock** - Simulate connection failures during service initialization
- **File System Error Mock** - Simulate database file access issues
- **Authentication Error Mock** - Simulate OAuth2 and account authentication failures
- **Timeout Simulation** - Force service initialization timeouts for error path testing

## Testing Strategy

### Test Execution Order
1. **Unit Tests First** - Verify individual component behavior and logic correctness
2. **Integration Tests** - Validate component interaction and startup flow integration
3. **Feature Tests** - End-to-end validation of user-facing functionality
4. **Performance Tests** - Ensure progress tracking doesn't impact startup performance

### Coverage Requirements
- **90% Line Coverage** - All core logic paths must be tested including error conditions
- **100% Error Path Coverage** - Every error condition and timeout scenario must be tested
- **UI Component Coverage** - All progress display components must have rendering tests
- **Integration Coverage** - All startup phases must have integration test coverage

### Performance Testing Criteria
- **Startup Time Impact** - Progress tracking should add <5% to overall startup time
- **Memory Usage** - Progress components should use <1MB additional memory
- **UI Responsiveness** - Progress updates should not block initialization tasks
- **Timeout Accuracy** - Timeout detection should be within 100ms of configured values