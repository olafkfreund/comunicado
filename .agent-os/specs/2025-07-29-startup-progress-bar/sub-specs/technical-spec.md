# Technical Specification

This is the technical specification for the spec detailed in @.agent-os/specs/2025-07-29-startup-progress-bar/spec.md

> Created: 2025-07-29
> Version: 1.0.0

## Technical Requirements

### Progress Bar Architecture
- **Framework Integration:** Utilize existing Ratatui infrastructure with custom startup screen component
- **State Management:** Track initialization phases, timing, and error states through dedicated StartupProgressManager
- **UI Threading:** Non-blocking progress updates that don't interfere with async initialization tasks
- **Theme Integration:** Consistent styling with existing theme system and color palette
- **Terminal Compatibility:** Full compatibility with modern terminals (Kitty, Foot, Wezterm) and graceful fallback for legacy terminals

### Phase Tracking System
- **Phase Definition:** Enum-based phase tracking (Database, IMAP, AccountSetup, Services, DashboardServices)
- **Timing Integration:** Real-time progress tracking with timeout countdown and ETA calculations
- **Error Classification:** Distinguish between timeout errors, connection failures, and critical errors
- **State Persistence:** Maintain phase state throughout startup process without impacting performance

### Progress Visualization
- **Multi-Component Layout:** Overall progress gauge, current phase indicator, phase list with status icons, and detailed phase information
- **Animation Support:** Smooth progress bar animations and pulsing indicators for active phases
- **Status Icons:** Visual indicators for each phase state (⏳ pending, 🔄 in-progress, ✅ complete, ❌ failed, ⏰ timeout)
- **Timeout Visualization:** Countdown timers and warning indicators when phases approach timeout limits

### Error Handling Integration
- **Graceful Degradation:** Continue startup process even when non-critical services fail
- **User Communication:** Clear error messages explaining impact of failed services
- **Service Status Tracking:** Maintain record of which services successfully initialized for runtime behavior
- **Recovery Information:** Provide guidance on what functionality remains available after failures

### Performance Considerations
- **Minimal Overhead:** Progress tracking should not add significant startup time
- **Efficient Rendering:** Optimize UI updates to avoid impacting initialization performance
- **Memory Usage:** Lightweight state management that doesn't increase memory footprint
- **Async Compatibility:** Full compatibility with existing async initialization workflow

## Approach Options

**Option A: Full-Screen Startup Overlay** (Selected)
- Pros: Maximum visibility, dedicated screen real estate, professional appearance, clear focus on startup process
- Cons: Completely blocks access to application until initialization completes

**Option B: Header Progress Bar**
- Pros: Allows partial UI access, less intrusive, maintains application context
- Cons: Limited space for detailed information, less visible, harder to implement with current initialization flow

**Option C: Terminal Status Messages Only**
- Pros: Simple implementation, minimal UI changes, follows traditional CLI patterns
- Cons: Poor visual feedback, no progress indication, doesn't address core user experience issues

**Rationale:** Option A provides the best user experience by giving startup progress the attention it deserves during the significant 33+ second initialization period. The full-screen approach allows for comprehensive information display and creates a professional loading experience that matches modern application expectations.

## External Dependencies

**No New Dependencies Required**
- **Ratatui Components:** Leverage existing Gauge, Block, List, and Paragraph widgets
- **Chrono Integration:** Use existing chrono dependency for time calculations and duration formatting
- **Theme System:** Integrate with existing theme management for consistent styling
- **Existing Utilities:** Reuse progress formatting and helper functions from sync_progress.rs

**Justification:** The specification leverages existing infrastructure to minimize complexity and maintain consistency with the current codebase. All required UI components and timing utilities are already available.

## Implementation Architecture

### StartupProgressManager Structure
```rust
pub struct StartupProgressManager {
    phases: Vec<StartupPhase>,
    current_phase: usize,
    started_at: Instant,
    is_visible: bool,
    error_states: HashMap<String, StartupError>,
}

pub enum StartupPhase {
    Database { timeout: Duration, status: PhaseStatus },
    ImapManager { timeout: Duration, status: PhaseStatus },
    AccountSetup { timeout: Duration, status: PhaseStatus },
    Services { timeout: Duration, status: PhaseStatus },
    DashboardServices { timeout: Duration, status: PhaseStatus },
}

pub enum PhaseStatus {
    Pending,
    InProgress { started_at: Instant },
    Completed { duration: Duration },
    Failed { error: String },
    TimedOut { duration: Duration },
}
```

### Integration Points
- **main.rs:** Replace existing timeout logic with progress-aware initialization calls
- **app.rs:** Add progress manager field and update methods to report phase changes
- **ui/mod.rs:** Add StartupProgressScreen component for rendering
- **Initialization Methods:** Modify existing initialize_* methods to report progress updates

### UI Component Structure
- **StartupProgressScreen:** Main component managing full-screen startup display
- **PhaseProgressGauge:** Overall progress visualization with percentage and ETA
- **PhaseListView:** Detailed phase status list with icons and timing information
- **CurrentPhaseDetails:** Expanded information about active phase with timeout countdown
- **ErrorSummaryPanel:** Summary of failed services and impact description