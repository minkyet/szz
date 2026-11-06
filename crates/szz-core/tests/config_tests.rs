use szz_core::config::{load_env_file, msa_client_id, parse_dotenv_entries};
use tempfile::tempdir;

#[test]
fn test_parse_dotenv_entries() {
    let sample = r#"
# Comment line
SZZ_MSA_CLIENT_ID=test-client-id-123
ANOTHER_VAR="quoted-value"
SINGLE_QUOTED='single-quoted-val'
export EXPORTED_VAR=exported-123 # trailing comment
EMPTY_LINE=
"#;

    let entries = parse_dotenv_entries(sample);
    let map: std::collections::HashMap<_, _> = entries.into_iter().collect();

    assert_eq!(map.get("SZZ_MSA_CLIENT_ID").map(|s| s.as_str()), Some("test-client-id-123"));
    assert_eq!(map.get("ANOTHER_VAR").map(|s| s.as_str()), Some("quoted-value"));
    assert_eq!(map.get("SINGLE_QUOTED").map(|s| s.as_str()), Some("single-quoted-val"));
    assert_eq!(map.get("EXPORTED_VAR").map(|s| s.as_str()), Some("exported-123"));
}

#[test]
fn test_load_env_file_applies_variables() {
    let tmp = tempdir().expect("Failed to create tempdir");
    let env_path = tmp.path().join(".env");
    let test_key = "SZZ_TEST_CUSTOM_KEY_XYZ";
    let test_val = "custom_test_value_789";

    std::fs::write(&env_path, format!("{test_key}={test_val}\n")).expect("Failed to write .env");

    assert!(std::env::var(test_key).is_err());
    load_env_file(&env_path).expect("Failed to load .env");
    assert_eq!(std::env::var(test_key).ok().as_deref(), Some(test_val));
}

#[test]
fn test_msa_client_id() {
    if let Ok(expected) = std::env::var("SZZ_MSA_CLIENT_ID") {
        let trimmed = expected.trim();
        if !trimmed.is_empty() {
            assert_eq!(msa_client_id().unwrap(), trimmed);
        }
    }
}
