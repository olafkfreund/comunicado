# What Is Missing - Complete Implementation Roadmap

> Generated: 2025-01-14
> Status: Ready for Implementation
> Total Items: 150+ across 12 categories

## 🔴 **CRITICAL - User-Facing Features (Priority 1)**

### Core User Experience
- [ ] **Save Attachment Functionality** (`src/events.rs:1367`)
  - Replace "Save attachment functionality coming soon" with working implementation
  - File dialog, download progress, error handling

- [ ] **AI Bulk Email Analysis** (`src/ui/ai_assistant_ui.rs:541`)
  - Replace "Bulk email analysis feature coming soon..." with functional UI
  - Batch email processing, analysis results display

- [ ] **Settings Interface in Modular UI** (`src/ui/components/modular_ui.rs:512`)
  - Replace "Settings interface coming soon..." with working settings panel
  - Integration with existing SettingsUI

- [ ] **Help System** (`src/ui/components/modular_ui.rs:531`)
  - Replace "Help system coming soon..." with comprehensive help interface
  - Keyboard shortcuts reference, user guide, tooltips

## 🟠 **HIGH PRIORITY - Core Functionality (Priority 2)**

### Email Operations
- [ ] **Move Folder Operation** (`src/app.rs:3693`)
  - Implement actual IMAP folder move functionality
  - Drag-and-drop support, confirmation dialogs

- [ ] **Email Threading & Display**
  - [ ] Mark messages as read (`src/app.rs:3740`)
  - [ ] Email deletion (`src/events.rs:1726`)
  - [ ] Email archiving (`src/events.rs:1743`)
  - [ ] Mark as unread (`src/events.rs:1775`)

### Folder Management
- [ ] **IMAP Folder Operations** (`src/app.rs`)
  - [ ] Folder creation (line 3815, 3823)
  - [ ] Folder deletion (line 3841, 3842)
  - [ ] Folder renaming (line 3860, 3861)
  - [ ] Folder properties (line 3779)
  - [ ] Clear all messages (line 3879, 3880)
  - [ ] Subscribe/Unsubscribe (line 3909)

### Contact Management
- [ ] **Contact Editor** (`src/ui/components/contacts.rs:789`)
  - Replace placeholder with full contact editing interface
  - Form validation, field management, save/cancel

- [ ] **Contact Operations** (`src/ui/components/contacts.rs`)
  - [ ] Contact deletion (line 335)
  - [ ] Contact export (line 339)
  - [ ] Contact import (line 343)

- [ ] **Address Book View** (`src/app.rs:4314`)
  - Full address book interface implementation

### Compose & Draft Management
- [ ] **Compose View** (`src/ui/components/email.rs:285-286`)
  - Replace placeholder with functional compose interface
  - Rich text editing, attachments, recipient management

- [ ] **Draft Management** (`src/ui/mod.rs:1865`)
  - Proper draft editing with pre-filled recipients and subject
  - Auto-save, draft recovery

## 🟡 **MEDIUM PRIORITY - Advanced Features (Priority 3)**

### Calendar System
- [ ] **Event Management** (`src/app.rs`)
  - [ ] Event editing dialog (line 4814)
  - [ ] Event details popup (line 4847)
  - [ ] Todo completion toggle (line 4890)

- [ ] **Calendar Views** (`src/events.rs`)
  - [ ] Event details view (line 978)
  - [ ] Todos view (line 1010)

- [ ] **Calendar Search** (`src/calendar/ui.rs:1336`)
  - Search mode implementation for calendar events

- [ ] **Outlook Calendar Sync** (`src/calendar/manager.rs:592`)
  - Full Outlook calendar integration

### Advanced Email Features
- [ ] **Advanced Email Filters** (`src/email/advanced_filters.rs:528`)
  - Complete field filtering implementation
  - Regex matching support (line 556)

- [ ] **Filter UI** (`src/email/advanced_filters_ui.rs`)
  - [ ] Filter editor key handling (line 654)
  - [ ] Template key handling (line 659)
  - [ ] Testing key handling (line 664)
  - [ ] Statistics key handling (line 669)

- [ ] **Email Search** (`src/tea/update.rs:444`)
  - Advanced email search functionality

### CLI Enhancements
- [ ] **Import Functions** (`src/cli.rs`)
  - [ ] Generic account config import (line 1418)
  - [ ] Thunderbird profile import (line 1430)
  - [ ] Database cleanup for account removal (line 2444)
  - [ ] Detailed statistics (line 3403)
  - [ ] Re-authentication flow (line 5093)

- [ ] **Capability Checking** (`src/cli.rs:1535`)
  - IMAP server capability detection and validation

## 🟢 **LOW PRIORITY - System Integration (Priority 4)**

### Migration System
- [ ] **Thunderbird Migration** (`src/migration/thunderbird.rs`)
  - [ ] JavaScript parsing (line 356)
  - [ ] Preference parsing (line 367)
  - [ ] Folder scanning (line 411)
  - [ ] Email migration (line 425)
  - [ ] Address book migration (line 442)
  - [ ] Filter migration (line 458)
  - [ ] Message conversion (line 474)

- [ ] **Migration Engine** (`src/migration/migration_engine.rs`)
  - [ ] Proper permission checking (line 391)
  - [ ] Migration logic for other types (line 418)
  - [ ] Conflict detection (line 502)
  - [ ] Email migration logic (line 532)
  - [ ] Contact migration logic (line 551)
  - [ ] Disk space checking (line 619)

- [ ] **Migration UI** (`src/migration/migration_ui.rs`)
  - [ ] Configuration key handling (line 874)
  - [ ] Planning key handling (line 879)
  - [ ] Progress key handling (line 884)
  - [ ] History key handling (line 889)

### KDE Connect Integration
- [ ] **SMS Integration** (`src/mobile/kde_connect/simple_client.rs:394`)
  - SMS message listening via D-Bus

- [ ] **Notification Integration** (`src/mobile/kde_connect/simple_client.rs`)
  - [ ] Notification listening (line 412)
  - [ ] Notification reply (line 441)

### Maildir Support
- [ ] **Maildir Writer** (`src/maildir/writer.rs:184`)
  - Message flag updates implementation

- [ ] **Maildir Importer** (`src/email/maildir_importer.rs:694`)
  - Resume capability for interrupted imports

## 🔧 **TECHNICAL DEBT - Internal Systems (Priority 5)**

### Plugin System
- [ ] **Plugin Loader** (`src/plugins/loader.rs`)
  - [ ] Script plugin loading (line 192)
  - [ ] WASM plugin loading (line 225)

- [ ] **Notes Plugin** (`src/plugins/notes/`)
  - [ ] Search functionality (multiple files)
  - [ ] Indexing system completion
  - [ ] Storage layer completion

### TEA Command System
- [ ] **Command Execution** (`src/tea/command.rs`)
  - [ ] Database commands (line 302)
  - [ ] Network commands (line 308)
  - [ ] Filesystem commands (line 314)
  - [ ] UI commands (line 320)
  - [ ] System commands (line 326)

### TEA Update System
- [ ] **UI Updates** (`src/tea/update.rs`)
  - [ ] Filter panel toggle (line 204)
  - [ ] Context menu display (line 216)
  - [ ] Message move (line 409)
  - [ ] Calendar message handlers (line 553)
  - [ ] Contacts message handlers (line 612)
  - [ ] Account message handlers (line 645)

### RFC Standards
- [ ] **vCard/iCalendar Conversion** (`src/rfc_standards.rs`)
  - [ ] Contact to vCard conversion (line 193)
  - [ ] iCalendar parsing (line 258)
  - [ ] Event to iCalendar conversion (line 283)
  - [ ] Calcard Event conversion (line 302, 334)

### Keyboard System
- [ ] **Shortcuts UI** (`src/keyboard/shortcuts_ui.rs`)
  - [ ] Conflicts rendering (line 794)
  - [ ] Settings rendering (line 799)
  - [ ] Import/export rendering (line 804)
  - [ ] Context navigation (lines 898-919)

### Settings & Configuration
- [ ] **Settings Stubs** (`src/ui/settings_ui.rs`)
  - [ ] Connection testing (line 974)
  - [ ] Cache clearing logic (line 1169)

- [ ] **Account Manager** (`src/ui/account_manager_ui.rs:611`)
  - Actual connection testing implementation

### System Integration
- [ ] **Email Sync Service** (`src/email/async_sync_service.rs:220`)
  - Full message sync with database storage

- [ ] **System Integration** (`src/system/integration.rs`)
  - [ ] Email account sync (line 585)
  - [ ] Calendar account sync (line 592)

## 📋 **Implementation Strategy**

### Phase 1: Critical User Features (Week 1)
1. Save Attachment Functionality
2. Settings Interface in Modular UI
3. Help System
4. AI Bulk Email Analysis

### Phase 2: Core Email Operations (Week 2)
1. Email operations (delete, archive, mark read/unread)
2. Folder management (create, delete, rename)
3. Compose view implementation
4. Contact editor

### Phase 3: Advanced Features (Week 3)
1. Calendar event management
2. Advanced email filters
3. CLI enhancements
4. Email search

### Phase 4: System Integration (Week 4)
1. Migration system
2. KDE Connect integration
3. Maildir improvements
4. Plugin system

### Phase 5: Technical Debt (Week 5)
1. TEA command system
2. RFC standards completion
3. Keyboard system enhancements
4. Internal optimizations

## 🎯 **Success Metrics**

- **User-Facing Features**: 100% functional (no "coming soon" messages)
- **Core Operations**: All email/calendar/contact operations working
- **CLI Completeness**: All import/export functions implemented
- **System Integration**: Migration and external service integration
- **Code Quality**: All TODO items resolved, comprehensive testing

---

*Ready to transform Comunicado from 95% to 100% complete!* 🚀