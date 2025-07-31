#!/usr/bin/env -S cargo +nightly -Zscript
//! Simple manual OAuth2 setup script for Gmail
//! 
//! Usage: ./manual_oauth_setup.rs

use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Manual Gmail OAuth2 Setup for Comunicado");
    println!("============================================\n");
    
    // Read client credentials from file
    let client_secret_path = "/home/olafkfreund/client_secret_771552156772-oamkti00mbglr2o0k7ejo4spgldgeu0i.apps.googleusercontent.com.json";
    let client_data = std::fs::read_to_string(client_secret_path)?;
    let client_json: serde_json::Value = serde_json::from_str(&client_data)?;
    
    let client_id = client_json["installed"]["client_id"]
        .as_str()
        .ok_or("Missing client_id")?;
    let client_secret = client_json["installed"]["client_secret"]
        .as_str()
        .ok_or("Missing client_secret")?;
    
    println!("✅ Found Google OAuth2 credentials");
    println!("   Client ID: {}...", &client_id[..20]);
    println!();
    
    // Build OAuth2 authorization URL
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/auth?client_id={}&redirect_uri={}&scope={}&response_type=code&access_type=offline&prompt=consent",
        client_id,
        "http://localhost:8080/oauth/callback",
        "https://mail.google.com/ https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile"
    );
    
    println!("🌐 Please visit this URL to authorize access:");
    println!("{}", auth_url);
    println!();
    
    print!("📋 After authorization, paste the authorization code here: ");
    io::stdout().flush()?;
    
    let mut auth_code = String::new();
    io::stdin().read_line(&mut auth_code)?;
    let auth_code = auth_code.trim();
    
    if auth_code.is_empty() {
        println!("❌ No authorization code provided");
        return Ok(());
    }
    
    println!("✅ Got authorization code: {}...", &auth_code[..10.min(auth_code.len())]);
    
    // Exchange code for tokens
    let token_request = format!(
        "client_id={}&client_secret={}&code={}&grant_type=authorization_code&redirect_uri={}",
        client_id,
        client_secret,
        auth_code,
        "http://localhost:8080/oauth/callback"
    );
    
    let client = reqwest::Client::new();
    let token_response = client
        .post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(token_request)
        .send()
        .await?;
    
    let token_data: serde_json::Value = token_response.json().await?;
    
    if let Some(error) = token_data.get("error") {
        println!("❌ Token exchange failed: {}", error);
        return Ok(());
    }
    
    let access_token = token_data["access_token"]
        .as_str()
        .ok_or("Missing access_token")?;
    let refresh_token = token_data.get("refresh_token")
        .and_then(|t| t.as_str());
    let expires_in = token_data["expires_in"]
        .as_u64()
        .unwrap_or(3600);
    
    println!("✅ Got OAuth2 tokens successfully!");
    println!("   Access token: {}...", &access_token[..20]);
    if let Some(refresh) = refresh_token {
        println!("   Refresh token: {}...", &refresh[..20]);
    }
    println!();
    
    // Calculate expiration time
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);
    
    // Create account config
    let account_config = serde_json::json!({
        "account_id": "gmail_olaf_loken_gmail_com",
        "display_name": "Olaf Krasicki Freund",
        "email_address": "olaf.loken@gmail.com",
        "provider": "gmail",
        "imap_server": "imap.gmail.com",
        "imap_port": 993,
        "smtp_server": "smtp.gmail.com",
        "smtp_port": 587,
        "token_expires_at": expires_at.to_rfc3339(),
        "scopes": [
            "https://mail.google.com/",
            "https://www.googleapis.com/auth/userinfo.email",
            "https://www.googleapis.com/auth/userinfo.profile"
        ]
    });
    
    // Write account config
    let config_path = "/home/olafkfreund/.config/comunicado/gmail_olaf_loken_gmail_com.json";
    std::fs::write(config_path, serde_json::to_string_pretty(&account_config)?)?;
    println!("✅ Account config written to: {}", config_path);
    
    // Write tokens to files (base64 encoded)
    let config_dir = "/home/olafkfreund/.config/comunicado";
    
    // Access token
    let access_token_encoded = base64::prelude::BASE64_STANDARD.encode(access_token);
    let access_token_path = format!("{}/gmail_olaf_loken_gmail_com.access.token", config_dir);
    std::fs::write(&access_token_path, access_token_encoded)?;
    println!("✅ Access token written to: {}", access_token_path);
    
    // Refresh token (if available)
    if let Some(refresh) = refresh_token {
        let refresh_token_encoded = base64::prelude::BASE64_STANDARD.encode(refresh);
        let refresh_token_path = format!("{}/gmail_olaf_loken_gmail_com.refresh.token", config_dir);
        std::fs::write(&refresh_token_path, refresh_token_encoded)?;
        println!("✅ Refresh token written to: {}", refresh_token_path);
    }
    
    // Store OAuth2 credentials
    let client_id_encoded = base64::prelude::BASE64_STANDARD.encode(client_id);
    let client_secret_encoded = base64::prelude::BASE64_STANDARD.encode(client_secret);
    let client_id_path = format!("{}/gmail_olaf_loken_gmail_com.client_id.cred", config_dir);
    let client_secret_path = format!("{}/gmail_olaf_loken_gmail_com.client_secret.cred", config_dir);
    
    std::fs::write(&client_id_path, client_id_encoded)?;
    std::fs::write(&client_secret_path, client_secret_encoded)?;
    println!("✅ OAuth2 credentials stored securely");
    
    println!("\n🎉 Gmail OAuth2 setup complete!");
    println!("   Your account should now work properly in Comunicado");
    println!("   Try running: cargo run --bin comunicado");
    
    Ok(())
}

// Dependencies needed in Cargo.toml:
// [dependencies]
// tokio = { version = "1", features = ["full"] }
// reqwest = { version = "0.11", features = ["json"] }
// serde_json = "1.0"
// chrono = { version = "0.4", features = ["serde"] }
// base64 = "0.22"