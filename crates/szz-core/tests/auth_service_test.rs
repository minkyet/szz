use szz_core::auth::service::{
    create_dev_account, ensure_active_session, get_auth_state, open_browser, remove_account,
    switch_active_account,
};
use szz_core::auth::AccountType;
use szz_core::paths::SzzPaths;
use tempfile::tempdir;

#[tokio::test]
async fn test_auth_service_dev_account_lifecycle() {
    let tmp = tempdir().expect("temp dir should be created");
    let paths = SzzPaths::custom(tmp.path().to_path_buf());

    // 1. Initially no accounts
    let initial_state = get_auth_state(&paths).expect("should load initial auth state");
    assert!(initial_state.accounts.is_empty());
    assert!(initial_state.active_account.is_none());

    // 2. ensure_active_session fails when no active account exists
    let no_session_err = ensure_active_session(&paths).await;
    assert!(no_session_err.is_err());

    // 3. Create first dev account "PlayerOne"
    let dev1 = create_dev_account(&paths, "PlayerOne").expect("dev account 1 created");
    assert_eq!(dev1.username, "PlayerOne");
    assert_eq!(dev1.account_type, AccountType::Dev);
    assert!(dev1.is_active);

    // 4. Check auth state has PlayerOne as active
    let state = get_auth_state(&paths).expect("auth state after player 1");
    assert_eq!(state.accounts.len(), 1);
    assert_eq!(state.active_account.as_ref().unwrap().username, "PlayerOne");

    // 5. ensure_active_session succeeds and returns DevAuthSession
    let session = ensure_active_session(&paths)
        .await
        .expect("session should be valid for dev account");
    assert_eq!(session.username(), "PlayerOne");
    assert_eq!(session.uuid(), "00000000-0000-0000-0000-000000000000");

    // 6. Create second dev account "PlayerTwo"
    let dev2 = create_dev_account(&paths, "PlayerTwo").expect("dev account 2 created");
    assert_eq!(dev2.username, "PlayerTwo");
    assert!(dev2.is_active);

    let state2 = get_auth_state(&paths).expect("auth state after player 2");
    assert_eq!(state2.accounts.len(), 2);
    assert_eq!(state2.active_account.as_ref().unwrap().username, "PlayerTwo");

    // 7. Switch active account back to PlayerOne by case-insensitive username
    let switched =
        switch_active_account(&paths, "playerone").expect("switch to playerone by username");
    assert_eq!(switched.username, "PlayerOne");
    assert!(switched.is_active);

    let state3 = get_auth_state(&paths).expect("auth state after switch");
    assert_eq!(state3.active_account.as_ref().unwrap().username, "PlayerOne");

    // 8. Switching to non-existent account fails
    let bad_switch = switch_active_account(&paths, "nonexistent");
    assert!(bad_switch.is_err());

    // 9. Remove PlayerOne
    remove_account(&paths, "PlayerOne").expect("remove PlayerOne");
    let state4 = get_auth_state(&paths).expect("auth state after remove");
    assert_eq!(state4.accounts.len(), 1);
    assert_eq!(state4.accounts[0].username, "PlayerTwo");
    // Fallback active account became PlayerTwo
    assert_eq!(state4.active_account.as_ref().unwrap().username, "PlayerTwo");

    // 10. Remove nonexistent account fails
    let bad_remove = remove_account(&paths, "PlayerOne");
    assert!(bad_remove.is_err());

    // 11. Remove remaining account
    remove_account(&paths, "PlayerTwo").expect("remove PlayerTwo");
    let state5 = get_auth_state(&paths).expect("auth state after all removed");
    assert!(state5.accounts.is_empty());
    assert!(state5.active_account.is_none());
}

#[tokio::test]
async fn test_auth_service_ensure_session_microsoft_expired_without_refresh() {
    use szz_core::auth::{AccountEntry, AccountStore};

    let tmp = tempdir().expect("temp dir should be created");
    let paths = SzzPaths::custom(tmp.path().to_path_buf());

    let mut store = AccountStore::load(&paths).expect("store should load");
    let expired_entry = AccountEntry {
        id: "msa-test-uuid".to_string(),
        account_type: AccountType::Microsoft,
        uuid: "msa-test-uuid".to_string(),
        username: "MsaExpiredPlayer".to_string(),
        skin_url: None,
        minecraft_token: "expired_token".to_string(),
        minecraft_token_expires_at: Some(100), // Far in the past
        refresh_token: None,                  // No refresh token available
        last_used: 100,
    };
    store
        .add_or_update_account(expired_entry)
        .expect("should save expired msa account");

    // Attempting ensure_active_session should fail because token is expired and refresh token is missing
    let err = ensure_active_session(&paths).await;
    assert!(err.is_err());
    let err_msg = err.unwrap_err().to_string();
    assert!(err_msg.contains("No refresh token"));
}

#[test]
fn test_open_browser_url_scheme_validation() {
    // Non-http/https schemes must be rejected with SzzError::Security
    let file_err = open_browser("file:///etc/passwd").unwrap_err();
    assert!(matches!(file_err, szz_core::error::SzzError::Security(_)));

    let js_err = open_browser("javascript:alert(1)").unwrap_err();
    assert!(matches!(js_err, szz_core::error::SzzError::Security(_)));

    // Leading hyphens (command injection / option flag injection) must be rejected
    let hyphen_err = open_browser("--help").unwrap_err();
    assert!(matches!(hyphen_err, szz_core::error::SzzError::Security(_)));

    // Malformed URLs must be rejected
    let malformed_err = open_browser("not a url").unwrap_err();
    assert!(matches!(malformed_err, szz_core::error::SzzError::Security(_)));
}


