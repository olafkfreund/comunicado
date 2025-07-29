//! Integration test for attachment functionality
//! This tests the attachment handling without relying on other components

use comunicado::email::{AttachmentInfo, AttachmentType, StoredAttachment};

#[test]
fn test_basic_attachment_type_detection() {
    // Test various content types
    assert_eq!(
        AttachmentType::from_content_type("application/pdf"),
        AttachmentType::Pdf
    );
    assert_eq!(
        AttachmentType::from_content_type("image/jpeg"),
        AttachmentType::Jpeg
    );
    assert_eq!(
        AttachmentType::from_content_type("image/png"),
        AttachmentType::Png
    );
    assert_eq!(
        AttachmentType::from_content_type("text/plain"),
        AttachmentType::Text
    );
    assert_eq!(
        AttachmentType::from_content_type("application/zip"),
        AttachmentType::Zip
    );
    assert_eq!(
        AttachmentType::from_content_type("unknown/type"),
        AttachmentType::Unknown
    );
}

#[test]
fn test_filename_type_detection() {
    // Test various filename extensions
    assert_eq!(
        AttachmentType::from_filename("document.pdf"),
        AttachmentType::Pdf
    );
    assert_eq!(
        AttachmentType::from_filename("photo.jpg"),
        AttachmentType::Jpeg
    );
    assert_eq!(
        AttachmentType::from_filename("image.PNG"),
        AttachmentType::Png
    ); // Case insensitive
    assert_eq!(
        AttachmentType::from_filename("notes.txt"),
        AttachmentType::Text
    );
    assert_eq!(
        AttachmentType::from_filename("archive.zip"),
        AttachmentType::Zip
    );
    assert_eq!(
        AttachmentType::from_filename("unknown.xyz"),
        AttachmentType::Unknown
    );
}

#[test]
fn test_attachment_type_properties() {
    // Test image types
    assert!(AttachmentType::Jpeg.is_image());
    assert!(AttachmentType::Png.is_image());
    assert!(!AttachmentType::Pdf.is_image());

    // Test document types
    assert!(AttachmentType::Pdf.is_document());
    assert!(AttachmentType::Word.is_document());
    assert!(!AttachmentType::Jpeg.is_document());

    // Test archive types
    assert!(AttachmentType::Zip.is_archive());
    assert!(AttachmentType::Rar.is_archive());
    assert!(!AttachmentType::Pdf.is_archive());

    // Test previewable types
    assert!(AttachmentType::Text.is_previewable());
    assert!(AttachmentType::Jpeg.is_previewable());
    assert!(!AttachmentType::Zip.is_previewable());
}

#[test]
fn test_attachment_info_creation() {
    let stored = StoredAttachment {
        id: "test_id".to_string(),
        filename: "test.pdf".to_string(),
        content_type: "application/pdf".to_string(),
        size: 1024,
        content_id: None,
        is_inline: false,
        data: None,
        file_path: None,
    };

    let info = AttachmentInfo::from_stored(stored);

    assert_eq!(info.attachment_type, AttachmentType::Pdf);
    assert_eq!(info.display_name, "test.pdf");
    assert!(info.is_safe);

    let formatted_size = info.format_size();
    assert!(formatted_size.contains("KB"));
}

#[test]
fn test_unsafe_attachment_detection() {
    let dangerous_files = vec!["malware.exe", "virus.bat", "script.vbs", "trojan.scr"];

    for filename in dangerous_files {
        let stored = StoredAttachment {
            id: "test_id".to_string(),
            filename: filename.to_string(),
            content_type: "application/octet-stream".to_string(),
            size: 1024,
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);
        assert!(
            !info.is_safe,
            "File {} should be marked as unsafe",
            filename
        );
    }
}

#[test]
fn test_attachment_descriptions_and_icons() {
    let types = vec![
        AttachmentType::Pdf,
        AttachmentType::Jpeg,
        AttachmentType::Word,
        AttachmentType::Excel,
        AttachmentType::Zip,
        AttachmentType::Text,
        AttachmentType::Unknown,
    ];

    for attachment_type in types {
        // Each type should have a non-empty description
        assert!(
            !attachment_type.description().is_empty(),
            "{:?} should have a description",
            attachment_type
        );

        // Each type should have a non-empty icon
        assert!(
            !attachment_type.icon().is_empty(),
            "{:?} should have an icon",
            attachment_type
        );
    }
}

#[test]
fn test_gmail_specific_content_types() {
    // Gmail sometimes uses specific content type formats
    assert_eq!(
        AttachmentType::from_content_type(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        ),
        AttachmentType::Word
    );
    assert_eq!(
        AttachmentType::from_content_type(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        ),
        AttachmentType::Excel
    );
    assert_eq!(
        AttachmentType::from_content_type(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        ),
        AttachmentType::PowerPoint
    );
}

#[test]
fn test_size_formatting() {
    let test_cases = vec![
        (512, "512 B"),
        (1024, "1.0 KB"),
        (1536, "1.5 KB"),
        (1024 * 1024, "1.0 MB"),
        (1024 * 1024 * 1024, "1.0 GB"),
    ];

    for (size_bytes, expected_format) in test_cases {
        let stored = StoredAttachment {
            id: "test_id".to_string(),
            filename: "test.txt".to_string(),
            content_type: "text/plain".to_string(),
            size: size_bytes as u32,
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);
        let formatted = info.format_size();

        // Check that the format contains the expected size unit
        if expected_format.contains("B") && !expected_format.contains("KB") {
            assert!(
                formatted.contains("B"),
                "Size {} should format to bytes",
                size_bytes
            );
        } else if expected_format.contains("KB") {
            assert!(
                formatted.contains("KB"),
                "Size {} should format to KB",
                size_bytes
            );
        } else if expected_format.contains("MB") {
            assert!(
                formatted.contains("MB"),
                "Size {} should format to MB",
                size_bytes
            );
        } else if expected_format.contains("GB") {
            assert!(
                formatted.contains("GB"),
                "Size {} should format to GB",
                size_bytes
            );
        }
    }
}

#[test]
fn test_type_precedence() {
    // When both content type and filename provide type info, content type should take precedence
    let stored = StoredAttachment {
        id: "test_id".to_string(),
        filename: "document.txt".to_string(), // Suggests text
        content_type: "application/pdf".to_string(), // Suggests PDF
        size: 1024,
        content_id: None,
        is_inline: false,
        data: None,
        file_path: None,
    };

    let info = AttachmentInfo::from_stored(stored);

    // Should prefer content type over filename
    assert_eq!(info.attachment_type, AttachmentType::Pdf);
}

#[test]
fn test_empty_filename_handling() {
    let stored = StoredAttachment {
        id: "test_id".to_string(),
        filename: "".to_string(), // Empty filename
        content_type: "application/pdf".to_string(),
        size: 1024,
        content_id: None,
        is_inline: false,
        data: None,
        file_path: None,
    };

    let info = AttachmentInfo::from_stored(stored);

    // Should generate a display name based on type
    assert_eq!(info.display_name, "Attachment.pdf");
    assert_eq!(info.attachment_type, AttachmentType::Pdf);
}
