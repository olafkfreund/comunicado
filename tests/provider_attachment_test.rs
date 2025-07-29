//! Provider-specific attachment handling tests
//! Tests attachment scenarios specific to various email providers

use comunicado::email::{AttachmentInfo, AttachmentType, StoredAttachment};
use comunicado::mime::decode_mime_header;

#[test]
fn test_gmail_attachment_scenarios() {
    // Gmail often uses very specific MIME types and encoded filenames
    let test_cases = vec![
        (
            "=?UTF-8?B?UmVwb3J0X1ExXzIwMjQuZG9jeA==?=", // "Report_Q1_2024.docx" in base64
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            AttachmentType::Word,
            "Report_Q1_2024.docx",
        ),
        ("image.jpg", "image/jpeg", AttachmentType::Jpeg, "image.jpg"),
        (
            "=?ISO-8859-1?Q?caf=E9_menu.pdf?=", // "café_menu.pdf"
            "application/pdf",
            AttachmentType::Pdf,
            "café_menu.pdf",
        ),
    ];

    for (filename, content_type, expected_type, expected_display_name) in test_cases {
        let stored = StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            size: 1024 * 100, // 100KB
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);

        assert_eq!(
            info.attachment_type, expected_type,
            "Gmail attachment type detection failed for {}",
            filename
        );
        assert_eq!(
            info.display_name, expected_display_name,
            "Gmail filename decoding failed for {}",
            filename
        );
        assert!(info.is_safe, "Gmail attachment should be marked as safe");
    }
}

#[test]
fn test_outlook_attachment_scenarios() {
    // Outlook/Exchange often includes additional parameters in content types
    let test_cases = vec![
        (
            "presentation.pptx",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation; name=\"presentation.pptx\"",
            AttachmentType::PowerPoint
        ),
        (
            "spreadsheet.xlsx", 
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet; charset=UTF-8",
            AttachmentType::Excel
        ),
        (
            "document.pdf",
            "application/pdf; name=document.pdf; filename=document.pdf",
            AttachmentType::Pdf
        ),
    ];

    for (filename, content_type, expected_type) in test_cases {
        let stored = StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            size: 1024 * 150, // 150KB
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);

        assert_eq!(
            info.attachment_type, expected_type,
            "Outlook attachment type detection failed for {}",
            filename
        );
        assert_eq!(
            info.display_name, filename,
            "Outlook filename should remain unchanged"
        );
    }
}

#[test]
fn test_yahoo_attachment_scenarios() {
    // Yahoo sometimes uses generic content types for various files
    let test_cases = vec![
        ("photo.jpg", "image/jpeg", AttachmentType::Jpeg),
        (
            "compressed.zip",
            "application/x-zip-compressed", // Yahoo-specific variant
            AttachmentType::Zip,
        ),
        ("archive.tar.gz", "application/gzip", AttachmentType::Tar),
    ];

    for (filename, content_type, expected_type) in test_cases {
        let stored = StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            size: 1024 * 75, // 75KB
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);

        assert_eq!(
            info.attachment_type, expected_type,
            "Yahoo attachment type detection failed for {}",
            filename
        );
    }
}

#[test]
fn test_protonmail_attachment_scenarios() {
    // ProtonMail emphasizes security and privacy
    let test_cases = vec![
        (
            "secure_document.pdf",
            "application/pdf",
            AttachmentType::Pdf,
            true, // Should be safe
        ),
        (
            "encrypted_notes.txt",
            "text/plain; charset=utf-8",
            AttachmentType::Text,
            true,
        ),
        ("backup.zip", "application/zip", AttachmentType::Zip, true),
    ];

    for (filename, content_type, expected_type, expected_safe) in test_cases {
        let stored = StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            size: 1024 * 200, // 200KB
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);

        assert_eq!(
            info.attachment_type, expected_type,
            "ProtonMail attachment type detection failed for {}",
            filename
        );
        assert_eq!(
            info.is_safe, expected_safe,
            "ProtonMail safety classification failed for {}",
            filename
        );
    }
}

#[test]
fn test_international_filename_handling() {
    // Test various international character encodings commonly seen in email
    let test_cases = vec![
        // Japanese
        (
            "=?UTF-8?B?44OG44K544OI44OV44Kh44Kk44OrLnR4dA==?=",
            "テストファイル.txt",
        ),
        // German with umlauts
        (
            "=?UTF-8?Q?M=C3=BCnchen_Pr=C3=A4sentation.pptx?=",
            "München_Präsentation.pptx",
        ),
        // French with accents
        ("=?ISO-8859-1?Q?r=E9sum=E9.pdf?=", "résumé.pdf"),
        // Chinese
        ("=?UTF-8?B?5paH5qGjLnR4dA==?=", "文档.txt"),
        // Russian
        (
            "=?UTF-8?B?0LTQvtC60YPQvNC10L3RgiDRgNGD0YHRgdC60LjQuS5wZGY=?=",
            "документ русский.pdf",
        ),
        // Mixed ASCII and international
        (
            "=?UTF-8?Q?Report_2024_=E2=80=93_M=C3=BCnchen.xlsx?=",
            "Report_2024_–_München.xlsx",
        ),
    ];

    for (encoded, expected_decoded) in test_cases {
        let stored = StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: encoded.to_string(),
            content_type: "application/octet-stream".to_string(),
            size: 1024,
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);

        assert_eq!(
            info.display_name, expected_decoded,
            "International filename decoding failed for: {}",
            encoded
        );
    }
}

#[test]
fn test_large_attachment_sizes() {
    // Test various large attachment sizes
    let test_cases = vec![
        (1024 * 1024, "1.0 MB"),                 // 1 MB
        (5 * 1024 * 1024, "5.0 MB"),             // 5 MB
        (25 * 1024 * 1024, "25.0 MB"),           // 25 MB
        (100 * 1024 * 1024, "100.0 MB"),         // 100 MB
        ((1024 * 1024 * 1024) as u32, "1.0 GB"), // 1 GB
    ];

    for (size_bytes, expected_format_contains) in test_cases {
        let stored = StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: "large_file.zip".to_string(),
            content_type: "application/zip".to_string(),
            size: size_bytes,
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);
        let formatted_size = info.format_size();

        // Check that the formatted size contains the expected unit
        let contains_expected = if expected_format_contains.contains("MB") {
            formatted_size.contains("MB")
        } else if expected_format_contains.contains("GB") {
            formatted_size.contains("GB")
        } else {
            false
        };

        assert!(
            contains_expected,
            "Size formatting failed for {} bytes, got: {}",
            size_bytes, formatted_size
        );
    }
}

#[test]
fn test_inline_attachment_handling() {
    // Test inline vs regular attachments (common in HTML emails)
    let inline_attachment = StoredAttachment {
        id: uuid::Uuid::new_v4().to_string(),
        filename: "inline_image.png".to_string(),
        content_type: "image/png".to_string(),
        size: 1024 * 50,
        content_id: Some("image001@domain.com".to_string()),
        is_inline: true,
        data: None,
        file_path: None,
    };

    let regular_attachment = StoredAttachment {
        id: uuid::Uuid::new_v4().to_string(),
        filename: "attachment.png".to_string(),
        content_type: "image/png".to_string(),
        size: 1024 * 50,
        content_id: None,
        is_inline: false,
        data: None,
        file_path: None,
    };

    let inline_info = AttachmentInfo::from_stored(inline_attachment);
    let regular_info = AttachmentInfo::from_stored(regular_attachment);

    // Both should be detected as PNG images
    assert_eq!(inline_info.attachment_type, AttachmentType::Png);
    assert_eq!(regular_info.attachment_type, AttachmentType::Png);

    // Both should be previewable
    assert!(inline_info.attachment_type.is_previewable());
    assert!(regular_info.attachment_type.is_previewable());

    // Both should be safe
    assert!(inline_info.is_safe);
    assert!(regular_info.is_safe);
}

#[test]
fn test_content_type_edge_cases() {
    // Test various edge cases in content type handling
    let test_cases = vec![
        // Case sensitivity
        ("document.PDF", "APPLICATION/PDF", AttachmentType::Pdf),
        ("IMAGE.JPG", "IMAGE/JPEG", AttachmentType::Jpeg),
        // Content type with extra whitespace
        ("file.txt", " text/plain ", AttachmentType::Text),
        ("file.zip", "application/zip  ", AttachmentType::Zip),
        // Unknown content type, rely on filename
        (
            "document.pdf",
            "application/octet-stream",
            AttachmentType::Pdf,
        ),
        ("photo.jpg", "application/binary", AttachmentType::Jpeg),
        // Both unknown - should be unknown
        (
            "unknown_file",
            "application/octet-stream",
            AttachmentType::Unknown,
        ),
        ("", "unknown/type", AttachmentType::Unknown),
    ];

    for (filename, content_type, expected_type) in test_cases {
        let stored = StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            size: 1024,
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);

        assert_eq!(
            info.attachment_type, expected_type,
            "Edge case handling failed for filename: '{}', content_type: '{}'",
            filename, content_type
        );
    }
}

#[test]
fn test_mime_header_edge_cases() {
    // Test MIME header decoding edge cases
    let test_cases = vec![
        // Multiple encoded words
        (
            "=?UTF-8?B?VGVzdA==?= =?UTF-8?B?RmlsZQ==?=.txt",
            "TestFile.txt",
        ),
        // Mixed encoded and plain text
        ("Report_=?UTF-8?Q?2024?=_final.pdf", "Report_2024_final.pdf"),
        // Different encodings in one filename
        (
            "=?UTF-8?B?VGVzdA==?==?ISO-8859-1?Q?_file?=.txt",
            "Test_file.txt",
        ),
        // Invalid encoding (should pass through)
        ("=?INVALID?B?invalid?=.txt", "=?INVALID?B?invalid?=.txt"),
        // Malformed encoded word (should pass through)
        ("=?UTF-8?B?incomplete", "=?UTF-8?B?incomplete"),
        // Empty encoded word
        ("=?UTF-8?B??=.txt", ".txt"),
        // Very long filename
        (
            "=?UTF-8?B?VmVyeUxvbmdGaWxlbmFtZVRoYXRFeGNlZWRzTm9ybWFsTGVuZ3RoTGltaXRz?=.pdf",
            "VeryLongFilenameThatExceedsNormalLengthLimits.pdf",
        ),
    ];

    for (encoded, expected) in test_cases {
        let decoded = decode_mime_header(encoded);
        assert_eq!(decoded, expected, "MIME decoding failed for: {}", encoded);
    }
}

#[test]
fn test_security_scenarios_by_provider() {
    // Test how different providers might handle potentially dangerous files
    let dangerous_files = vec![
        // Scripts that might be dangerous
        ("script.js", "application/javascript"),
        ("macro.vbs", "text/vbscript"),
        ("automation.ps1", "text/plain"), // PowerShell disguised as text
        // Executables with misleading names
        ("document.pdf.exe", "application/octet-stream"),
        ("photo.jpg.scr", "application/octet-stream"),
        // Archives that might contain malware
        ("update.zip", "application/zip"),
        ("package.tar.gz", "application/gzip"),
    ];

    for (filename, content_type) in dangerous_files {
        let stored = StoredAttachment {
            id: uuid::Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            content_type: content_type.to_string(),
            size: 1024 * 10,
            content_id: None,
            is_inline: false,
            data: None,
            file_path: None,
        };

        let info = AttachmentInfo::from_stored(stored);

        // Files with dangerous extensions should be marked unsafe
        let has_dangerous_extension = filename.ends_with(".exe")
            || filename.ends_with(".scr")
            || filename.ends_with(".vbs")
            || filename.ends_with(".js");

        if has_dangerous_extension {
            assert!(
                !info.is_safe,
                "File {} should be marked as unsafe due to dangerous extension",
                filename
            );
        }

        // All files should still have valid type detection
        assert!(
            matches!(
                info.attachment_type,
                AttachmentType::Unknown
                    | AttachmentType::Text
                    | AttachmentType::Zip
                    | AttachmentType::Tar
            ),
            "Dangerous file {} should still have valid type detection",
            filename
        );
    }
}
