use szz_core::auth::{AccountEntry, AccountStore, AccountType};
use szz_core::paths::SzzPaths;
use tempfile::tempdir;

#[test]
fn test_account_store_lifecycle() {
    let tmp = tempdir().expect("Must create tempdir");
    let paths = SzzPaths::custom(tmp.path());

    // 1. Initial load on empty dir
    let mut store = AccountStore::load(&paths).expect("Must load store");
    assert_eq!(store.accounts().len(), 0);
    assert_eq!(store.active_account_id(), None);

    // Default session when empty is Dev SzzPlayer
    let session = store.get_active_session();
    assert_eq!(session.username(), "SzzPlayer");

    // 2. Create Dev account
    let dev_acc = store
        .create_dev_account("TestPlayer")
        .expect("Must create dev account");
    assert_eq!(dev_acc.username, "TestPlayer");
    assert_eq!(dev_acc.id, "dev-testplayer");
    assert_eq!(store.active_account_id(), Some("dev-testplayer"));

    // 3. Reload from disk
    let mut reloaded = AccountStore::load(&paths).expect("Must reload store");
    assert_eq!(reloaded.accounts().len(), 1);
    assert_eq!(reloaded.active_account_id(), Some("dev-testplayer"));
    assert_eq!(reloaded.get_active_session().username(), "TestPlayer");

    // 4. Add Microsoft Account
    let ms_acc = AccountEntry {
        id: "ms-uuid-1".to_string(),
        account_type: AccountType::Microsoft,
        uuid: "11111111-2222-3333-4444-555555555555".to_string(),
        username: "OfficialPlayer".to_string(),
        skin_url: Some("http://textures.minecraft.net/texture/sample".to_string()),
        minecraft_token: "jwt_token_sample".to_string(),
        minecraft_token_expires_at: Some(2000000000),
        refresh_token: Some("sample_refresh_token".to_string()),
        last_used: 1000,
    };
    reloaded
        .add_or_update_account(ms_acc)
        .expect("Must add MS account");

    assert_eq!(reloaded.accounts().len(), 2);
    assert_eq!(reloaded.active_account_id(), Some("ms-uuid-1"));
    let ms_session = reloaded.get_active_session();
    assert_eq!(ms_session.username(), "OfficialPlayer");
    assert_eq!(ms_session.access_token(), "jwt_token_sample");

    // 5. Switch active account back to dev
    reloaded
        .switch_active("dev-testplayer")
        .expect("Must switch active account");
    assert_eq!(reloaded.active_account_id(), Some("dev-testplayer"));
    assert_eq!(reloaded.get_active_session().username(), "TestPlayer");

    // 6. Remove Microsoft account
    reloaded
        .remove_account("ms-uuid-1")
        .expect("Must remove account");
    assert_eq!(reloaded.accounts().len(), 1);
    assert_eq!(reloaded.active_account_id(), Some("dev-testplayer"));
}
