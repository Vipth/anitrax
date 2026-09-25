//! OAuth + secure token storage.
//!
//! AniList desktop auth uses the **implicit grant**, token-only: we open
//! `https://anilist.co/api/v2/oauth/authorize?client_id=…&response_type=token`
//! (no `redirect_uri` — AniList uses the URL registered on the client, which
//! for this client id is AniList's own `https://anilist.co/api/v2/oauth/pin`
//! page). The user approves, AniList shows them the access token on that
//! page, and they paste it into Settings — no custom URI scheme, no deep-link
//! OS registration to rely on.
//!
//! Tokens never touch the SQLite file — they live in the OS keychain.

use keyring::Entry;

use crate::error::{AppError, AppResult};
use crate::tracker::model::ServiceKind;

const KEYRING_SERVICE: &str = "dev.bcnet.anitrax";
/// AniTrax's own AniList API client (registered once under the maintainer's
/// account, redirect set to AniList's own PIN page) — every install shares
/// this id, same as any other public app consuming AniList's API. A client
/// id identifies the *application*, not the person: AniList still issues
/// each user their own separate access token, so this carries no per-user
/// data.
pub const ANILIST_CLIENT_ID: &str = "50498";

fn entry(service: ServiceKind) -> AppResult<Entry> {
    Entry::new(KEYRING_SERVICE, &format!("token:{}", service.as_str()))
        .map_err(|e| AppError::Keychain(e.to_string()))
}

pub fn store_token(service: ServiceKind, token: &str) -> AppResult<()> {
    entry(service)?
        .set_password(token)
        .map_err(|e| AppError::Keychain(e.to_string()))
}

pub fn load_token(service: ServiceKind) -> AppResult<Option<String>> {
    match entry(service)?.get_password() {
        Ok(t) => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}

pub fn require_token(service: ServiceKind) -> AppResult<String> {
    load_token(service)?.ok_or_else(|| AppError::NotAuthenticated(service.as_str().into()))
}

pub fn delete_token(service: ServiceKind) -> AppResult<()> {
    match entry(service)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}

/// Build the AniList authorize URL for the implicit grant.
///
/// Note: AniList's implicit grant does **not** accept a `redirect_uri` parameter
/// — passing one yields `unsupported_grant_type`. The redirect target is
/// whatever is registered on the client in AniList's developer settings.
pub fn anilist_authorize_url(client_id: &str) -> String {
    format!(
        "https://anilist.co/api/v2/oauth/authorize?client_id={}&response_type=token",
        urlencoding(client_id),
    )
}

fn urlencoding(s: &str) -> String {
    // Minimal percent-encoding for the characters we actually pass.
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ':' => "%3A".into(),
            '/' => "%2F".into(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_has_no_redirect_uri() {
        // AniList's implicit grant rejects a redirect_uri param.
        let url = anilist_authorize_url("50498");
        assert_eq!(
            url,
            "https://anilist.co/api/v2/oauth/authorize?client_id=50498&response_type=token"
        );
        assert!(!url.contains("redirect_uri"));
    }
}
