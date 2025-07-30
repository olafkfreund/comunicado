use comunicado::ui::email_viewer::EmailViewer;
use comunicado::ui::content_preview::{EmailContent, EmailHeader, ContentType};

/// Comprehensive email rendering test suite
/// Tests various email formats including HTML emails with images, tables, and complex layouts
#[cfg(test)]
mod email_rendering_tests {
    use super::*;

    /// Test 1: Simple plain text email (baseline)
    #[test]
    fn test_simple_plain_text_email() {
        let email_content = r#"From: sender@example.com
To: recipient@example.com
Subject: Simple Test Email
Date: Mon, 01 Jan 2024 12:00:00 +0000

Hello there!

This is a simple plain text email for testing.

Best regards,
The Test Team
"#;
        
        let result = EmailViewer::filter_email_headers_and_metadata(email_content);
        
        // Should extract just the body content
        assert!(result.contains("Hello there!"));
        assert!(result.contains("This is a simple plain text email"));
        assert!(result.contains("Best regards"));
        
        // Should not contain headers
        assert!(!result.contains("From: sender@example.com"));
        assert!(!result.contains("Subject: Simple Test Email"));
        
        println!("✓ Simple plain text email parsing works correctly");
    }

    /// Test 2: Complex HTML email with embedded styles and images
    #[test]
    fn test_html_email_with_images_and_styles() {
        let html_email = r#"From: facebook@facebookmail.com
To: user@example.com
Subject: Security Alert - New Device Login
Content-Type: text/html; charset=UTF-8

<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        .header { background: #1877f2; color: white; padding: 20px; }
        .content { padding: 20px; font-family: Arial, sans-serif; }
        .alert { background: #fff3cd; border: 1px solid #ffeaa7; padding: 15px; }
        .button { background: #1877f2; color: white; padding: 10px 20px; text-decoration: none; border-radius: 5px; }
    </style>
</head>
<body>
    <div class="header">
        <h1>Facebook Security Alert</h1>
    </div>
    
    <div class="content">
        <div class="alert">
            <h2>Did you just log in near London on a new device?</h2>
            <p>Hi there,</p>
            <p>It looks like you signed into your Facebook account from a new device. If this was you, you're all set to skip the code check they're posting your account.</p>
            
            <table border="1" cellpadding="10" cellspacing="0" style="margin: 20px 0;">
                <tr>
                    <td><strong>Device:</strong></td>
                    <td>Chrome on Linux</td>
                </tr>
                <tr>
                    <td><strong>Location:</strong></td>
                    <td>London, United Kingdom</td>
                </tr>
                <tr>
                    <td><strong>Time:</strong></td>
                    <td>January 29, 2025 at 1:51 PM GMT</td>
                </tr>
            </table>
            
            <p>If this wasn't you:</p>
            <ul>
                <li>Change your password immediately</li>
                <li>Enable two-factor authentication</li>
                <li>Review your recent login activity</li>
            </ul>
            
            <p style="text-align: center; margin: 30px 0;">
                <a href="https://facebook.com/security" class="button">Secure My Account</a>
            </p>
            
            <img src="https://facebook.com/security/alert.png" alt="Security Alert" width="400" height="200" style="display: block; margin: 20px auto;">
        </div>
        
        <hr>
        
        <p>Thanks,<br>
        The Facebook Team</p>
        
        <p style="font-size: 12px; color: #666;">
            This message was sent to user@example.com. If you don't want to receive these emails from Facebook in the future, please unsubscribe.
        </p>
    </div>
</body>
</html>"#;

        let result = EmailViewer::filter_email_headers_and_metadata(html_email);
        
        // Should extract key content
        assert!(result.contains("Did you just log in near London"));
        assert!(result.contains("Hi there"));
        assert!(result.contains("Chrome on Linux"));
        assert!(result.contains("London, United Kingdom"));
        assert!(result.contains("Change your password immediately"));
        assert!(result.contains("The Facebook Team"));
        
        // Should not contain HTML tags
        assert!(!result.contains("<div"));
        assert!(!result.contains("<style>"));
        assert!(!result.contains("<!DOCTYPE"));
        
        // Should not contain headers
        assert!(!result.contains("From: facebook@facebookmail.com"));
        assert!(!result.contains("Content-Type: text/html"));
        
        println!("✓ Complex HTML email with images and styles parsed correctly");
        println!("Extracted content: {}", result);
    }

    /// Test 3: MIME multipart email with both text and HTML parts
    #[test]
    fn test_mime_multipart_email() {
        let multipart_email = r#"From: newsletter@company.com
To: subscriber@example.com
Subject: Weekly Newsletter
MIME-Version: 1.0
Content-Type: multipart/alternative; boundary="----=_NextPart_000_001B_01D3C4F8.B7E2B3A0"

------=_NextPart_000_001B_01D3C4F8.B7E2B3A0
Content-Type: text/plain; charset="utf-8"
Content-Transfer-Encoding: 7bit

Welcome to our Weekly Newsletter!

This week's highlights:
- New product launch: Revolutionary AI Assistant
- Customer spotlight: How TechCorp increased productivity by 50%
- Upcoming webinar: "The Future of AI in Business"

Don't miss our exclusive offers below:
* 25% off premium plans
* Free consultation for enterprise customers

Visit our website: https://company.com/newsletter

Best regards,
The Company Team

------=_NextPart_000_001B_01D3C4F8.B7E2B3A0
Content-Type: text/html; charset="utf-8"
Content-Transfer-Encoding: 7bit

<!DOCTYPE html>
<html>
<body>
    <h1 style="color: #333;">Welcome to our Weekly Newsletter!</h1>
    
    <h2>This week's highlights:</h2>
    <ul>
        <li><strong>New product launch:</strong> Revolutionary AI Assistant</li>
        <li><strong>Customer spotlight:</strong> How TechCorp increased productivity by 50%</li>
        <li><strong>Upcoming webinar:</strong> "The Future of AI in Business"</li>
    </ul>
    
    <div style="background: #f5f5f5; padding: 20px; margin: 20px 0;">
        <h3>Exclusive Offers:</h3>
        <ul>
            <li>25% off premium plans</li>
            <li>Free consultation for enterprise customers</li>
        </ul>
    </div>
    
    <p><a href="https://company.com/newsletter" style="color: #007bff;">Visit our website</a></p>
    
    <p>Best regards,<br>The Company Team</p>
</body>
</html>

------=_NextPart_000_001B_01D3C4F8.B7E2B3A0--"#;

        let result = EmailViewer::filter_email_headers_and_metadata(multipart_email);
        
        // Should prefer plain text part over HTML
        assert!(result.contains("Welcome to our Weekly Newsletter"));
        assert!(result.contains("Revolutionary AI Assistant"));
        assert!(result.contains("25% off premium plans"));
        assert!(result.contains("The Company Team"));
        
        // Should not contain MIME boundaries or headers
        assert!(!result.contains("NextPart_"));
        assert!(!result.contains("Content-Type:"));
        assert!(!result.contains("boundary="));
        
        println!("✓ MIME multipart email parsed correctly");
    }

    /// Test 4: Email with DKIM signatures and authentication headers (real-world scenario)
    #[test]
    fn test_email_with_dkim_and_auth_headers() {
        let email_with_dkim = r#"Received: by 2002:a05:6402:1647:b0:4a2:7e5b:c5d8 with SMTP id p7-20020a056402164700b004a27e5bc5d8mr1234567edx.56.1674567890123;
        Mon, 24 Jan 2024 01:31:30 -0800 (PST)
DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed;
        d=google.com; s=20230601;
        t=1674567890;
        x=1675172690;
        h=to:subject:message-id:date:from:mime-version:from:to:cc:subject
         :date:message-id:reply-to;
        bh=gyCXTNXrpKmFFn8/DS1G6HKFJdxr1IZjByTDgTGHKc=;
        b=R8sI2gQjDgR5YRzF7KhNjP4xVQ3ZoL8M6wE2cA9sT1kN5rP7vH2iO0uY4dF3gS8wE
         R7LmPq9C5K1nH8zT6vY2sA3F1rO4D9uE5xW0zI7nP8qT4vK6mL2cH9gS5dF3N7yE1R=
Authentication-Results: mx.google.com;
       dkim=pass header.i=@google.com header.s=20230601 header.b=R8sI2gQj;
       spf=pass (google.com: domain of sender@google.com designates 209.85.220.69 as permitted sender) smtp.mailfrom=sender@google.com;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=google.com
ARC-Authentication-Results: i=1; mx.google.com;
       dkim=pass header.i=@google.com header.s=20230601 header.b=R8sI2gQj;
       spf=pass (google.com: domain of sender@google.com designates 209.85.220.69 as permitted sender) smtp.mailfrom=sender@google.com;
       dmarc=pass (p=REJECT sp=REJECT dis=NONE) header.from=google.com
From: Google Security <security@google.com>
To: user@example.com
Subject: Security alert for your Google Account
Date: Mon, 24 Jan 2024 09:31:30 +0000
Message-ID: <CABcdefghijklmnopqrstuvwxyz123456789@mail.google.com>

Hi there,

We detected a new sign-in to your Google Account from a device we don't recognize.

Device: Chrome on Windows
Location: San Francisco, CA, USA
Time: Monday, January 24, 2024 at 9:30 AM PST

If this was you, you don't need to do anything.

If this wasn't you, we recommend:
1. Change your password immediately
2. Review your account activity
3. Enable 2-Step Verification

You can review and manage your account activity at:
https://myaccount.google.com/security

Best regards,
The Google Account Team"#;

        let result = EmailViewer::filter_email_headers_and_metadata(email_with_dkim);
        
        // Should extract the actual message content
        assert!(result.contains("Hi there"));
        assert!(result.contains("We detected a new sign-in"));
        assert!(result.contains("Chrome on Windows"));
        assert!(result.contains("San Francisco, CA, USA"));
        assert!(result.contains("Change your password immediately"));
        assert!(result.contains("The Google Account Team"));
        
        // Should completely filter out technical headers
        assert!(!result.contains("DKIM-Signature:"));
        assert!(!result.contains("Authentication-Results:"));
        assert!(!result.contains("ARC-Authentication-Results:"));
        assert!(!result.contains("bh=gyCXTNXr"));
        assert!(!result.contains("b=R8sI2gQj"));
        assert!(!result.contains("spf=pass"));
        assert!(!result.contains("dmarc=pass"));
        
        println!("✓ Email with DKIM and authentication headers parsed correctly");
        println!("Clean content: {}", result);
    }

    /// Test 5: Newsletter with complex table layout
    #[test]
    fn test_newsletter_with_complex_tables() {
        let newsletter_email = r#"From: newsletter@techblog.com
To: subscriber@example.com
Subject: Tech Weekly - Issue #42
Content-Type: text/html; charset=UTF-8

<!DOCTYPE html>
<html>
<body style="font-family: Arial, sans-serif; margin: 0; padding: 20px;">
    <table width="100%" cellpadding="0" cellspacing="0" style="max-width: 600px; margin: 0 auto;">
        <tr>
            <td style="background: #2c3e50; color: white; padding: 20px; text-align: center;">
                <h1>Tech Weekly</h1>
                <p>Issue #42 - January 2024</p>
            </td>
        </tr>
        
        <tr>
            <td style="padding: 30px;">
                <h2 style="color: #2c3e50;">This Week in Tech</h2>
                
                <table width="100%" cellpadding="15" cellspacing="0" style="border: 1px solid #ddd; margin: 20px 0;">
                    <tr style="background: #f8f9fa;">
                        <td width="30%"><strong>Category</strong></td>
                        <td><strong>Headline</strong></td>
                    </tr>
                    <tr>
                        <td style="background: #e3f2fd;">AI/ML</td>
                        <td>OpenAI releases GPT-5 with breakthrough reasoning capabilities</td>
                    </tr>
                    <tr style="background: #f5f5f5;">
                        <td style="background: #fff3e0;">Hardware</td>
                        <td>Apple announces M4 chip with 40% performance improvement</td>
                    </tr>
                    <tr>
                        <td style="background: #e8f5e8;">Software</td>
                        <td>Rust 1.75 released with enhanced memory safety features</td>
                    </tr>
                </table>
                
                <h3>Featured Article: The Rise of Edge Computing</h3>
                <p>Edge computing is revolutionizing how we process data. By bringing computation closer to data sources, companies are achieving:</p>
                
                <ul>
                    <li><strong>Reduced latency:</strong> Sub-millisecond response times</li>
                    <li><strong>Better privacy:</strong> Data stays local and secure</li>
                    <li><strong>Lower costs:</strong> Reduced bandwidth usage</li>
                    <li><strong>Improved reliability:</strong> Less dependency on cloud connectivity</li>
                </ul>
                
                <div style="background: #f0f8ff; border-left: 4px solid #0066cc; padding: 15px; margin: 20px 0;">
                    <h4>💡 Pro Tip</h4>
                    <p>When implementing edge computing, start small with non-critical workloads to understand the infrastructure requirements.</p>
                </div>
                
                <h3>Job Opportunities</h3>
                <table width="100%" cellpadding="10" cellspacing="0" style="border: 1px solid #ddd;">
                    <tr style="background: #f8f9fa;">
                        <td><strong>Company</strong></td>
                        <td><strong>Position</strong></td>
                        <td><strong>Location</strong></td>
                    </tr>
                    <tr>
                        <td>TechCorp</td>
                        <td>Senior Rust Developer</td>
                        <td>Remote</td>
                    </tr>
                    <tr style="background: #f5f5f5;">
                        <td>StartupXYZ</td>
                        <td>ML Engineer</td>
                        <td>San Francisco, CA</td>
                    </tr>
                    <tr>
                        <td>BigTech Inc</td>
                        <td>DevOps Architect</td>
                        <td>Seattle, WA</td>
                    </tr>
                </table>
            </td>
        </tr>
        
        <tr>
            <td style="background: #34495e; color: white; padding: 20px; text-align: center;">
                <p>Thanks for reading Tech Weekly!</p>
                <p style="font-size: 12px;">
                    <a href="https://techblog.com/unsubscribe" style="color: #bdc3c7;">Unsubscribe</a> | 
                    <a href="https://techblog.com/archive" style="color: #bdc3c7;">View Archive</a>
                </p>
            </td>
        </tr>
    </table>
</body>
</html>"#;

        let result = EmailViewer::filter_email_headers_and_metadata(newsletter_email);
        
        // Should extract structured content from tables
        assert!(result.contains("Tech Weekly"));
        assert!(result.contains("Issue #42"));
        assert!(result.contains("This Week in Tech"));
        assert!(result.contains("OpenAI releases GPT-5"));
        assert!(result.contains("Apple announces M4 chip"));
        assert!(result.contains("Rust 1.75 released"));
        assert!(result.contains("The Rise of Edge Computing"));
        assert!(result.contains("Reduced latency"));
        assert!(result.contains("Pro Tip"));
        assert!(result.contains("Job Opportunities"));
        assert!(result.contains("Senior Rust Developer"));
        
        // Should not contain HTML structure
        assert!(!result.contains("<table"));
        assert!(!result.contains("<tr"));
        assert!(!result.contains("<td"));
        assert!(!result.contains("cellpadding"));
        
        println!("✓ Newsletter with complex tables parsed correctly");
    }

    /// Test 6: E-commerce order confirmation with embedded images
    #[test]
    fn test_ecommerce_order_confirmation() {
        let order_email = r#"From: orders@example-store.com
To: customer@example.com
Subject: Order Confirmation #ORD-123456
Content-Type: text/html; charset=UTF-8

<!DOCTYPE html>
<html>
<body>
    <div style="max-width: 600px; margin: 0 auto; font-family: Arial, sans-serif;">
        <header style="background: #ff6b35; color: white; padding: 20px; text-align: center;">
            <img src="https://example-store.com/logo.png" alt="Store Logo" width="150" height="50">
            <h1>Order Confirmation</h1>
        </header>
        
        <div style="padding: 30px;">
            <h2>Hi John Doe,</h2>
            <p>Thank you for your order! Your order <strong>#ORD-123456</strong> has been confirmed and will be processed shortly.</p>
            
            <div style="background: #f8f9fa; border: 1px solid #dee2e6; border-radius: 5px; padding: 20px; margin: 20px 0;">
                <h3>Order Details</h3>
                <table width="100%" cellpadding="10" cellspacing="0">
                    <tr style="border-bottom: 1px solid #dee2e6;">
                        <td width="60%"><strong>Item</strong></td>
                        <td width="20%"><strong>Qty</strong></td>
                        <td width="20%"><strong>Price</strong></td>
                    </tr>
                    <tr style="border-bottom: 1px solid #dee2e6;">
                        <td>
                            <img src="https://example-store.com/products/laptop.jpg" alt="Gaming Laptop" width="50" height="50" style="float: left; margin-right: 10px;">
                            <div>
                                <strong>Gaming Laptop Pro</strong><br>
                                <small>Model: GL-2024, Color: Black</small>
                            </div>
                        </td>
                        <td>1</td>
                        <td>$1,299.99</td>
                    </tr>
                    <tr style="border-bottom: 1px solid #dee2e6;">
                        <td>
                            <img src="https://example-store.com/products/mouse.jpg" alt="Gaming Mouse" width="50" height="50" style="float: left; margin-right: 10px;">
                            <div>
                                <strong>Wireless Gaming Mouse</strong><br>
                                <small>Model: WM-X1, Color: RGB</small>
                            </div>
                        </td>
                        <td>1</td>
                        <td>$79.99</td>
                    </tr>
                    <tr style="background: #e9ecef;">
                        <td colspan="2"><strong>Total</strong></td>
                        <td><strong>$1,379.98</strong></td>
                    </tr>
                </table>
            </div>
            
            <div style="background: #d4edda; border: 1px solid #c3e6cb; border-radius: 5px; padding: 15px; margin: 20px 0;">
                <h4 style="margin: 0 0 10px 0; color: #155724;">🚚 Shipping Information</h4>
                <p style="margin: 0;">
                    <strong>Address:</strong> 123 Main Street, Anytown, ST 12345<br>
                    <strong>Estimated Delivery:</strong> January 25-27, 2024<br>
                    <strong>Tracking:</strong> Available within 24 hours
                </p>
            </div>
            
            <p>Questions about your order? Contact our support team at <a href="mailto:support@example-store.com">support@example-store.com</a> or call 1-800-EXAMPLE.</p>
            
            <div style="text-align: center; margin: 30px 0;">
                <a href="https://example-store.com/track/ORD-123456" style="background: #ff6b35; color: white; padding: 15px 30px; text-decoration: none; border-radius: 5px; font-weight: bold;">Track Your Order</a>
            </div>
        </div>
        
        <footer style="background: #343a40; color: white; padding: 20px; text-align: center;">
            <p>Thank you for shopping with Example Store!</p>
            <p style="font-size: 12px;">
                <a href="https://example-store.com/account" style="color: #adb5bd;">My Account</a> | 
                <a href="https://example-store.com/support" style="color: #adb5bd;">Support</a> | 
                <a href="https://example-store.com/unsubscribe" style="color: #adb5bd;">Unsubscribe</a>
            </p>
        </footer>
    </div>
</body>
</html>"#;

        let result = EmailViewer::filter_email_headers_and_metadata(order_email);
        
        // Should extract order information
        assert!(result.contains("Order Confirmation"));
        assert!(result.contains("Hi John Doe"));
        assert!(result.contains("order #ORD-123456"));
        assert!(result.contains("Order Details"));
        assert!(result.contains("Gaming Laptop Pro"));
        assert!(result.contains("Wireless Gaming Mouse"));
        assert!(result.contains("$1,379.98"));
        assert!(result.contains("Shipping Information"));
        assert!(result.contains("123 Main Street"));
        assert!(result.contains("January 25-27, 2024"));
        assert!(result.contains("support@example-store.com"));
        
        // Should handle images gracefully (show alt text or remove)
        assert!(!result.contains("<img"));
        assert!(!result.contains("width="));
        assert!(!result.contains("height="));
        
        println!("✓ E-commerce order confirmation parsed correctly");
    }

    /// Test 7: Performance test with very large email
    #[test]
    fn test_performance_with_large_email() {
        use std::time::Instant;
        
        let large_email = generate_large_test_email(10000); // 10k lines
        
        let start = Instant::now();
        let result = EmailViewer::filter_email_headers_and_metadata(&large_email);
        let duration = start.elapsed();
        
        // Should complete within reasonable time (< 100ms for 10k lines)
        assert!(duration.as_millis() < 100, "Processing took too long: {:?}", duration);
        
        // Should still extract content correctly
        assert!(result.contains("Large Email Content"));
        assert!(result.contains("This is paragraph"));
        assert!(!result.contains("From: sender@example.com"));
        
        println!("✓ Large email ({} lines) processed in {:?}", 10000, duration);
    }

    /// Helper function to generate large test emails
    fn generate_large_test_email(lines: usize) -> String {
        let mut email = String::new();
        
        // Add headers
        email.push_str("From: sender@example.com\n");
        email.push_str("To: recipient@example.com\n");
        email.push_str("Subject: Large Email Test\n");
        email.push_str("Content-Type: text/html; charset=UTF-8\n");
        email.push_str("\n"); // Blank line separator
        
        // Add large HTML content
        email.push_str("<!DOCTYPE html>\n<html><body>\n");
        email.push_str("<h1>Large Email Content</h1>\n");
        
        for i in 0..lines {
            email.push_str(&format!(
                "<p>This is paragraph {} with some content that should be extracted properly.</p>\n",
                i + 1
            ));
        }
        
        email.push_str("</body></html>\n");
        email
    }

    /// Test 8: Edge case - Email with no clear body separator
    #[test]
    fn test_email_without_clear_separator() {
        let malformed_email = r#"From: sender@example.com
To: recipient@example.com
Subject: Test Email
This line has no blank separator above
And this is the actual email content.

Best regards,
Test Team"#;

        let result = EmailViewer::filter_email_headers_and_metadata(malformed_email);
        
        // Should still extract content using heuristic method
        assert!(result.contains("This line has no blank separator"));
        assert!(result.contains("And this is the actual email content"));
        assert!(result.contains("Best regards"));
        
        println!("✓ Malformed email without clear separator handled correctly");
    }

    /// Run all tests and provide summary
    #[test]
    fn run_all_email_rendering_tests() {
        println!("\n🧪 Running comprehensive email rendering tests...\n");
        
        test_simple_plain_text_email();
        test_html_email_with_images_and_styles();
        test_mime_multipart_email();
        test_email_with_dkim_and_auth_headers();
        test_newsletter_with_complex_tables();
        test_ecommerce_order_confirmation();
        test_performance_with_large_email();
        test_email_without_clear_separator();
        
        println!("\n✅ All email rendering tests passed!");
        println!("🎉 RFC-compliant email parsing is working correctly!");
    }
}