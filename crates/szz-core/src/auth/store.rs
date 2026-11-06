use crate::auth::{AuthSession, DevAuthSession, MicrosoftAuthSession};
use crate::error::{Result, SzzError};
use crate::paths::SzzPaths;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Microsoft,
    Dev,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    pub id: String,
    pub account_type: AccountType,
    pub uuid: String,
    pub username: String,
    #[serde(default)]
    pub skin_url: Option<String>,
    pub minecraft_token: String,
    #[serde(default)]
    pub minecraft_token_expires_at: Option<u64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub last_used: u64,
}

impl AccountEntry {
    pub fn to_session(&self) -> Box<dyn AuthSession> {
        match self.account_type {
            AccountType::Dev => Box::new(DevAuthSession {
                username: self.username.clone(),
                uuid: self.uuid.clone(),
                token: self.minecraft_token.clone(),
            }),
            AccountType::Microsoft => Box::new(MicrosoftAuthSession {
                username: self.username.clone(),
                uuid: self.uuid.clone(),
                access_token: self.minecraft_token.clone(),
                skin_url: self.skin_url.clone(),
            }),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountsConfig {
    pub active_account_id: Option<String>,
    #[serde(default)]
    pub accounts: Vec<AccountEntry>,
}

#[derive(Debug, Clone)]
pub struct AccountStore {
    paths: SzzPaths,
    config: AccountsConfig,
}

impl AccountStore {
    pub fn load(paths: &SzzPaths) -> Result<Self> {
        let file_path = paths.accounts_file();
        let config = match std::fs::read_to_string(&file_path) {
            Ok(content) => serde_json::from_str::<AccountsConfig>(&content).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => AccountsConfig::default(),
            Err(e) => return Err(SzzError::Io { path: file_path, source: e }),
        };

        Ok(Self {
            paths: paths.clone(),
            config,
        })
    }

    pub fn save(&self) -> Result<()> {
        let file_path = self.paths.accounts_file();
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SzzError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let json = serde_json::to_string_pretty(&self.config)?;
        std::fs::write(&file_path, json).map_err(|e| SzzError::Io {
            path: file_path.clone(),
            source: e,
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }

    pub fn accounts(&self) -> &[AccountEntry] {
        &self.config.accounts
    }

    pub fn active_account_id(&self) -> Option<&str> {
        self.config.active_account_id.as_deref()
    }

    pub fn active_account(&self) -> Option<&AccountEntry> {
        let active_id = self.config.active_account_id.as_deref()?;
        self.config.accounts.iter().find(|a| a.id == active_id)
    }

    pub fn active_account_mut(&mut self) -> Option<&mut AccountEntry> {
        let active_id = self.config.active_account_id.clone()?;
        self.config.accounts.iter_mut().find(|a| a.id == active_id)
    }

    pub fn add_or_update_account(&mut self, entry: AccountEntry) -> Result<()> {
        let id = entry.id.clone();
        if let Some(existing) = self.config.accounts.iter_mut().find(|a| a.id == id) {
            *existing = entry;
        } else {
            self.config.accounts.push(entry);
        }
        self.config.active_account_id = Some(id);
        self.save()
    }

    pub fn add_or_update_microsoft_account(
        &mut self,
        profile: &super::models::MinecraftProfileResponse,
        access_token: String,
        expires_in: u64,
        refresh_token: Option<String>,
    ) -> Result<AccountEntry> {
        let now = current_timestamp();
        let entry = AccountEntry {
            id: profile.id.clone(),
            account_type: AccountType::Microsoft,
            uuid: profile.id.clone(),
            username: profile.name.clone(),
            skin_url: profile.active_skin_url().map(String::from),
            minecraft_token: access_token,
            minecraft_token_expires_at: Some(now + expires_in),
            refresh_token,
            last_used: now,
        };
        self.add_or_update_account(entry.clone())?;
        Ok(entry)
    }

    pub fn switch_active(&mut self, account_id: &str) -> Result<()> {
        let acc = self
            .config
            .accounts
            .iter_mut()
            .find(|a| a.id == account_id)
            .ok_or_else(|| SzzError::Auth(format!("Account with ID {} not found", account_id)))?;
        acc.last_used = current_timestamp();
        self.config.active_account_id = Some(account_id.to_string());
        self.save()
    }

    pub fn remove_account(&mut self, account_id: &str) -> Result<()> {
        self.config.accounts.retain(|a| a.id != account_id);
        if self.config.active_account_id.as_deref() == Some(account_id) {
            self.config.active_account_id = self.config.accounts.first().map(|a| a.id.clone());
        }
        self.save()
    }

    pub fn create_dev_account(&mut self, username: &str) -> Result<AccountEntry> {
        let id = format!("dev-{}", username.trim().to_lowercase());
        let entry = AccountEntry {
            id: id.clone(),
            account_type: AccountType::Dev,
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
            username: username.trim().to_string(),
            skin_url: None,
            minecraft_token: "offline_token".to_string(),
            minecraft_token_expires_at: None,
            refresh_token: None,
            last_used: current_timestamp(),
        };
        self.add_or_update_account(entry.clone())?;
        Ok(entry)
    }

    pub fn get_active_session(&self) -> Box<dyn AuthSession> {
        self.active_account()
            .map(|a| a.to_session())
            .unwrap_or_else(|| Box::new(DevAuthSession::new("SzzPlayer")))
    }

    pub async fn ensure_active_session_valid(
        &mut self,
        client: &reqwest::Client,
        client_id: &str,
    ) -> Result<Box<dyn AuthSession>> {
        let active = match self.active_account() {
            Some(a) => a.clone(),
            None => return Ok(Box::new(DevAuthSession::new("SzzPlayer"))),
        };

        if active.account_type == AccountType::Dev {
            return Ok(active.to_session());
        }

        let now = current_timestamp();
        let is_expired = match active.minecraft_token_expires_at {
            Some(exp) => exp <= now + 300,
            None => false,
        };

        if !is_expired {
            return Ok(active.to_session());
        }

        let refresh_token = active.refresh_token.ok_or_else(|| {
            SzzError::Auth("No refresh token available to refresh expired session".to_string())
        })?;

        println!(
            "[szz-core] Refreshing expired Minecraft token for {}...",
            active.username
        );
        let new_tokens =
            super::msa::refresh_microsoft_token(client, client_id, &refresh_token).await?;
        let (mc_login, profile) =
            super::msa::complete_minecraft_auth(client, &new_tokens.access_token).await?;

        let updated_entry = self.add_or_update_microsoft_account(
            &profile,
            mc_login.access_token,
            mc_login.expires_in,
            new_tokens.refresh_token.or(Some(refresh_token)),
        )?;

        Ok(updated_entry.to_session())
    }
}
