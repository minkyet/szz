pub mod flows;
pub mod models;
pub mod msa;
pub mod service;
pub mod store;

pub use flows::*;
pub use models::*;
pub use service::{
    create_dev_account, ensure_active_session, get_auth_state, login_with_loopback, open_browser,
    poll_and_complete_device_code, remove_account, switch_active_account,
};
pub use store::{AccountEntry, AccountStore, AccountType};

pub trait AuthSession: std::fmt::Debug + Send + Sync {
    fn username(&self) -> &str;
    fn uuid(&self) -> &str;
    fn access_token(&self) -> &str;
    fn user_type(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct DevAuthSession {
    pub username: String,
    pub uuid: String,
    pub token: String,
}

impl DevAuthSession {
    pub fn new(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
            token: "offline_token".to_string(),
        }
    }
}

impl AuthSession for DevAuthSession {
    fn username(&self) -> &str {
        &self.username
    }

    fn uuid(&self) -> &str {
        &self.uuid
    }

    fn access_token(&self) -> &str {
        &self.token
    }

    fn user_type(&self) -> &str {
        "msa"
    }
}

#[derive(Debug, Clone)]
pub struct MicrosoftAuthSession {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub skin_url: Option<String>,
}

impl MicrosoftAuthSession {
    pub fn new(
        username: impl Into<String>,
        uuid: impl Into<String>,
        access_token: impl Into<String>,
        skin_url: Option<String>,
    ) -> Self {
        Self {
            username: username.into(),
            uuid: uuid.into(),
            access_token: access_token.into(),
            skin_url,
        }
    }
}

impl AuthSession for MicrosoftAuthSession {
    fn username(&self) -> &str {
        &self.username
    }

    fn uuid(&self) -> &str {
        &self.uuid
    }

    fn access_token(&self) -> &str {
        &self.access_token
    }

    fn user_type(&self) -> &str {
        "msa"
    }
}
