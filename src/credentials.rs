use keyring::Entry;

const KEYCHAIN_SERVICE: &str = "eventfold";

/// Saves a credential to the OS keychain under service `"eventfold"`.
/// Supported keys: `"anthropic_key"`, `"github_pat"`.
/// Passing an empty string stores an empty value (effectively clearing it).
#[tauri::command]
pub fn save_credential(key: String, value: String) -> Result<(), String> {
    let entry = Entry::new(KEYCHAIN_SERVICE, &key)
        .map_err(|e| format!("Keychain error: {}", e))?;
    entry
        .set_password(&value)
        .map_err(|e| format!("Failed to save credential '{}': {}", key, e))
}

/// Retrieves a credential from the OS keychain.
/// Returns `Ok(None)` if not set, `Err` only if the keychain is unavailable.
#[tauri::command]
pub fn get_credential(key: String) -> Result<Option<String>, String> {
    let entry = Entry::new(KEYCHAIN_SERVICE, &key)
        .map_err(|e| format!("Keychain error: {}", e))?;
    match entry.get_password() {
        Ok(val) => Ok(Some(val)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to get credential '{}': {}", key, e)),
    }
}

// ── Internal helper — called by generator and github commands directly ────────
// Not a Tauri command; just a plain Rust function for intra-crate use.

pub fn get_credential_internal(key: &str) -> Result<Option<String>, String> {
    let entry = Entry::new(KEYCHAIN_SERVICE, key)
        .map_err(|e| format!("Keychain error: {}", e))?;
    match entry.get_password() {
        Ok(val) => Ok(Some(val)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to get credential '{}': {}", key, e)),
    }
}
