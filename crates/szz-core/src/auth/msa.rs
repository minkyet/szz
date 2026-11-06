use super::models::*;
use crate::error::{Result, SzzError};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
pub fn build_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("szz-launcher/0.1.0")
        .build()
        .map_err(|e| SzzError::Network {
            url: "http-client-init".to_string(),
            source: e,
        })
}

async fn post_token_endpoint(
    client: &reqwest::Client,
    params: &[(&str, &str)],
    action_description: &str,
) -> Result<MicrosoftTokenResponse> {
    let url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
    let resp = client
        .post(url)
        .form(params)
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
            "Failed to {}: HTTP {} - {}",
            action_description, status, body
        )));
    }

    resp.json().await.map_err(|e| SzzError::Network {
        url: url.to_string(),
        source: e,
    })
}

pub async fn exchange_code_for_tokens(
    client: &reqwest::Client,
    client_id: &str,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<MicrosoftTokenResponse> {
    let params = [
        ("client_id", client_id),
        ("code", code),
        ("code_verifier", code_verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
        ("scope", "XboxLive.signin offline_access"),
    ];
    post_token_endpoint(client, &params, "exchange authorization code").await
}

pub async fn refresh_microsoft_token(
    client: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> Result<MicrosoftTokenResponse> {
    let params = [
        ("client_id", client_id),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
        ("scope", "XboxLive.signin offline_access"),
    ];
    post_token_endpoint(client, &params, "refresh Microsoft token").await
}

pub async fn authenticate_xbox_live(
    client: &reqwest::Client,
    ms_access_token: &str,
) -> Result<(String, String)> {
    let url = "https://user.auth.xboxlive.com/user/authenticate";
    let body = XboxLiveAuthRequest {
        properties: XboxLiveAuthProperties {
            auth_method: "RPS".to_string(),
            site_name: "user.auth.xboxlive.com".to_string(),
            rps_ticket: format!("d={}", ms_access_token),
        },
        relying_party: "http://auth.xboxlive.com".to_string(),
        token_type: "JWT".to_string(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("x-xbl-contract-version", HeaderValue::from_static("1"));

    let resp = client
        .post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| SzzError::Network {
            url: url.to_string(),
            source: e,
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        return Err(SzzError::Auth(format!(
            "Xbox Live user authentication failed: HTTP {} - {}",
            status, err_body
        )));
    }

    let auth_resp: XboxLiveAuthResponse = resp.json().await.map_err(|e| SzzError::Network {
        url: url.to_string(),
        source: e,
    })?;

    let uhs = auth_resp
        .user_hash()
        .ok_or_else(|| {
            SzzError::Auth("No user hash (uhs) found in Xbox Live response".to_string())
        })?
        .to_string();

    Ok((auth_resp.token, uhs))
}

pub async fn authorize_xsts(client: &reqwest::Client, xbl_token: &str) -> Result<String> {
    let url = "https://xsts.auth.xboxlive.com/xsts/authorize";
    let body = XstsAuthRequest {
        properties: XstsAuthProperties {
            sandbox_id: "RETAIL".to_string(),
            user_tokens: vec![xbl_token.to_string()],
        },
        relying_party: "rp://api.minecraftservices.com/".to_string(),
        token_type: "JWT".to_string(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("x-xbl-contract-version", HeaderValue::from_static("1"));

    let resp = client
        .post(url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| SzzError::Network {
            url: url.to_string(),
            source: e,
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        if let Ok(err) = resp.json::<XstsErrorResponse>().await {
            if let Some(xerr) = err.x_err {
                let msg = match xerr {
                    2148916233 => "No Xbox Live account found. Please sign into xbox.com to create a Gamertag.",
                    2148916238 => "Child account detected. Parental consent is required before accessing Minecraft.",
                    2148916235 => "Xbox Live is not available in your country/region.",
                    2148916236 | 2148916237 => "Adult verification is required for this account.",
                    _ => "Xbox Security Token Service (XSTS) returned an authorization error.",
                };
                return Err(SzzError::Auth(format!("{} (XErr: {})", msg, xerr)));
            }
        }
        return Err(SzzError::Auth(format!(
            "XSTS authorization failed with HTTP {}",
            status
        )));
    }

    let auth_resp: XboxLiveAuthResponse = resp.json().await.map_err(|e| SzzError::Network {
        url: url.to_string(),
        source: e,
    })?;

    Ok(auth_resp.token)
}

pub async fn login_minecraft_with_xbox(
    client: &reqwest::Client,
    uhs: &str,
    xsts_token: &str,
) -> Result<MinecraftLoginResponse> {
    let url = "https://api.minecraftservices.com/authentication/login_with_xbox";
    let body = MinecraftLoginRequest {
        identity_token: format!("XBL3.0 x={};{}", uhs, xsts_token),
    };

    let resp = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| SzzError::Network {
            url: url.to_string(),
            source: e,
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        return Err(SzzError::Auth(format!(
            "Minecraft services login failed: HTTP {} - {}",
            status, err_body
        )));
    }

    let login_resp: MinecraftLoginResponse = resp.json().await.map_err(|e| SzzError::Network {
        url: url.to_string(),
        source: e,
    })?;

    Ok(login_resp)
}

pub async fn get_minecraft_profile(
    client: &reqwest::Client,
    mc_token: &str,
) -> Result<MinecraftProfileResponse> {
    let url = "https://api.minecraftservices.com/minecraft/profile";
    let resp = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {}", mc_token))
        .send()
        .await
        .map_err(|e| SzzError::Network {
            url: url.to_string(),
            source: e,
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(SzzError::Auth(
                "This Microsoft account does not own Minecraft Java Edition or has not created a profile yet."
                    .to_string(),
            ));
        }
        return Err(SzzError::Auth(format!(
            "Failed to fetch Minecraft profile: HTTP {} - {}",
            status, err_body
        )));
    }

    let profile: MinecraftProfileResponse = resp.json().await.map_err(|e| SzzError::Network {
        url: url.to_string(),
        source: e,
    })?;

    Ok(profile)
}

pub async fn complete_minecraft_auth(
    client: &reqwest::Client,
    ms_access_token: &str,
) -> Result<(MinecraftLoginResponse, MinecraftProfileResponse)> {
    let (xbl_token, uhs) = authenticate_xbox_live(client, ms_access_token).await?;
    let xsts_token = authorize_xsts(client, &xbl_token).await?;
    let mc_login = login_minecraft_with_xbox(client, &uhs, &xsts_token).await?;
    let profile = get_minecraft_profile(client, &mc_login.access_token).await?;

    Ok((mc_login, profile))
}
