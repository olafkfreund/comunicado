# CLAUDE.md

## Agent OS Documentation

### Product Context
- **Mission & Vision:** @.agent-os/product/mission.md
- **Technical Architecture:** @.agent-os/product/tech-stack.md
- **Development Roadmap:** @.agent-os/product/roadmap.md
- **Decision History:** @.agent-os/product/decisions.md

### Development Standards
- **Code Style:** @~/.agent-os/standards/code-style.md
- **Best Practices:** @~/.agent-os/standards/best-practices.md

### Project Management
- **Active Specs:** @.agent-os/specs/
- **Spec Planning:** Use `@~/.agent-os/instructions/create-spec.md`
- **Tasks Execution:** Use `@~/.agent-os/instructions/execute-tasks.md`

## Workflow Instructions

When asked to work on this codebase:

1. **First**, check @.agent-os/product/roadmap.md for current priorities
2. **Then**, follow the appropriate instruction file:
   - For new features: @.agent-os/instructions/create-spec.md
   - For tasks execution: @.agent-os/instructions/execute-tasks.md
3. **Always**, adhere to the standards in the files listed above

## Method Documentation Requirements

**CRITICAL**: Every time methods are added, modified, or removed, you MUST:

1. **Update Method Documentation** - Update docs/method-documentation.md and relevant method files
2. **Check UI Thread Blocking** - Ensure new methods don't block the UI (34 methods currently do)
3. **Add Rustdoc Comments** - Document all methods with proper rustdoc syntax
4. **Test Responsiveness** - Verify UI remains responsive during operations
5. **Update Status** - Mark methods as ✅ Complete, ⚠️ Needs Work, ❌ Stub, or 📝 Missing Docs

### Documentation Files to Maintain:
- `docs/method-documentation.md` - Overview and statistics
- `docs/core-methods.md` - Main application methods  
- `docs/ui-methods.md` - UI component methods
- `docs/email-methods.md` - Email functionality methods
- `docs/auth-methods.md` - Authentication methods
- `docs/calendar-methods.md` - Calendar feature methods
- `docs/services-methods.md` - Background service methods

### Critical UI Performance Rule:
**NEVER** add synchronous operations that block the UI thread. All IMAP, CalDAV, and network operations MUST use background processing with progress indicators.

## Important Notes

- Product-specific files in `.agent-os/product/` override any global standards
- User's specific instructions override (or amend) instructions found in `.agent-os/specs/...`
- Always adhere to established patterns, code style, and best practices documented above
- **Documentation maintenance is MANDATORY** for all code changes