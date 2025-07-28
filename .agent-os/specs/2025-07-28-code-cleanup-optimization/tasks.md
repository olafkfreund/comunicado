# Spec Tasks

These are the tasks to be completed for the spec detailed in @.agent-os/specs/2025-07-28-code-cleanup-optimization/spec.md

> Created: 2025-07-28
> Status: Completed

## Tasks

- [x] 1. **Duplicate Function Analysis and Removal**
    - [x] 1.1 Identify duplicate `clean_raw_email_content()` functions in database and UI layers
    - [x] 1.2 Remove UI layer cleaning function (~107 lines)
    - [x] 1.3 Update UI to use pre-cleaned content from database layer
    - [x] 1.4 Verify unified content cleaning approach works correctly

- [x] 2. **Dead Code Elimination**
    - [x] 2.1 Apply cargo fix for automatic unused import/variable cleanup
    - [x] 2.2 Remove unused functions in src/ui/content_preview.rs (5 functions)
    - [x] 2.3 Remove unused functions in src/ui/account_switcher.rs (2 functions)
    - [x] 2.4 Remove unused functions in src/ui/draft_list.rs (1 function)
    - [x] 2.5 Remove unused functions in src/ui/email_viewer.rs (2 functions)
    - [x] 2.6 Remove unused functions across backend modules (8 functions)
    - [x] 2.7 Remove unused ConnectionStream enum from src/imap/connection.rs
    - [x] 2.8 Verify all removals don't break functionality

- [x] 3. **Enhanced HTML Rendering Implementation**
    - [x] 3.1 Add ammonia dependency for HTML sanitization
    - [x] 3.2 Add pulldown-cmark dependency for markdown support
    - [x] 3.3 Implement w3m/lynx-style HTML renderer with multi-layered approach
    - [x] 3.4 Add HTML element handling (headers, lists, tables, links)
    - [x] 3.5 Implement terminal-specific styling and formatting
    - [x] 3.6 Create fallback mechanisms for different content types

- [x] 4. **Content Cleaning System Enhancement**
    - [x] 4.1 Enhance database layer content cleaning with Apple Mail patterns
    - [x] 4.2 Add aggressive fallback content extraction for edge cases
    - [x] 4.3 Implement 30+ email header pattern recognition
    - [x] 4.4 Add technical content detection and filtering
    - [x] 4.5 Verify raw headers and HTML source are properly filtered

- [x] 5. **Quality Verification and Testing**
    - [x] 5.1 Clear database and test with fresh email sync
    - [x] 5.2 Verify compiler warning reduction (target: 40%+ improvement)
    - [x] 5.3 Test HTML email display shows formatted content, not raw source
    - [x] 5.4 Verify technical headers are completely filtered out
    - [x] 5.5 Confirm application builds and runs successfully

## Results Achieved

- **Code Reduction:** 900+ lines of duplicate/dead code eliminated
- **Warning Reduction:** 54% improvement (81 to 37 warnings)
- **Functions Removed:** 13+ unused functions across multiple modules
- **Dependencies Added:** ammonia, pulldown-cmark for enhanced content processing
- **Architecture Improvement:** Unified content processing with single responsibility