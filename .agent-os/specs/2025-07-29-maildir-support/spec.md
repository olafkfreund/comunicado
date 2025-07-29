# Spec Requirements Document

> Spec: Maildir Support Implementation
> Created: 2025-07-29
> Status: Planning

## Overview

Implement comprehensive Maildir format support for Comunicado to enable seamless import/export of emails from and to standard Maildir directories, allowing interoperability with mutt, dovecot, and other email clients while maintaining folder structure and metadata integrity.

## User Stories

### Email Migration from Legacy Clients

As a terminal power user migrating from mutt or other Maildir-based clients, I want to import my existing Maildir email archives into Comunicado, so that I can access my complete email history without losing important messages or folder organization.

**Detailed Workflow:** User selects "Import from Maildir" in the account management interface, browses to their existing Maildir directory (typically ~/Maildir or ~/.mail), selects which folders to import, and Comunicado preserves the folder hierarchy while converting emails to its internal storage format with proper metadata mapping.

### Email Backup and Portability

As a privacy-conscious user, I want to export my Comunicado emails to standard Maildir format, so that I can create portable backups and maintain the ability to switch to other email clients without vendor lock-in.

**Detailed Workflow:** User accesses the export functionality through the folder context menu or main account settings, selects specific folders or entire accounts for export, chooses destination directory, and Comunicado creates a properly structured Maildir hierarchy with all emails, metadata, and folder relationships preserved.

### Cross-Client Workflow Integration

As a system administrator, I want to use Comunicado alongside other email tools that work with Maildir, so that I can maintain my existing email processing scripts and workflows while benefiting from Comunicado's modern interface.

**Detailed Workflow:** User configures Comunicado to work with an existing Maildir setup, enabling bidirectional synchronization where changes made by external tools are reflected in Comunicado and vice versa, maintaining compatibility with existing automation and filtering scripts.

## Spec Scope

1. **Maildir Import Functionality** - Import emails from existing Maildir directories with full folder structure preservation
2. **Maildir Export Functionality** - Export Comunicado emails to standard Maildir format with metadata mapping
3. **Folder Structure Mapping** - Maintain hierarchical folder relationships between Comunicado's internal format and Maildir structure
4. **Metadata Preservation** - Preserve email flags, timestamps, and other metadata during import/export operations
5. **Progress Tracking Interface** - Provide user feedback during long-running import/export operations with progress indicators

## Out of Scope

- Real-time Maildir synchronization (this spec focuses on one-time import/export operations)
- Automatic detection of Maildir directories on the system
- Conversion between different Maildir variants or formats
- Integration with external Maildir management tools during runtime

## Expected Deliverable

1. **Import Wizard Interface** - Users can browse and select Maildir directories for import with folder selection capabilities
2. **Export Functionality** - Users can export selected folders or entire accounts to Maildir format through intuitive interface
3. **Metadata Integrity Verification** - All imported/exported emails maintain proper timestamps, flags, and folder relationships

## Spec Documentation

- Tasks: @.agent-os/specs/2025-07-29-maildir-support/tasks.md
- Technical Specification: @.agent-os/specs/2025-07-29-maildir-support/sub-specs/technical-spec.md
- Database Schema: @.agent-os/specs/2025-07-29-maildir-support/sub-specs/database-schema.md
- API Specification: @.agent-os/specs/2025-07-29-maildir-support/sub-specs/api-spec.md
- Tests Specification: @.agent-os/specs/2025-07-29-maildir-support/sub-specs/tests.md