use szz_core::auth::models::*;
use szz_core::auth::{AuthSession, DevAuthSession, MicrosoftAuthSession};

#[test]
fn test_dev_auth_session() {
    let dev = DevAuthSession::new("Steve");
    assert_eq!(dev.username(), "Steve");
    assert_eq!(dev.uuid(), "00000000-0000-0000-0000-000000000000");
    assert_eq!(dev.access_token(), "offline_token");
    assert_eq!(dev.user_type(), "msa");
}

#[test]
fn test_microsoft_auth_session() {
    let session = MicrosoftAuthSession::new(
        "Alex",
        "12345678-1234-1234-1234-123456789abc",
        "test_jwt_token",
        Some("http://textures.minecraft.net/texture/alex".to_string()),
    );
    assert_eq!(session.username(), "Alex");
    assert_eq!(session.uuid(), "12345678-1234-1234-1234-123456789abc");
    assert_eq!(session.access_token(), "test_jwt_token");
    assert_eq!(session.user_type(), "msa");
    assert_eq!(
        session.skin_url.as_deref(),
        Some("http://textures.minecraft.net/texture/alex")
    );
}

#[test]
fn test_xbox_live_response_parsing() {
    let json_data = r#"{
        "IssueInstant": "2026-09-30T00:00:00.0000000Z",
        "NotAfter": "2026-10-01T00:00:00.0000000Z",
        "Token": "xbl_token_sample",
        "DisplayClaims": {
            "xui": [
                {
                    "uhs": "1234567890"
                }
            ]
        }
    }"#;

    let parsed: XboxLiveAuthResponse = serde_json::from_str(json_data).expect("Must parse XBL response");
    assert_eq!(parsed.token, "xbl_token_sample");
    assert_eq!(parsed.user_hash(), Some("1234567890"));
}

#[test]
fn test_xsts_error_parsing() {
    let json_data = r#"{
        "Identity": "0",
        "XErr": 2148916238,
        "Message": "",
        "Redirect": "https://start.ui.xboxlive.com/AddChild"
    }"#;

    let parsed: XstsErrorResponse = serde_json::from_str(json_data).expect("Must parse XErr response");
    assert_eq!(parsed.x_err, Some(2148916238));
}

#[test]
fn test_minecraft_profile_skin_parsing() {
    let json_data = r#"{
        "id": "4566e69fc90748ee8d71d7ba5aa00c20",
        "name": "Alex",
        "skins": [
            {
                "id": "skin-id-1",
                "state": "ACTIVE",
                "url": "http://textures.minecraft.net/texture/abc",
                "variant": "SLIM"
            }
        ]
    }"#;

    let profile: MinecraftProfileResponse = serde_json::from_str(json_data).expect("Must parse profile");
    assert_eq!(profile.name, "Alex");
    assert_eq!(profile.id, "4566e69fc90748ee8d71d7ba5aa00c20");
    assert_eq!(profile.active_skin_url(), Some("http://textures.minecraft.net/texture/abc"));
}
