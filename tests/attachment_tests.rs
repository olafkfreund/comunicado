//! Comprehensive attachment handling tests for various email providers and scenarios
//!
//! This test suite validates:
//! - Attachment parsing from different MIME structures
//! - Type detection for various file formats
//! - Viewer functionality for supported formats
//! - Safety validation for potentially dangerous files
//! - Performance with large attachments

use comunicado::email::{AttachmentInfo, AttachmentType, AttachmentViewer, StoredAttachment};
use comunicado::mime::decode_mime_header;
use std::collections::HashMap;
use tokio_test;

/// Test data representing attachments from various email providers
struct TestAttachment {
    filename: String,
    content_type: String,
    size: usize,
    data: Vec<u8>,
    expected_type: AttachmentType,
    is_viewable: bool,
    provider: &'static str,
}

impl TestAttachment {
    fn to_stored_attachment(&self) -> StoredAttachment {
        StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: self.filename.clone(),
            content_type: self.content_type.clone(),
            size: self.size as i64,
            content_id: None,
            is_inline: false,
            data: Some(self.data.clone()),
            file_path: None,
        }
    }
}

/// Create test data for various attachment types and providers
fn create_test_attachments() -> Vec<TestAttachment> {
    vec![
        // Gmail PDF attachment
        TestAttachment {
            filename: "document.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size: 1024 * 50, // 50KB
            data: create_pdf_test_data(),
            expected_type: AttachmentType::Pdf,
            is_viewable: true,
            provider: "Gmail",
        },
        // Outlook Word document
        TestAttachment {
            filename: "report.docx".to_string(),
            content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                .to_string(),
            size: 1024 * 200, // 200KB
            data: create_word_test_data(),
            expected_type: AttachmentType::Word,
            is_viewable: true,
            provider: "Outlook",
        },
        // Yahoo JPEG image
        TestAttachment {
            filename: "photo.jpg".to_string(),
            content_type: "image/jpeg".to_string(),
            size: 1024 * 100, // 100KB
            data: create_jpeg_test_data(),
            expected_type: AttachmentType::Jpeg,
            is_viewable: true,
            provider: "Yahoo",
        },
        // Generic IMAP PNG image
        TestAttachment {
            filename: "screenshot.png".to_string(),
            content_type: "image/png".to_string(),
            size: 1024 * 75, // 75KB
            data: create_png_test_data(),
            expected_type: AttachmentType::Png,
            is_viewable: true,
            provider: "Generic IMAP",
        },
        // ProtonMail text file
        TestAttachment {
            filename: "notes.txt".to_string(),
            content_type: "text/plain".to_string(),
            size: 2048, // 2KB
            data: b"This is a test text file with some content.\nLine 2\nLine 3".to_vec(),
            expected_type: AttachmentType::Text,
            is_viewable: true,
            provider: "ProtonMail",
        },
        // Gmail ZIP archive
        TestAttachment {
            filename: "files.zip".to_string(),
            content_type: "application/zip".to_string(),
            size: 1024 * 500, // 500KB
            data: create_zip_test_data(),
            expected_type: AttachmentType::Zip,
            is_viewable: true,
            provider: "Gmail",
        },
        // Outlook Excel spreadsheet
        TestAttachment {
            filename: "data.xlsx".to_string(),
            content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                .to_string(),
            size: 1024 * 150, // 150KB
            data: create_excel_test_data(),
            expected_type: AttachmentType::Excel,
            is_viewable: true,
            provider: "Outlook",
        },
        // Potentially dangerous executable (should be marked unsafe)
        TestAttachment {
            filename: "malware.exe".to_string(),
            content_type: "application/octet-stream".to_string(),
            size: 1024 * 10,                                                  // 10KB
            data: b"MZ\x90\x00".repeat(2560).into_iter().flatten().collect(), // Fake PE header
            expected_type: AttachmentType::Unknown,
            is_viewable: false,
            provider: "Unknown",
        },
        // MIME-encoded filename test (common in international emails)
        TestAttachment {
            filename: "=?UTF-8?B?44OG44K544OI44OV44Kh44Kk44OrLnR4dA==?=".to_string(), // テストファイル.txt in base64
            content_type: "text/plain; charset=UTF-8".to_string(),
            size: 512,
            data: "テスト内容です。\nThis is test content in Japanese."
                .as_bytes()
                .to_vec(),
            expected_type: AttachmentType::Text,
            is_viewable: true,
            provider: "International",
        },
    ]
}

/// Create minimal valid PDF test data
fn create_pdf_test_data() -> Vec<u8> {
    let mut pdf_data = Vec::new();
    pdf_data.extend_from_slice(b"%PDF-1.4\n");
    pdf_data.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    pdf_data.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    pdf_data.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n");
    pdf_data.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
    pdf_data.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n%%EOF\n");
    pdf_data.resize(1024 * 50, b' '); // Pad to expected size
    pdf_data
}

/// Create minimal valid Word document test data (ZIP-based)
fn create_word_test_data() -> Vec<u8> {
    let mut word_data = Vec::new();
    word_data.extend_from_slice(b"PK\x03\x04"); // ZIP signature
    word_data.extend_from_slice(&[0u8; 26]); // ZIP header
    word_data.extend_from_slice(b"[Content_Types].xml"); // Typical Word file structure
    word_data.resize(1024 * 200, 0); // Pad to expected size
    word_data
}

/// Create minimal valid JPEG test data
fn create_jpeg_test_data() -> Vec<u8> {
    let mut jpeg_data = Vec::new();
    jpeg_data.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]); // JPEG signature
    jpeg_data.extend_from_slice(&[0x00, 0x10]); // Length
    jpeg_data.extend_from_slice(b"JFIF\x00"); // JFIF identifier
    jpeg_data.resize(1024 * 100, 0xFF); // Pad with typical JPEG data
    jpeg_data.extend_from_slice(&[0xFF, 0xD9]); // JPEG end marker
    jpeg_data
}

/// Create minimal valid PNG test data
fn create_png_test_data() -> Vec<u8> {
    let mut png_data = Vec::new();
    png_data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]); // PNG signature
    png_data.extend_from_slice(b"\x00\x00\x00\x0DIHDR"); // IHDR chunk
    png_data.resize(1024 * 75, 0); // Pad to expected size
    png_data
}

/// Create minimal valid ZIP test data
fn create_zip_test_data() -> Vec<u8> {
    let mut zip_data = Vec::new();
    zip_data.extend_from_slice(b"PK\x03\x04"); // ZIP signature
    zip_data.extend_from_slice(&[0u8; 26]); // ZIP header
    zip_data.extend_from_slice(b"test.txt"); // Filename
    zip_data.extend_from_slice(b"Hello, World!"); // File content
    zip_data.resize(1024 * 500, 0); // Pad to expected size
    zip_data
}

/// Create minimal valid Excel test data (ZIP-based like Word)
fn create_excel_test_data() -> Vec<u8> {
    let mut excel_data = Vec::new();
    excel_data.extend_from_slice(b"PK\x03\x04"); // ZIP signature
    excel_data.extend_from_slice(&[0u8; 26]); // ZIP header
    excel_data.extend_from_slice(b"xl/workbook.xml"); // Typical Excel file structure
    excel_data.resize(1024 * 150, 0); // Pad to expected size
    excel_data
}

#[tokio::test]
async fn test_attachment_type_detection() {
    let test_attachments = create_test_attachments();

    for test_attachment in &test_attachments {
        // Test content type detection
        let type_from_content = AttachmentType::from_content_type(&test_attachment.content_type);
        assert_eq!(
            type_from_content, test_attachment.expected_type,
            "Content type detection failed for {} from {}",
            test_attachment.filename, test_attachment.provider
        );

        // Test filename detection
        let decoded_filename = decode_mime_header(&test_attachment.filename);
        let type_from_filename = AttachmentType::from_filename(&decoded_filename);

        // For most cases, both should match, except for generic content types
        if test_attachment.content_type != "application/octet-stream" {
            assert!(
                type_from_filename == test_attachment.expected_type
                    || type_from_content == test_attachment.expected_type,
                "Neither filename nor content type detection worked for {} from {}",
                test_attachment.filename,
                test_attachment.provider
            );
        }
    }
}

#[tokio::test]
async fn test_attachment_info_creation() {
    let test_attachments = create_test_attachments();

    for test_attachment in &test_attachments {
        let stored = test_attachment.to_stored_attachment();
        let attachment_info = AttachmentInfo::from_stored(stored);

        // Verify basic properties
        assert_eq!(
            attachment_info.attachment_type,
            test_attachment.expected_type
        );
        assert_eq!(attachment_info.stored.size as usize, test_attachment.size);

        // Test display name (should decode MIME if needed)
        let expected_display_name = if test_attachment.filename.contains("=?") {
            decode_mime_header(&test_attachment.filename)
        } else {
            test_attachment.filename.clone()
        };
        assert_eq!(attachment_info.display_name, expected_display_name);

        // Test size formatting
        let formatted_size = attachment_info.format_size();
        assert!(!formatted_size.is_empty());
        assert!(
            formatted_size.contains("KB")
                || formatted_size.contains("MB")
                || formatted_size.contains("B")
        );

        // Test safety classification
        let is_safe_expected = !test_attachment.filename.ends_with(".exe");
        assert_eq!(attachment_info.is_safe, is_safe_expected);
    }
}

#[tokio::test]
async fn test_attachment_viewer_functionality() {
    let mut viewer = AttachmentViewer::default();
    let test_attachments = create_test_attachments();

    for test_attachment in &test_attachments {
        let stored = test_attachment.to_stored_attachment();
        let attachment_info = AttachmentInfo::from_stored(stored);

        // Test viewing the attachment
        let view_result = viewer
            .view_attachment(&attachment_info, &test_attachment.data)
            .await;

        match view_result {
            comunicado::email::ViewResult::Content(lines) => {
                assert!(
                    !lines.is_empty(),
                    "Viewer should produce content for {}",
                    test_attachment.filename
                );

                // Verify that the content contains expected elements
                let content_text = lines
                    .iter()
                    .map(|line| {
                        line.spans
                            .iter()
                            .map(|span| span.content.as_ref())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // Should contain file information
                assert!(
                    content_text.contains(&test_attachment.filename)
                        || content_text.contains(&attachment_info.display_name),
                    "Content should contain filename for {}",
                    test_attachment.filename
                );

                // Should contain type information
                assert!(
                    content_text.contains(test_attachment.expected_type.description()),
                    "Content should contain type description for {}",
                    test_attachment.filename
                );
            }
            comunicado::email::ViewResult::ExternalViewer(_) => {
                // Some files might require external viewers
                assert!(
                    !test_attachment.is_viewable,
                    "File {} should be viewable internally but requires external viewer",
                    test_attachment.filename
                );
            }
            comunicado::email::ViewResult::NotSupported(reason) => {
                assert!(
                    !test_attachment.is_viewable,
                    "File {} should be viewable but got not supported: {}",
                    test_attachment.filename, reason
                );
            }
            comunicado::email::ViewResult::Error(error) => {
                panic!(
                    "Unexpected error viewing {}: {}",
                    test_attachment.filename, error
                );
            }
        }

        // Test that viewer state is properly updated
        assert!(
            viewer.has_content(),
            "Viewer should have content after viewing {}",
            test_attachment.filename
        );
        assert!(
            viewer.current_attachment().is_some(),
            "Viewer should track current attachment"
        );

        // Test clearing
        viewer.clear();
        assert!(
            !viewer.has_content(),
            "Viewer should be clear after clearing"
        );
        assert!(
            viewer.current_attachment().is_none(),
            "Viewer should not track attachment after clearing"
        );
    }
}

#[tokio::test]
async fn test_mime_header_decoding() {
    let test_cases = vec![
        // UTF-8 Base64 encoded
        (
            "=?UTF-8?B?44OG44K544OI44OV44Kh44Kk44OrLnR4dA==?=",
            "テストファイル.txt",
        ),
        // UTF-8 Quoted-Printable
        (
            "=?UTF-8?Q?test_file_with_spaces.txt?=",
            "test_file_with_spaces.txt",
        ),
        // ISO-8859-1 encoded
        ("=?ISO-8859-1?Q?caf=E9.txt?=", "café.txt"),
        // Multiple encoded words
        (
            "=?UTF-8?B?44OG44K544OI?= =?UTF-8?B?44OV44Kh44Kk44Or?=.txt",
            "テストファイル.txt",
        ),
        // Plain ASCII (should pass through unchanged)
        ("document.pdf", "document.pdf"),
        // Empty string
        ("", ""),
    ];

    for (encoded, expected) in test_cases {
        let decoded = decode_mime_header(encoded);
        assert_eq!(decoded, expected, "MIME decoding failed for: {}", encoded);
    }
}

#[tokio::test]
async fn test_attachment_type_properties() {
    let test_cases = vec![
        (AttachmentType::Jpeg, true, false, false, true),
        (AttachmentType::Png, true, false, false, true),
        (AttachmentType::Pdf, false, true, false, false),
        (AttachmentType::Word, false, true, false, false),
        (AttachmentType::Zip, false, false, true, false),
        (AttachmentType::Text, false, false, false, true),
        (AttachmentType::Unknown, false, false, false, false),
    ];

    for (attachment_type, is_image, is_document, is_archive, is_previewable) in test_cases {
        assert_eq!(
            attachment_type.is_image(),
            is_image,
            "{:?} image classification",
            attachment_type
        );
        assert_eq!(
            attachment_type.is_document(),
            is_document,
            "{:?} document classification",
            attachment_type
        );
        assert_eq!(
            attachment_type.is_archive(),
            is_archive,
            "{:?} archive classification",
            attachment_type
        );
        assert_eq!(
            attachment_type.is_previewable(),
            is_previewable,
            "{:?} previewable classification",
            attachment_type
        );

        // Verify each type has a non-empty description and icon
        assert!(
            !attachment_type.description().is_empty(),
            "{:?} should have description",
            attachment_type
        );
        assert!(
            !attachment_type.icon().is_empty(),
            "{:?} should have icon",
            attachment_type
        );
    }
}

#[tokio::test]
async fn test_large_attachment_handling() {
    // Test with a reasonably large attachment to ensure performance
    let large_text_data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(10000);
    let large_attachment = TestAttachment {
        filename: "large_document.txt".to_string(),
        content_type: "text/plain".to_string(),
        size: large_text_data.len(),
        data: large_text_data.as_bytes().to_vec(),
        expected_type: AttachmentType::Text,
        is_viewable: true,
        provider: "Test",
    };

    let stored = large_attachment.to_stored_attachment();
    let attachment_info = AttachmentInfo::from_stored(stored);

    let mut viewer = AttachmentViewer::default();

    // This should complete within a reasonable time and not crash
    let start = std::time::Instant::now();
    let view_result = viewer
        .view_attachment(&attachment_info, &large_attachment.data)
        .await;
    let duration = start.elapsed();

    // Should complete within 5 seconds even for large files
    assert!(
        duration.as_secs() < 5,
        "Large attachment viewing took too long: {:?}",
        duration
    );

    match view_result {
        comunicado::email::ViewResult::Content(lines) => {
            assert!(!lines.is_empty(), "Large attachment should produce content");
            // Content should be truncated if too large for preview
            assert!(
                lines.len() <= 1100,
                "Content should be truncated for large files"
            ); // ~1000 lines + headers
        }
        _ => panic!("Large text attachment should be viewable"),
    }
}

#[tokio::test]
async fn test_security_validation() {
    let dangerous_files = vec![
        ("malware.exe", "application/octet-stream"),
        ("virus.bat", "application/x-msdos-program"),
        ("script.vbs", "text/vbscript"),
        ("trojan.scr", "application/octet-stream"),
        ("payload.jar", "application/java-archive"),
    ];

    for (filename, content_type) in dangerous_files {
        let dangerous_attachment = TestAttachment {
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            size: 1024,
            data: vec![0; 1024],
            expected_type: AttachmentType::Unknown,
            is_viewable: false,
            provider: "Security Test",
        };

        let stored = dangerous_attachment.to_stored_attachment();
        let attachment_info = AttachmentInfo::from_stored(stored);

        // Should be marked as unsafe
        assert!(
            !attachment_info.is_safe,
            "File {} should be marked as unsafe",
            filename
        );

        // Viewer should still handle it but with warnings
        let mut viewer = AttachmentViewer::default();
        let view_result = viewer
            .view_attachment(&attachment_info, &dangerous_attachment.data)
            .await;

        match view_result {
            comunicado::email::ViewResult::Content(lines) => {
                let content_text = lines
                    .iter()
                    .map(|line| {
                        line.spans
                            .iter()
                            .map(|span| span.content.as_ref())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // Should contain safety warnings
                assert!(
                    content_text.contains("unsafe")
                        || content_text.contains("⚠️")
                        || content_text.contains("Potentially unsafe"),
                    "Content should contain safety warning for {}",
                    filename
                );
            }
            _ => {} // Other results are also acceptable for unsafe files
        }
    }
}

#[cfg(test)]
mod provider_specific_tests {
    use super::*;

    #[tokio::test]
    async fn test_gmail_attachment_parsing() {
        // Gmail typically uses standard MIME types but may have specific encoding
        let gmail_attachment = TestAttachment {
            filename: "=?UTF-8?B?UmVwb3J0X1E0XzIwMjQuZG9jeA==?=".to_string(), // Report_Q4_2024.docx
            content_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                .to_string(),
            size: 1024 * 300,
            data: create_word_test_data(),
            expected_type: AttachmentType::Word,
            is_viewable: true,
            provider: "Gmail",
        };

        let stored = gmail_attachment.to_stored_attachment();
        let attachment_info = AttachmentInfo::from_stored(stored);

        assert_eq!(attachment_info.display_name, "Report_Q4_2024.docx");
        assert_eq!(attachment_info.attachment_type, AttachmentType::Word);
    }

    #[tokio::test]
    async fn test_outlook_attachment_parsing() {
        // Outlook may use different MIME encoding
        let outlook_attachment = TestAttachment {
            filename: "meeting_notes.pdf".to_string(),
            content_type: "application/pdf; name=\"meeting_notes.pdf\"".to_string(),
            size: 1024 * 75,
            data: create_pdf_test_data(),
            expected_type: AttachmentType::Pdf,
            is_viewable: true,
            provider: "Outlook",
        };

        let stored = outlook_attachment.to_stored_attachment();
        let attachment_info = AttachmentInfo::from_stored(stored);

        // Should correctly parse content type even with parameters
        assert_eq!(attachment_info.attachment_type, AttachmentType::Pdf);
        assert!(attachment_info.is_safe);
    }
}
