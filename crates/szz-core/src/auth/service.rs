use crate::auth::flows::{self, DeviceCodeResponse};
use crate::auth::models::{AccountProfile, AuthStatePayload};
use crate::auth::msa;
use crate::auth::store::{AccountStore, AccountType};
use crate::auth::AuthSession;
use crate::error::{Result, SzzError};
use crate::paths::SzzPaths;

/// Opens a web browser with the provided URL on the host operating system.
/// Validates that the URL scheme is strictly http or https and does not contain argument injection.
pub fn open_browser(url: &str) -> Result<()> {
    if url.starts_with('-') {
        return Err(SzzError::Security(
            "URL cannot start with a hyphen".to_string(),
        ));
    }

    crate::catalog::validate_url_scheme(url)
        .map_err(|e| SzzError::Security(e.to_string()))?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map_err(|e| SzzError::Auth(format!("Failed to open browser on Windows: {}", e)))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| SzzError::Auth(format!("Failed to open browser on macOS: {}", e)))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| SzzError::Auth(format!("Failed to open browser on Linux: {}", e)))?;
    }
    Ok(())
}

/// Orchestrates loopback OAuth2 login flow:
/// 1. Reads client_id from environment or configuration
/// 2. Starts local HTTP loopback listener on random port
/// 3. Opens browser using `open_browser_fn` callback
/// 4. Awaits authorization code from browser redirection
/// 5. Exchanges authorization code for Microsoft tokens
/// 6. Completes Minecraft authentication (Xbox Live -> XSTS -> Minecraft Token -> Profile)
/// 7. Persists new account to `AccountStore` and marks it active
pub async fn login_with_loopback<F>(paths: &SzzPaths, open_browser_fn: F) -> Result<AccountProfile>
where
    F: FnOnce(&str) -> Result<()>,
{
    let client_id = crate::config::msa_client_id()?;

    let (handle, listener) = flows::start_loopback_listener(&client_id).await?;

    open_browser_fn(&handle.auth_url)?;

    let code = flows::await_loopback_code(listener, 180).await?;

    let http_client = msa::build_http_client()?;
    let tokens = msa::exchange_code_for_tokens(
        &http_client,
        &client_id,
        &code,
        &handle.code_verifier,
        &handle.redirect_uri,
    )
    .await?;

    let (mc_login, profile) =
        msa::complete_minecraft_auth(&http_client, &tokens.access_token).await?;

    let mut store = AccountStore::load(paths)?;
    let entry = store.add_or_update_microsoft_account(
        &profile,
        mc_login.access_token,
        mc_login.expires_in,
        tokens.refresh_token,
    )?;

    Ok(AccountProfile::from_entry(&entry, true))
}

/// Requests a device code from Microsoft identity service for headless authentication.
pub async fn request_device_code() -> Result<DeviceCodeResponse> {
    let client_id = crate::config::msa_client_id()?;
    let http_client = msa::build_http_client()?;
    flows::request_device_code(&http_client, &client_id).await
}

/// Polls Microsoft OAuth endpoint for device code authorization.
/// If user completed authorization, completes Minecraft auth and updates `AccountStore`.
pub async fn poll_and_complete_device_code(
    paths: &SzzPaths,
    device_code: &str,
) -> Result<Option<AccountProfile>> {
    let client_id = crate::config::msa_client_id()?;
    let http_client = msa::build_http_client()?;

    let maybe_tokens =
        flows::poll_device_code_token(&http_client, &client_id, device_code).await?;

    let tokens = match maybe_tokens {
        Some(t) => t,
        None => return Ok(None),
    };

    let (mc_login, profile) =
        msa::complete_minecraft_auth(&http_client, &tokens.access_token).await?;

    let mut store = AccountStore::load(paths)?;
    let entry = store.add_or_update_microsoft_account(
        &profile,
        mc_login.access_token,
        mc_login.expires_in,
        tokens.refresh_token,
    )?;

    Ok(Some(AccountProfile::from_entry(&entry, true)))
}

/// Retrieves all accounts and the currently active account from `AccountStore`.
pub fn get_auth_state(paths: &SzzPaths) -> Result<AuthStatePayload> {
    let store = AccountStore::load(paths)?;
    let active_id = store.active_account_id();

    let accounts: Vec<AccountProfile> = store
        .accounts()
        .iter()
        .map(|acc| AccountProfile::from_entry(acc, Some(acc.id.as_str()) == active_id))
        .collect();

    let active_account = accounts.iter().find(|a| a.is_active).cloned();

    Ok(AuthStatePayload {
        accounts,
        active_account,
    })
}

/// Switches active account by UUID or username.
pub fn switch_active_account(paths: &SzzPaths, identifier: &str) -> Result<AccountProfile> {
    let mut store = AccountStore::load(paths)?;
    let account = store
        .accounts()
        .iter()
        .find(|a| a.id == identifier || a.username.eq_ignore_ascii_case(identifier))
        .cloned()
        .ok_or_else(|| SzzError::Auth(format!("Account '{}' not found", identifier)))?;

    store.switch_active(&account.id)?;
    Ok(AccountProfile::from_entry(&account, true))
}

/// Removes an account by UUID or username.
pub fn remove_account(paths: &SzzPaths, identifier: &str) -> Result<()> {
    let mut store = AccountStore::load(paths)?;
    let account_id = store
        .accounts()
        .iter()
        .find(|a| a.id == identifier || a.username.eq_ignore_ascii_case(identifier))
        .map(|a| a.id.clone())
        .ok_or_else(|| SzzError::Auth(format!("Account '{}' not found", identifier)))?;

    store.remove_account(&account_id)
}

/// Creates a new offline development account and sets it active.
pub fn create_dev_account(paths: &SzzPaths, username: &str) -> Result<AccountProfile> {
    let mut store = AccountStore::load(paths)?;
    let entry = store.create_dev_account(username)?;
    Ok(AccountProfile::from_entry(&entry, true))
}

/// Validates that the active account session is valid, refreshing expired Microsoft tokens if needed.
/// Returns error if no active account is configured.
pub async fn ensure_active_session(paths: &SzzPaths) -> Result<Box<dyn AuthSession>> {
    let mut store = AccountStore::load(paths)?;

    let active = match store.active_account() {
        Some(a) => a.clone(),
        None => {
            return Err(SzzError::Auth(
                "No active account found. Please run 'szz-cli auth login' or 'szz-cli auth add-dev <username>' first.".to_string(),
            ));
        }
    };

    if active.account_type == AccountType::Dev {
        return Ok(active.to_session());
    }

    let client_id = crate::config::msa_client_id()?;
    let http_client = msa::build_http_client()?;

    store.ensure_active_session_valid(&http_client, &client_id).await
}
