use serde::{Deserialize};

/// Google OAuth2 credentials file format (downloaded from Google Cloud Console)
#[derive(Debug, Deserialize)]
struct GoogleCredentialsFile {
    installed: GoogleInstalledApp,
}

#[derive(Debug, Deserialize)]
struct GoogleInstalledApp {
    client_id: String,
    project_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_secret: String,
    redirect_uris: Vec<String>,
}

fn main() {
    // Test JSON parsing with our test credentials file
    let test_json = std::fs::read_to_string("../test-credentials.json")
        .expect("Failed to read test credentials file");
    
    let credentials: GoogleCredentialsFile = serde_json::from_str(&test_json)
        .expect("Failed to parse credentials JSON");
    
    println!("✅ Successfully parsed Google credentials:");
    println!("   Client ID: {}", credentials.installed.client_id);
    println!("   Project ID: {}", credentials.installed.project_id);
    println!("   Client Secret: {}***", &credentials.installed.client_secret[..8]);
    println!("   Auth URI: {}", credentials.installed.auth_uri);
    println!("   Token URI: {}", credentials.installed.token_uri);
    println!("   Redirect URIs: {:?}", credentials.installed.redirect_uris);
    
    println!("\n✅ JSON parsing functionality verified!");
}