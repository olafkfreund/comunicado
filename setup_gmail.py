#!/usr/bin/env python3
"""
Simple Gmail OAuth2 Setup Script for Comunicado
Usage: python3 setup_gmail.py
"""

import json
import webbrowser
import http.server
import socketserver
import urllib.parse
import base64
import requests
from datetime import datetime, timedelta
import os
import sys
from threading import Thread
import time

# Configuration
CLIENT_SECRET_FILE = "/home/olafkfreund/client_secret_771552156772-oamkti00mbglr2o0k7ejo4spgldgeu0i.apps.googleusercontent.com.json"
CONFIG_DIR = "/home/olafkfreund/.config/comunicado"
REDIRECT_PORT = 8080
REDIRECT_URI = f"http://localhost:{REDIRECT_PORT}/oauth/callback"

# Global variable to store the authorization code
auth_code = None
server_stopped = False

class CallbackHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        global auth_code, server_stopped
        
        if '/oauth/callback' in self.path:
            # Parse the authorization code from the callback URL
            parsed_url = urllib.parse.urlparse(self.path)
            params = urllib.parse.parse_qs(parsed_url.query)
            
            if 'code' in params:
                auth_code = params['code'][0]
                
                # Send success response
                self.send_response(200)
                self.send_header('Content-type', 'text/html')
                self.end_headers()
                
                html = """
                <html>
                <head><title>Authorization Successful</title></head>
                <body style="font-family: Arial, sans-serif; text-align: center; padding: 50px;">
                    <h1 style="color: green;">✅ Authorization Successful!</h1>
                    <p>You can now close this browser window and return to the terminal.</p>
                    <p>Comunicado will complete the setup automatically.</p>
                </body>
                </html>
                """
                self.wfile.write(html.encode())
                server_stopped = True
                
            elif 'error' in params:
                error = params['error'][0]
                self.send_response(400)
                self.send_header('Content-type', 'text/html')
                self.end_headers()
                
                html = f"""
                <html>
                <head><title>Authorization Failed</title></head>
                <body style="font-family: Arial, sans-serif; text-align: center; padding: 50px;">
                    <h1 style="color: red;">❌ Authorization Failed</h1>
                    <p>Error: {error}</p>
                    <p>Please close this window and try again.</p>
                </body>
                </html>
                """
                self.wfile.write(html.encode())
                server_stopped = True
        else:
            self.send_response(404)
            self.end_headers()
    
    def log_message(self, format, *args):
        # Suppress server logs
        pass

def start_callback_server():
    """Start a simple HTTP server to handle OAuth2 callback"""
    global server_stopped
    
    with socketserver.TCPServer(("", REDIRECT_PORT), CallbackHandler) as httpd:
        print(f"🌐 Started callback server on http://localhost:{REDIRECT_PORT}")
        
        while not server_stopped:
            httpd.handle_request()
            time.sleep(0.1)

def main():
    print("🔧 Gmail OAuth2 Setup for Comunicado")
    print("====================================\n")
    
    # Read client credentials
    try:
        with open(CLIENT_SECRET_FILE, 'r') as f:
            client_data = json.load(f)
        
        client_id = client_data['installed']['client_id']
        client_secret = client_data['installed']['client_secret']
        
        print("✅ Found Google OAuth2 credentials")
        print(f"   Client ID: {client_id[:20]}...")
        print()
        
    except FileNotFoundError:
        print(f"❌ Client secret file not found: {CLIENT_SECRET_FILE}")
        print("   Please download it from Google Cloud Console")
        return 1
    except KeyError as e:
        print(f"❌ Invalid client secret file format: missing {e}")
        return 1
    
    # Create config directory if it doesn't exist
    os.makedirs(CONFIG_DIR, exist_ok=True)
    
    # Build authorization URL
    auth_params = {
        'client_id': client_id,
        'redirect_uri': REDIRECT_URI,
        'scope': 'https://mail.google.com/ https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile',
        'response_type': 'code',
        'access_type': 'offline',
        'prompt': 'consent'
    }
    
    auth_url = 'https://accounts.google.com/o/oauth2/auth?' + urllib.parse.urlencode(auth_params)
    
    print("🚀 Starting OAuth2 authorization process...")
    print("   1. A callback server will start on localhost:8080")
    print("   2. Your browser will open to Google's authorization page")
    print("   3. Log in and grant permissions")
    print("   4. You'll be redirected back automatically")
    print()
    
    input("Press Enter to continue...")
    
    # Start callback server in a separate thread
    server_thread = Thread(target=start_callback_server, daemon=True)
    server_thread.start()
    
    # Open browser
    print("🌐 Opening browser for authorization...")
    webbrowser.open(auth_url)
    
    print("⏳ Waiting for authorization...")
    print("   (This will timeout in 300 seconds)")
    
    # Wait for authorization code
    timeout = 300  # 5 minutes
    start_time = time.time()
    
    while auth_code is None and not server_stopped:
        if time.time() - start_time > timeout:
            print("❌ Authorization timeout!")
            print("   Please try again.")
            return 1
        
        time.sleep(1)
    
    if auth_code is None:
        print("❌ Authorization failed or was cancelled")
        return 1
    
    print(f"✅ Got authorization code: {auth_code[:10]}...")
    
    # Exchange code for tokens
    print("🔄 Exchanging code for access tokens...")
    
    token_data = {
        'client_id': client_id,
        'client_secret': client_secret,
        'code': auth_code,
        'grant_type': 'authorization_code',
        'redirect_uri': REDIRECT_URI
    }
    
    try:
        response = requests.post('https://oauth2.googleapis.com/token', data=token_data)
        response.raise_for_status()
        token_response = response.json()
        
    except requests.RequestException as e:
        print(f"❌ Token exchange failed: {e}")
        return 1
    
    if 'error' in token_response:
        print(f"❌ Token exchange error: {token_response['error']}")
        if 'error_description' in token_response:
            print(f"   {token_response['error_description']}")
        return 1
    
    access_token = token_response['access_token']
    refresh_token = token_response.get('refresh_token')
    expires_in = token_response.get('expires_in', 3600)
    
    print("✅ Got OAuth2 tokens successfully!")
    print(f"   Access token: {access_token[:20]}...")
    if refresh_token:
        print(f"   Refresh token: {refresh_token[:20]}...")
    print()
    
    # Calculate expiration time
    expires_at = datetime.utcnow() + timedelta(seconds=expires_in)
    
    # Create account config
    account_config = {
        "account_id": "gmail_olaf_loken_gmail_com",
        "display_name": "Olaf Krasicki Freund",
        "email_address": "olaf.loken@gmail.com",
        "provider": "gmail",
        "imap_server": "imap.gmail.com",
        "imap_port": 993,
        "smtp_server": "smtp.gmail.com",
        "smtp_port": 587,
        "token_expires_at": expires_at.isoformat() + 'Z',
        "scopes": [
            "https://mail.google.com/",
            "https://www.googleapis.com/auth/userinfo.email",
            "https://www.googleapis.com/auth/userinfo.profile"
        ]
    }
    
    # Write account config
    config_path = os.path.join(CONFIG_DIR, "gmail_olaf_loken_gmail_com.json")
    with open(config_path, 'w') as f:
        json.dump(account_config, f, indent=2)
    print(f"✅ Account config written to: {config_path}")
    
    # Write tokens (base64 encoded)
    access_token_encoded = base64.b64encode(access_token.encode()).decode()
    access_token_path = os.path.join(CONFIG_DIR, "gmail_olaf_loken_gmail_com.access.token")
    with open(access_token_path, 'w') as f:
        f.write(access_token_encoded)
    print(f"✅ Access token written to: {access_token_path}")
    
    if refresh_token:
        refresh_token_encoded = base64.b64encode(refresh_token.encode()).decode()
        refresh_token_path = os.path.join(CONFIG_DIR, "gmail_olaf_loken_gmail_com.refresh.token")
        with open(refresh_token_path, 'w') as f:
            f.write(refresh_token_encoded)
        print(f"✅ Refresh token written to: {refresh_token_path}")
    
    # Store OAuth2 credentials
    client_id_encoded = base64.b64encode(client_id.encode()).decode()
    client_secret_encoded = base64.b64encode(client_secret.encode()).decode()
    
    client_id_path = os.path.join(CONFIG_DIR, "gmail_olaf_loken_gmail_com.client_id.cred")
    client_secret_path = os.path.join(CONFIG_DIR, "gmail_olaf_loken_gmail_com.client_secret.cred")
    
    with open(client_id_path, 'w') as f:
        f.write(client_id_encoded)
    with open(client_secret_path, 'w') as f:
        f.write(client_secret_encoded)
    
    print("✅ OAuth2 credentials stored securely")
    
    print("\n🎉 Gmail OAuth2 setup complete!")
    print("   Your account should now work properly in Comunicado")
    print("   Try running: cargo run --bin comunicado")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())