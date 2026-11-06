use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct XboxLiveAuthRequest {
    #[serde(rename = "Properties")]
    pub properties: XboxLiveAuthProperties,
    #[serde(rename = "RelyingParty")]
    pub relying_party: String,
    #[serde(rename = "TokenType")]
    pub token_type: String,
}

#[derive(Debug, Serialize)]
pub struct XboxLiveAuthProperties {
    #[serde(rename = "AuthMethod")]
    pub auth_method: String,
    #[serde(rename = "SiteName")]
    pub site_name: String,
    #[serde(rename = "RpsTicket")]
    pub rps_ticket: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XboxLiveUserClaim {
    pub uhs: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XboxLiveDisplayClaims {
    pub xui: Vec<XboxLiveUserClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XboxLiveAuthResponse {
    #[serde(rename = "IssueInstant")]
    pub issue_instant: Option<String>,
    #[serde(rename = "NotAfter")]
    pub not_after: Option<String>,
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: XboxLiveDisplayClaims,
}

impl XboxLiveAuthResponse {
    pub fn user_hash(&self) -> Option<&str> {
        self.display_claims.xui.first().map(|x| x.uhs.as_str())
    }
}

#[derive(Debug, Serialize)]
pub struct XstsAuthRequest {
    #[serde(rename = "Properties")]
    pub properties: XstsAuthProperties,
    #[serde(rename = "RelyingParty")]
    pub relying_party: String,
    #[serde(rename = "TokenType")]
    pub token_type: String,
}

#[derive(Debug, Serialize)]
pub struct XstsAuthProperties {
    #[serde(rename = "SandboxId")]
    pub sandbox_id: String,
    #[serde(rename = "UserTokens")]
    pub user_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XstsErrorResponse {
    #[serde(rename = "Identity")]
    pub identity: Option<String>,
    #[serde(rename = "XErr")]
    pub x_err: Option<u64>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
    #[serde(rename = "Redirect")]
    pub redirect: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MinecraftLoginRequest {
    #[serde(rename = "identityToken")]
    pub identity_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftLoginResponse {
    pub username: Option<String>,
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfileSkin {
    pub id: String,
    pub state: String,
    pub url: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftProfileResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<MinecraftProfileSkin>,
}

impl MinecraftProfileResponse {
    pub fn active_skin_url(&self) -> Option<&str> {
        self.skins
            .iter()
            .find(|s| s.state.eq_ignore_ascii_case("ACTIVE"))
            .map(|s| s.url.as_str())
            .or_else(|| self.skins.first().map(|s| s.url.as_str()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountProfile {
    pub id: String,
    pub account_type: crate::auth::store::AccountType,
    pub uuid: String,
    pub username: String,
    pub skin_url: Option<String>,
    pub is_active: bool,
}

impl AccountProfile {
    pub fn from_entry(entry: &crate::auth::store::AccountEntry, is_active: bool) -> Self {
        Self {
            id: entry.id.clone(),
            account_type: entry.account_type.clone(),
            uuid: entry.uuid.clone(),
            username: entry.username.clone(),
            skin_url: entry.skin_url.clone(),
            is_active,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatePayload {
    pub accounts: Vec<AccountProfile>,
    pub active_account: Option<AccountProfile>,
}

