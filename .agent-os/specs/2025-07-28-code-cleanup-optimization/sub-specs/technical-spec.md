# Technical Specification

This is the technical specification for the spec detailed in @.agent-os/specs/2025-07-28-code-cleanup-optimization/spec.md

> Created: 2025-07-28
> Version: 1.0.0

## Technical Requirements

- Remove duplicate `clean_raw_email_content()` functions from UI and database layers
- Eliminate unused/dead functions identified by compiler warnings
- Implement unified content processing at database layer only
- Establish w3m/lynx-style HTML rendering with proper sanitization
- Apply cargo fix for automatic cleanup of unused imports/variables
- Reduce compiler warnings by minimum 40%

## Approach Options

**Option A:** Incremental cleanup over multiple development cycles
- Pros: Less disruptive, gradual improvement
- Cons: Issues persist, slower user benefit delivery

**Option B:** Comprehensive cleanup in focused initiative (Selected)
- Pros: Immediate issue resolution, establishes clean architecture
- Cons: Requires focused effort, potential for regressions

**Option C:** Targeted fixes for user-visible issues only
- Pros: Quick user wins, minimal code changes
- Cons: Technical debt remains, future development impacted

**Rationale:** Option B selected because user-visible issues required immediate attention and technical debt was impacting development velocity significantly.

## External Dependencies

- **ammonia** - HTML sanitization and security filtering
- **pulldown-cmark** - Enhanced markdown support for content rendering
- **html2text** - Fallback HTML to text conversion

**Justification:** These libraries provide production-ready HTML processing with security considerations, eliminating need for custom implementation of complex HTML parsing and sanitization logic.

## Implementation Results

### Content Cleaning Architecture
- **Database Layer:** `StoredMessage::clean_raw_email_content()` with aggressive header filtering
- **UI Layer:** Direct use of pre-cleaned content, no duplicate processing
- **Pattern Recognition:** 30+ email header patterns for comprehensive filtering

### Code Quality Improvements
- **Functions Removed:** 13+ dead functions across multiple modules
- **Lines Eliminated:** 900+ lines of duplicate and unused code
- **Warnings Reduced:** From 81 to 37 (54% improvement)

### HTML Rendering System
- **Primary:** Enhanced w3m/lynx-style renderer with element handling
- **Sanitization:** ammonia-based HTML security filtering
- **Fallbacks:** html2text and basic parsing for edge cases
- **Terminal Optimization:** Proper styling and formatting for TUI display