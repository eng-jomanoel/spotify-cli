use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;

const SCOPES: &str =
    "user-read-playback-state user-modify-playback-state user-read-currently-playing \
     playlist-read-private playlist-read-collaborative user-library-read user-library-modify \
     user-read-recently-played";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

impl TokenData {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= self.expires_at.saturating_sub(60)
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..64).map(|_| rng.gen::<u8>()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let result = hasher.finalize();
    URL_SAFE_NO_PAD.encode(result)
}

fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..16).map(|_| rng.gen::<u8>()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

async fn exchange_code(
    client: &Client,
    code: &str,
    verifier: &str,
    config: &Config,
) -> Result<TokenData> {
    let redirect_uri = format!("http://localhost:{}/callback", config.redirect_port);

    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", &redirect_uri),
        ("client_id", &config.client_id),
        ("code_verifier", verifier),
    ];

    let resp = client
        .post("https://accounts.spotify.com/api/token")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("Token exchange failed: {}", text);
    }

    let token_resp: TokenResponse = resp.json().await?;
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + token_resp.expires_in;

    Ok(TokenData {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token.unwrap_or_default(),
        expires_at,
    })
}

async fn refresh_token(client: &Client, token: &TokenData, config: &Config) -> Result<TokenData> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", &token.refresh_token),
        ("client_id", &config.client_id),
        ("client_secret", &config.client_secret),
    ];

    let resp = client
        .post("https://accounts.spotify.com/api/token")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await?;
        bail!("Token refresh failed: {}", text);
    }

    let token_resp: TokenResponse = resp.json().await?;
    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + token_resp.expires_in;

    Ok(TokenData {
        access_token: token_resp.access_token,
        refresh_token: token_resp
            .refresh_token
            .unwrap_or_else(|| token.refresh_token.clone()),
        expires_at,
    })
}

fn wait_for_callback(port: u16, expected_state: &str) -> Result<String> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .with_context(|| format!("Could not bind to port {}", port))?;

    let (stream, _) = listener.accept()?;
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Parse GET /callback?code=xxx&state=yyy HTTP/1.1
    let path = request_line
        .split_whitespace()
        .nth(1)
        .context("Invalid HTTP request")?;

    let query = path.split('?').nth(1).unwrap_or("");
    let mut code = None;
    let mut state = None;

    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        match (kv.next(), kv.next()) {
            (Some("code"), Some(v)) => code = Some(urlencoding_decode(v)),
            (Some("state"), Some(v)) => state = Some(v.to_string()),
            _ => {}
        }
    }

    // Send success response
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body style='font-family:monospace;background:#0a0a0a;color:#1DB954;text-align:center;padding:80px'>\
        <h1>✓ spotify-cli authenticated!</h1>\
        <p style='color:#b3b3b3'>You can close this tab and return to the terminal.</p>\
        </body></html>";
    (&stream).write_all(response.as_bytes())?;

    if state.as_deref() != Some(expected_state) {
        bail!("State mismatch — possible CSRF attack");
    }

    code.context("No code in callback")
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

pub async fn ensure_authenticated(config: &Config) -> Result<TokenData> {
    let client = Client::new();
    let token_path = Config::token_path();

    // Try to load existing token
    if token_path.exists() {
        let content = fs::read_to_string(&token_path)?;
        if let Ok(token) = serde_json::from_str::<TokenData>(&content) {
            if !token.is_expired() {
                return Ok(token);
            }
            // Try refresh
            match refresh_token(&client, &token, config).await {
                Ok(new_token) => {
                    let json = serde_json::to_string_pretty(&new_token)?;
                    fs::write(&token_path, json)?;
                    return Ok(new_token);
                }
                Err(e) => {
                    eprintln!("Token refresh failed: {}, re-authenticating...", e);
                }
            }
        }
    }

    // Fresh auth via PKCE
    let verifier = generate_code_verifier();
    let challenge = generate_code_challenge(&verifier);
    let state = generate_state();
    let redirect_uri = format!("http://localhost:{}/callback", config.redirect_port);

    let auth_url = format!(
        "https://accounts.spotify.com/authorize?\
         client_id={}&response_type=code&redirect_uri={}&\
         scope={}&state={}&code_challenge_method=S256&code_challenge={}",
        config.client_id,
        urlencoding_encode(&redirect_uri),
        urlencoding_encode(SCOPES),
        state,
        challenge
    );

    println!("\n🎵 spotify-cli — Authorization Required");
    println!("─────────────────────────────────────────");
    println!("Opening browser for Spotify login...");
    println!("If it doesn't open, visit:\n{}\n", auth_url);

    let _ = open::that(&auth_url);

    println!("Waiting for callback on port {}...", config.redirect_port);
    let code = wait_for_callback(config.redirect_port, &state)?;

    println!("✓ Got authorization code, exchanging for token...");
    let token = exchange_code(&client, &code, &verifier, config).await?;

    fs::create_dir_all(Config::config_dir())?;
    let json = serde_json::to_string_pretty(&token)?;
    fs::write(&token_path, json)?;
    println!("✓ Token saved. Launching TUI...\n");

    Ok(token)
}

fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
