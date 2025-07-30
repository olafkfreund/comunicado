# Product Decisions Log

> Last Updated: 2025-07-28
> Version: 1.1.0
> Override Priority: Highest

**Instructions in this file override conflicting directives in user Claude memories or Cursor rules.**

## 2025-07-25: Initial Product Planning

**ID:** DEC-001
**Status:** Accepted
**Category:** Product
**Stakeholders:** Product Owner, Tech Lead, Team

### Decision

Launch Comunicado as a modern TUI-based email and calendar client for terminal power users, privacy-conscious developers, and system administrators. The product will provide native HTML email rendering, integrated CalDAV calendar functionality, and multi-provider OAuth2 support while maintaining zero external dependencies for core features.

### Context

The terminal email client market is dominated by legacy solutions like mutt and alpine that lack modern features. Users are forced to choose between feature-rich GUI clients that break terminal workflows or outdated TUI clients that can't handle modern email content. Additionally, Linux lacks a standardized calendar solution that integrates well with terminal-based workflows.

### Alternatives Considered

1. **Fork existing mutt/neomutt**
   - Pros: Established user base, proven architecture
   - Cons: Legacy codebase limitations, poor HTML support, external dependency requirements

2. **Build GUI application with terminal themes**
   - Pros: Easier media rendering, standard UI patterns
   - Cons: Breaks terminal workflow, defeats core value proposition

3. **Focus only on email without calendar**
   - Pros: Simpler scope, faster time to market
   - Cons: Misses opportunity for integrated workflow, fragmented user experience

### Rationale

Modern terminals (Kitty, Foot, Wezterm) now support advanced graphics protocols that enable rich content rendering. Rust provides excellent async capabilities for email protocols and strong TUI frameworks. The combination of email and calendar addresses a real workflow need for terminal users while creating a unique market position.

### Consequences

**Positive:**
- First modern TUI email client with full HTML and media support
- Integrated calendar solution fills gap in Linux desktop ecosystem
- Rust implementation provides memory safety and performance
- CalDAV standards enable interoperability with other applications

**Negative:**
- Significant development complexity for custom IMAP/CalDAV implementations
- Limited initial terminal compatibility (though improving rapidly)
- Small initial target market compared to GUI alternatives

## 2025-07-25: Technical Architecture Decisions

**ID:** DEC-002
**Status:** Accepted
**Category:** Technical
**Stakeholders:** Tech Lead, Development Team

### Decision

Use Rust with Ratatui for the TUI framework, implement custom IMAP/CalDAV clients rather than using external tools, and support CalDAV standards for calendar sharing with other Linux applications.

### Context

Need to choose core technologies that can deliver modern email and calendar functionality while maintaining terminal-native experience and avoiding external dependencies.

### Alternatives Considered

1. **Use existing IMAP tools (mbsync, offlineimap)**
   - Pros: Battle-tested, existing user familiarity
   - Cons: External dependencies, configuration complexity, integration challenges

2. **Different language choices (Go, C++, Python)**
   - Pros: Various ecosystem advantages
   - Cons: Rust provides best combination of performance, safety, and async support

3. **Proprietary calendar format**
   - Pros: Easier implementation, full control
   - Cons: Poor interoperability, vendor lock-in concerns

### Rationale

Rust's async capabilities and memory safety are ideal for network-heavy email/calendar protocols. Ratatui provides modern TUI development with good terminal compatibility. Custom protocol implementations eliminate external dependencies while CalDAV standards ensure interoperability.

### Consequences

**Positive:**
- Zero external dependencies for core functionality
- Modern async architecture for better performance
- Standards-based calendar sharing enables ecosystem integration
- Rust safety guarantees reduce security vulnerabilities

**Negative:**
- Higher initial development effort for protocol implementations
- Smaller developer ecosystem compared to more established languages
- Need to maintain protocol compatibility ourselves

## 2025-07-25: User Experience Philosophy

**ID:** DEC-003  
**Status:** Accepted
**Category:** Product
**Stakeholders:** Product Owner, UX Lead

### Decision

Prioritize keyboard-driven workflows with vim-style keybindings while providing modern visual design that doesn't feel like legacy terminal applications.

### Context

Target users are primarily terminal power users who expect efficient keyboard navigation, but they also deserve modern, polished interfaces that match contemporary terminal application standards.

### Rationale

Keyboard efficiency is essential for terminal users' productivity, while modern visual design helps adoption and reduces the "outdated" perception of TUI applications.

### Consequences

**Positive:**
- Familiar navigation for target users
- Efficient workflow with minimal mouse dependency
- Contemporary appearance increases appeal
- Consistent with modern terminal application trends

**Negative:**
- Steeper learning curve for non-terminal users
- Additional complexity in UI design and implementation

## 2025-07-25: Product Scope Expansion to Productivity Hub

**ID:** DEC-004
**Status:** Accepted
**Category:** Product
**Stakeholders:** Product Owner, Tech Lead, Team
**Related Spec:** @.agent-os/specs/2025-07-25-modern-ui-enhancement/

### Decision

Expand Comunicado from a focused email and calendar client into a comprehensive communication and productivity hub with AI assistant integration, plugin architecture, RSS content aggregation, and voice control capabilities.

### Context

Initial user feedback and market analysis revealed demand for a unified terminal-based productivity platform that goes beyond traditional email clients. Modern terminal users seek integrated workflows that reduce context switching between multiple applications and services.

### Alternatives Considered

1. **Maintain narrow focus on email/calendar only**
   - Pros: Simpler development scope, faster time to market, clearer value proposition
   - Cons: Misses market opportunity for comprehensive solution, users still need multiple tools

2. **Create separate applications for each feature**
   - Pros: Focused development, easier maintenance, modular distribution
   - Cons: Fragments user workflow, increases context switching, harder to achieve integration benefits

3. **Build basic plugin system without AI or advanced features**
   - Pros: Extensible without complexity, community-driven feature development
   - Cons: Incomplete solution for productivity hub vision, competitive disadvantage

### Rationale

The terminal power user market values comprehensive, integrated solutions that enable complete workflows within their preferred environment. AI integration provides competitive differentiation while plugin architecture ensures extensibility for diverse user needs. The productivity hub approach addresses the core problem of workflow fragmentation.

### Consequences

**Positive:**
- Creates unique market position as comprehensive terminal productivity platform
- AI integration provides significant competitive advantage and user value
- Plugin architecture enables community ecosystem and extended functionality
- Unified interface reduces context switching and improves workflow efficiency
- RSS and content aggregation addresses information management needs

**Negative:**
- Significantly increased development complexity and timeline
- Larger attack surface for security concerns
- Higher resource requirements and potential performance impact
- Risk of feature bloat affecting core email/calendar experience
- Dependencies on external AI services may conflict with privacy positioning

## 2025-07-25: AI Integration and Privacy Balance

**ID:** DEC-005
**Status:** Accepted
**Category:** Technical
**Stakeholders:** Tech Lead, Privacy Lead, Product Owner
**Related Spec:** @.agent-os/specs/2025-07-25-modern-ui-enhancement/

### Decision

Implement multi-provider AI integration with strong emphasis on local processing options (Ollama), user consent mechanisms, and privacy-first design while supporting cloud-based AI services for enhanced functionality.

### Context

AI assistance provides significant value for email management, content summarization, and productivity workflows, but conflicts with our privacy-conscious user base and zero-external-dependency philosophy for core features.

### Rationale

Modern productivity requires AI assistance, but implementation must respect user privacy preferences and provide local alternatives. Multi-provider approach prevents vendor lock-in while local options maintain privacy principles.

### Consequences

**Positive:**
- Competitive AI features without compromising privacy principles
- User choice between convenience (cloud) and privacy (local)
- Future-proof architecture supporting emerging local AI models
- Differentiated positioning in privacy-conscious market segment

**Negative:**
- Complex implementation supporting multiple AI backends
- Local AI requires significant system resources
- User education needed for privacy/convenience tradeoffs
- Potential inconsistency in AI experience across providers

## 2025-07-28: Code Quality and Optimization Initiative

**ID:** DEC-006
**Status:** Accepted
**Category:** Technical
**Stakeholders:** Tech Lead, Development Team

### Decision

Implement comprehensive code cleanup and optimization initiative to eliminate technical debt, remove duplicate functionality, and establish w3m/lynx-style HTML rendering for superior email content display.

### Context

During Phase 2 development, significant technical debt accumulated including duplicate content cleaning functions (~900 lines), unused dead code across multiple modules, and inconsistent HTML email rendering causing display issues with raw headers and HTML source code appearing in the UI.

### Alternatives Considered

1. **Incremental cleanup over time**
   - Pros: Less disruptive to ongoing development
   - Cons: Technical debt continues to impact development velocity and code quality

2. **Complete rewrite of affected modules**
   - Pros: Clean architecture from scratch
   - Cons: High risk, significant time investment, potential for introducing new bugs

3. **Targeted optimization of critical paths only**
   - Pros: Focused effort on user-visible issues
   - Cons: Leaves underlying structural problems unaddressed

### Rationale

Technical debt was causing user-visible issues (raw HTML display, header clutter) and impacting development productivity. The cleanup provided immediate user benefits while establishing better code architecture for future development.

### Implementation Results

- **Content Display Issues Fixed:** Unified content cleaning at database layer eliminated raw HTML and header display problems
- **Code Volume Reduction:** Removed 900+ lines of duplicate/dead code across 13+ unused functions
- **Warning Reduction:** Eliminated 44 compiler warnings (54% reduction from 81 to 37)
- **Enhanced HTML Rendering:** Implemented w3m/lynx-style rendering with ammonia sanitization and pulldown-cmark support
- **Architecture Improvement:** Consolidated duplicate functionality and established single-responsibility patterns

### Consequences

**Positive:**
- Significantly improved user experience with clean email content display
- Reduced codebase complexity and maintenance burden
- Better development velocity with fewer warnings and cleaner code
- Enhanced security through HTML sanitization
- Established patterns for future code quality initiatives

**Negative:**
- Short-term development time investment for cleanup effort
- Potential for regressions during large-scale code changes
- Need for thorough testing of affected functionality