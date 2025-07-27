use base64::prelude::*;

/// Decode MIME-encoded header text (RFC 2047)
/// 
/// Handles encoded words in the format: =?charset?encoding?encoded-text?=
/// where encoding is either 'B' (base64) or 'Q' (quoted-printable)
/// 
/// Examples:
/// - =?UTF-8?B?SGVsbG8gV29ybGQ=?= -> "Hello World"
/// - =?ISO-8859-1?Q?Hello_World?= -> "Hello World"
/// - Regular text -> unchanged
pub fn decode_mime_header(header: &str) -> String {
    let mut result = String::new();
    let mut remaining = header;
    
    while !remaining.is_empty() {
        if let Some(start) = remaining.find("=?") {
            // Add any text before the encoded word
            result.push_str(&remaining[..start]);
            remaining = &remaining[start..];
            
            // Find the end of the encoded word
            if let Some(end) = remaining.find("?=") {
                let encoded_word = &remaining[2..end]; // Skip "=?"
                remaining = &remaining[end + 2..]; // Skip "?="
                
                // Parse the encoded word: charset?encoding?text
                let parts: Vec<&str> = encoded_word.split('?').collect();
                if parts.len() == 3 {
                    let charset = parts[0];
                    let encoding = parts[1].to_uppercase();
                    let encoded_text = parts[2];
                    
                    match decode_encoded_word(charset, &encoding, encoded_text) {
                        Ok(decoded) => result.push_str(&decoded),
                        Err(_) => {
                            // If decoding fails, include the original encoded word
                            result.push_str("=?");
                            result.push_str(encoded_word);
                            result.push_str("?=");
                        }
                    }
                } else {
                    // Invalid format, include as-is
                    result.push_str("=?");
                    result.push_str(encoded_word);
                    result.push_str("?=");
                }
            } else {
                // No closing ?=, include the rest as-is
                result.push_str(remaining);
                break;
            }
        } else {
            // No more encoded words, add the rest
            result.push_str(remaining);
            break;
        }
    }
    
    result
}

/// Decode a single encoded word
fn decode_encoded_word(charset: &str, encoding: &str, encoded_text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = match encoding {
        "B" => decode_base64(encoded_text)?,
        "Q" => decode_quoted_printable(encoded_text)?,
        _ => return Err(format!("Unsupported encoding: {}", encoding).into()),
    };
    
    // Convert bytes to string based on charset
    let decoded = match charset.to_uppercase().as_str() {
        "UTF-8" => String::from_utf8(bytes)?,
        "ISO-8859-1" | "LATIN-1" => {
            // Convert Latin-1 to UTF-8
            String::from_utf8(bytes.iter().map(|&b| b as u32).map(char::from_u32).collect::<Option<Vec<_>>>().ok_or("Invalid Latin-1")?.into_iter().collect::<String>().into_bytes())?
        },
        "ASCII" | "US-ASCII" => {
            // ASCII is a subset of UTF-8
            String::from_utf8(bytes)?
        },
        _ => {
            // For unsupported charsets, try UTF-8 and fall back to lossy conversion
            match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => String::from_utf8_lossy(&e.into_bytes()).to_string(),
            }
        }
    };
    
    Ok(decoded)
}

/// Decode Base64 encoded text
fn decode_base64(encoded: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(BASE64_STANDARD.decode(encoded.as_bytes())?)
}

/// Decode Quoted-Printable encoded text
fn decode_quoted_printable(encoded: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    let mut chars = encoded.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '=' => {
                // Get the next two characters for hex decoding
                if let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
                    if let Ok(byte_val) = u8::from_str_radix(&format!("{}{}", c1, c2), 16) {
                        result.push(byte_val);
                    } else {
                        // Invalid hex, include as-is
                        result.push(b'=');
                        result.push(c1 as u8);
                        result.push(c2 as u8);
                    }
                } else {
                    // Not enough characters, include as-is
                    result.push(b'=');
                    if let Some(c) = chars.peek() {
                        result.push(*c as u8);
                        chars.next();
                    }
                }
            },
            '_' => {
                // Underscore represents space in quoted-printable
                result.push(b' ');
            },
            _ => {
                // Regular character
                result.push(ch as u8);
            }
        }
    }
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_base64_utf8() {
        let input = "=?UTF-8?B?SGVsbG8gV29ybGQ=?=";
        let expected = "Hello World";
        assert_eq!(decode_mime_header(input), expected);
    }

    #[test]
    fn test_decode_quoted_printable() {
        let input = "=?ISO-8859-1?Q?Hello_World?=";
        let expected = "Hello World";
        assert_eq!(decode_mime_header(input), expected);
    }

    #[test]
    fn test_mixed_text() {
        let input = "Regular text =?UTF-8?B?SGVsbG8=?= more text";
        let expected = "Regular text Hello more text";
        assert_eq!(decode_mime_header(input), expected);
    }

    #[test]
    fn test_no_encoding() {
        let input = "Regular text without encoding";
        assert_eq!(decode_mime_header(input), input);
    }

    #[test]
    fn test_multiple_encoded_words() {
        let input = "=?UTF-8?B?SGVsbG8=?= =?UTF-8?B?V29ybGQ=?=";
        let expected = "Hello World";
        assert_eq!(decode_mime_header(input), expected);
    }
}