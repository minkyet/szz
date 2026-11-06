use super::models::MicrosoftTokenResponse;
use crate::error::{Result, SzzError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub fn base64url_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::with_capacity((data.len() * 4).div_ceil(3));
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as u32
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        result.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        if i + 1 < data.len() {
            result.push(ALPHABET[((triple >> 6) & 0x3F) as usize] as char);
        }
        if i + 2 < data.len() {
            result.push(ALPHABET[(triple & 0x3F) as usize] as char);
        }
        i += 3;
    }
    result
}

pub fn generate_pkce() -> (String, String) {
    let mut bytes = [0u8; 32];
    #[cfg(unix)]
    {
        if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
            use std::io::Read;
            let _ = file.read_exact(&mut bytes);
        }
    }
    #[cfg(not(unix))]
    {
        let mut seed = (std::process::id() as u64)
            ^ (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64);
        for b in bytes.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *b = (seed >> 33) as u8;
        }
    }

    let code_verifier = base64url_encode(&bytes);
    let hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64url_encode(&hash);

    (code_verifier, code_challenge)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopbackAuthHandle {
    pub port: u16,
    pub auth_url: String,
    pub redirect_uri: String,
    pub code_verifier: String,
}

pub fn percent_decode(input: &str) -> String {
    let mut result = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_val) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                result.push(hex_val);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

pub async fn start_loopback_listener(
    client_id: &str,
) -> Result<(LoopbackAuthHandle, TcpListener)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| SzzError::Auth(format!("Failed to bind local loopback listener: {}", e)))?;

    let port = listener
        .local_addr()
        .map_err(|e| SzzError::Auth(format!("Failed to get loopback port: {}", e)))?
        .port();

    let redirect_uri = format!("http://localhost:{}", port);
    let encoded_redirect_uri = format!("http%3A%2F%2Flocalhost%3A{}", port);
    let (code_verifier, code_challenge) = generate_pkce();

    let auth_url = format!(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize?client_id={}&response_type=code&redirect_uri={}&response_mode=query&scope=XboxLive.signin%20offline_access&code_challenge={}&code_challenge_method=S256&prompt=select_account",
        client_id,
        encoded_redirect_uri,
        code_challenge
    );

    let handle = LoopbackAuthHandle {
        port,
        auth_url,
        redirect_uri,
        code_verifier,
    };

    Ok((handle, listener))
}

pub fn extract_code_from_http_request(request: &str) -> Option<String> {
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split('?').nth(1)?;

    for pair in query.split('&') {
        let mut parts = pair.split('=');
        if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
            if k == "code" {
                return Some(percent_decode(v));
            }
        }
    }
    None
}

pub async fn await_loopback_code(listener: TcpListener, timeout_secs: u64) -> Result<String> {
    let timeout_fut = tokio::time::sleep(Duration::from_secs(timeout_secs));
    tokio::pin!(timeout_fut);

    loop {
        tokio::select! {
            res = listener.accept() => {
                let (mut socket, _) = res.map_err(|e| SzzError::Auth(format!("Loopback accept error: {}", e)))?;
                let mut buf = [0u8; 4096];
                let n = socket.read(&mut buf).await.map_err(|e| SzzError::Auth(format!("Loopback read error: {}", e)))?;
                let req_str = String::from_utf8_lossy(&buf[..n]);

                if let Some(code) = extract_code_from_http_request(&req_str) {
                    let html_body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>SZZ Launcher - Sign In</title>
    <style>
        body { background-color: #0f172a; color: #f8fafc; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
        .card { background-color: #1e293b; border: 1px solid #334155; border-radius: 16px; padding: 40px; text-align: center; max-width: 420px; box-shadow: 0 20px 25px -5px rgba(0,0,0,0.5); }
        h2 { color: #22c55e; margin: 0 0 12px 0; font-size: 24px; font-weight: 700; }
        p { color: #94a3b8; font-size: 15px; margin: 0; line-height: 1.5; }
    </style>
</head>
<body>
    <div class="card">
        <h2>Authentication Successful!</h2>
        <p>You may now close this tab and return to SZZ Launcher.</p>
    </div>
</body>
</html>"#;

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html_body.len(),
                        html_body
                    );

                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.flush().await;

                    return Ok(code);
                } else {
                    let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = socket.write_all(not_found.as_bytes()).await;
                    let _ = socket.flush().await;
                }
            }
            _ = &mut timeout_fut => {
                return Err(SzzError::Auth("Loopback authentication timed out waiting for browser sign in".to_string()));
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeErrorPayload {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

pub async fn request_device_code(
    client: &reqwest::Client,
    client_id: &str,
) -> Result<DeviceCodeResponse> {
    let url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
    let params = [
        ("client_id", client_id),
        ("scope", "XboxLive.signin offline_access"),
    ];

    let resp = client
        .post(url)
        .form(&params)
        .send()
        .await
        .map_err(|e| SzzError::Network {
            url: url.to_string(),
            source: e,
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(SzzError::Auth(format!(
            "Device code request failed: HTTP {} - {}",
            status, body
        )));
    }

    let device_resp: DeviceCodeResponse = resp.json().await.map_err(|e| SzzError::Network {
        url: url.to_string(),
        source: e,
    })?;

    Ok(device_resp)
}

pub async fn poll_device_code_token(
    client: &reqwest::Client,
    client_id: &str,
    device_code: &str,
) -> Result<Option<MicrosoftTokenResponse>> {
    let url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
    let params = [
        ("client_id", client_id),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
    ];

    let resp = client
        .post(url)
        .form(&params)
        .send()
        .await
        .map_err(|e| SzzError::Network {
            url: url.to_string(),
            source: e,
        })?;

    if resp.status().is_success() {
        let token_resp: MicrosoftTokenResponse = resp.json().await.map_err(|e| SzzError::Network {
            url: url.to_string(),
            source: e,
        })?;
        return Ok(Some(token_resp));
    }

    if let Ok(err_payload) = resp.json::<DeviceCodeErrorPayload>().await {
        match err_payload.error.as_str() {
            "authorization_pending" => Ok(None),
            "slow_down" => Ok(None),
            "expired_token" => Err(SzzError::Auth("Device login code has expired. Please try again.".to_string())),
            "access_denied" => Err(SzzError::Auth("Login request was cancelled by the user.".to_string())),
            other => Err(SzzError::Auth(format!(
                "Device login error: {} ({})",
                other,
                err_payload.error_description.unwrap_or_default()
            ))),
        }
    } else {
        Err(SzzError::Auth("Unexpected error polling device code token endpoint".to_string()))
    }
}
