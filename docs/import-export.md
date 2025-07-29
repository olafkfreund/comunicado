# Import and Export

Comunicado provides comprehensive import and export capabilities, making it easy to migrate from other email clients, backup your data, and maintain compatibility with standard email formats.

## Maildir Support

Maildir is a standard email storage format used by many email clients and servers. Comunicado's Maildir support ensures compatibility with a wide range of applications and provides a reliable way to backup and transfer your email data.

### Understanding Maildir Format

**Structure Overview**
Maildir stores each email as a separate file in a specific directory structure:
```
/path/to/maildir/
├── INBOX/
│   ├── new/        # Unread messages
│   ├── cur/        # Read messages
│   └── tmp/        # Temporary files
├── Sent/
├── Drafts/
└── Archive/
```

**File Naming Convention**
Each email file follows the Maildir naming standard:
- `timestamp.unique_id.hostname` for new messages
- `timestamp.unique_id.hostname:2,flags` for processed messages

**Message Flags**
Maildir uses single-letter flags to indicate message status:
- `S` - Seen (read)
- `R` - Replied
- `F` - Flagged (important)
- `T` - Trashed (deleted)
- `D` - Draft

### Exporting to Maildir

**Complete Account Export**
Export all folders and messages from a Comunicado account:

1. Access the import/export menu (`Ctrl+E`)
2. Select "Export to Maildir"
3. Choose the account to export
4. Select the destination directory
5. Review the export preview
6. Confirm the export operation

**Export Preview**
Before exporting, Comunicado shows:
- Total number of folders to export
- Message count per folder
- Estimated export size
- Estimated completion time
- Folder structure preview

**Selective Export**
Export specific folders or date ranges:
- Choose individual folders to export
- Set date range filters
- Exclude certain message types
- Apply size or count limits

**Export Progress**
During export, monitor progress through:
- Real-time folder completion status
- Message processing rate
- Remaining time estimates
- Error reporting and recovery

### Importing from Maildir

**Source Validation**
Before importing, Comunicado validates the Maildir structure:
- Checks for proper directory hierarchy
- Validates file naming conventions
- Tests file accessibility and permissions
- Reports any structural issues

**Import Preview**
Review what will be imported:
- Discovered folder structure
- Message counts per folder
- File size totals
- Potential import conflicts

**Import Process**
The import operation includes:
- Message parsing and validation
- Header extraction and processing
- Attachment handling and storage
- Flag and status conversion
- Database integration and indexing

**Conflict Resolution**
When duplicate messages are detected:
- Skip duplicates (default)
- Replace existing messages
- Keep both versions
- User decision per conflict

### Folder Mapping

**Automatic Mapping**
Comunicado automatically maps common folder names:
- `INBOX` → Inbox
- `Sent` → Sent Messages
- `Drafts` → Drafts
- `Trash` → Deleted Messages
- `Junk` → Spam

**Custom Mapping**
Override automatic mapping for:
- Non-standard folder names
- Multiple language folder names
- Custom organizational structures
- Provider-specific folders

## Other Import Formats

### mbox Format
**Single File Email Storage**
Import from mbox files used by:
- Thunderbird
- Apple Mail
- Pine/Alpine
- Many Unix mail systems

**mbox Import Process**
1. Select mbox file or directory
2. Choose target Comunicado account
3. Map mbox folders to Comunicado folders
4. Configure message parsing options
5. Start import with progress monitoring

### PST Files (Outlook)
**Microsoft Outlook Data**
Limited support for PST file import:
- Extract message headers and body
- Handle basic attachments
- Import folder structure
- Convert Outlook-specific features

**PST Import Limitations**
- Requires external PST parsing tools
- May not preserve all metadata
- Complex formatting might be simplified
- Encrypted PST files need pre-processing

### Thunderbird Import
**Direct Profile Import**
Import directly from Thunderbird profiles:
- Automatic profile detection
- Message database conversion
- Address book import
- Settings migration assistance

**Supported Thunderbird Data**
- All message folders and content
- Message filters and rules
- Account configurations
- Address book contacts

## Export Options

### Backup Formats
**Complete Backup**
Full Comunicado data export including:
- All email messages and attachments
- Calendar events and appointments
- Contact information
- Account configurations
- User preferences and settings

**Incremental Backup**
Export only data changed since last backup:
- New and modified messages
- Updated calendar events
- Changed contacts
- Modified settings

### Standard Formats
**EML Files**
Export individual messages as standard EML files:
- RFC822 compliant format
- Preserves all headers and metadata
- Includes attachments
- Compatible with most email clients

**vCard Contacts**
Export contact information:
- Standard vCard 3.0/4.0 format
- Contact photos and attachments
- Custom field preservation
- Group membership information

**iCalendar Events**
Export calendar data:
- Standard iCS format
- Event attachments and notes
- Recurring event patterns
- Time zone information

## Migration Assistance

### From Popular Clients
**Thunderbird Migration**
Step-by-step migration from Thunderbird:
1. Export Thunderbird profile location
2. Close Thunderbird completely
3. Run Comunicado migration wizard
4. Select Thunderbird profile directory
5. Choose data to migrate
6. Complete automatic migration

**Apple Mail Migration**
Migrate from macOS Mail app:
1. Locate Mail data directory
2. Export mailboxes to mbox format
3. Use Comunicado's mbox import
4. Manual account reconfiguration
5. Settings and preferences transfer

**Outlook Migration**
Migrate from Microsoft Outlook:
1. Export to PST file (if not already)
2. Use third-party PST to mbox converter
3. Import converted mbox files
4. Reconfigure account settings
5. Manual filter and rule recreation

### Server-to-Server Migration
**IMAP Server Migration**
Transfer between IMAP servers:
1. Configure both accounts in Comunicado
2. Use server-to-server transfer feature
3. Select folders and date ranges
4. Monitor transfer progress
5. Verify data integrity

**Account Consolidation**
Merge multiple accounts:
- Combine similar folders
- Deduplicate messages
- Preserve message threading
- Update contact references

## Data Validation and Integrity

### Import Validation
**Message Integrity Checks**
During import operations:
- Verify message structure and headers
- Validate attachment integrity
- Check character encoding consistency
- Test message parsing accuracy

**Error Handling**
When import errors occur:
- Log detailed error information
- Continue processing remaining messages
- Provide manual error resolution options
- Generate import summary reports

### Export Validation
**Data Completeness**
Before and after export:
- Compare message counts
- Verify attachment preservation
- Check metadata accuracy
- Validate folder structure

**Format Compliance**
Ensure exported data meets standards:
- RFC compliance for email formats
- Standard calendar format adherence
- Contact format validation
- Character encoding verification

## Automation and Scripting

### Batch Operations
**Automated Import/Export**
Script large-scale operations:
- Command-line import/export tools
- Batch processing multiple accounts
- Scheduled backup operations
- Integration with system backup tools

**Configuration Files**
Define import/export operations in configuration:
- Source and destination specifications
- Filter and mapping rules
- Error handling preferences
- Progress reporting options

### Integration Points
**External Tools**
Communicate with other applications:
- Backup software integration
- Email archiving systems
- Compliance and legal tools
- Data analysis applications

**API Access**
Programmatic access to import/export:
- REST API for data operations
- WebDAV for calendar data
- Standard protocols for integration
- Custom plugin development

## Performance Considerations

### Large Dataset Handling
**Memory Management**
For importing large amounts of data:
- Streaming processing for large files
- Memory-efficient parsing algorithms
- Progress checkpointing and resume
- Resource usage monitoring

**Network Optimization**
During server transfers:
- Parallel connection handling
- Bandwidth throttling options
- Resume interrupted transfers
- Compression where supported

### Storage Efficiency
**Space Optimization**
Minimize storage requirements:
- Duplicate detection and elimination
- Compression for archived data
- Efficient database storage
- Temporary file cleanup

## Compliance and Legal Considerations

### Data Retention
**Export for Compliance**
Meet legal requirements:
- Complete audit trail preservation
- Metadata retention requirements
- Date range export capabilities
- Searchable export formats

**Privacy Protection**
During export operations:
- Sensitive data redaction options
- Encryption for exported data
- Access control for export files
- Secure deletion of temporary data

### Format Standards
**Industry Compatibility**
Ensure exports work with:
- Legal discovery tools
- Archival systems
- Regulatory compliance platforms
- Industry-standard formats

This comprehensive import and export system makes Comunicado a flexible hub for your email data, whether you're migrating from another client, setting up backup procedures, or need to share data with other systems. The focus on standard formats ensures your data remains accessible and portable.