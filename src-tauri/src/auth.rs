//! OAuth + secure token storage.
//!
//! AniList desktop auth uses the **implicit grant**. We open
//! `https://anilist.co/api/v2/oauth/authorize?client_id=…&response_type=token`
//! (no `redirect_uri` — AniList uses the URL registered on the client) and the
//! user approves. Then, depending on the client's registered redirect URL:
//!   * `anitrax://oauth/anilist` — the deep-link plugin hands us the full
//!     redirect URL and we pull `access_token` out of the fragment; or
//!   * `https://anilist.co/api/v2/oauth/pin` — AniList shows the token on a page
//!     and the user pastes it into Settings.
//!
//! Tokens never touch the SQLite file — they live in the OS keychain.

use keyring::Entry;
use url::Url;

use crate::error::{AppError, AppResult};
use crate::tracker::model::ServiceKind;

const KEYRING_SERVICE: &str = "dev.bcnet.anitrax";
pub const ANILIST_REDIRECT: &str = "anitrax://oauth/anilist";

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

/// Extract `access_token` from a redirect URL like
/// `anitrax://oauth/anilist#access_token=abc&token_type=Bearer&expires_in=...`
pub fn parse_anilist_redirect(redirect: &str) -> Option<String> {
    let url = Url::parse(redirect).ok()?;

    // Implicit grant returns the token in the URL fragment.
    if let Some(fragment) = url.fragment() {
        for pair in fragment.split('&') {
            let mut it = pair.splitn(2, '=');
            if it.next() == Some("access_token") {
                if let Some(tok) = it.next() {
                    return Some(tok.to_string());
                }
            }
        }
    }

    // Fall back to the query string just in case.
    for (k, v) in url.query_pairs() {
        if k == "access_token" {
            return Some(v.into_owned());
        }
    }
    None
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
    fn pulls_token_from_fragment() {
        let url = "anitrax://oauth/anilist#access_token=xyz123&token_type=Bearer&expires_in=31536000";
        assert_eq!(parse_anilist_redirect(url).as_deref(), Some("xyz123"));
    }

    #[test]
    fn pulls_token_from_query() {
        let url = "anitrax://oauth/anilist?access_token=abc";
        assert_eq!(parse_anilist_redirect(url).as_deref(), Some("abc"));
    }

    #[test]
    fn missing_token_is_none() {
        let url = "anitrax://oauth/anilist#error=access_denied";
        assert_eq!(parse_anilist_redirect(url), None);
    }

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

    #[test]
    fn parses_token_from_pin_redirect_fragment() {
        let url = "https://anilist.co/api/v2/oauth/pin#access_token=eyJ0eXAi.abc.def&token_type=Bearer";
        assert_eq!(
            parse_anilist_redirect(url).as_deref(),
            Some("eyJ0eXAi.abc.def")
        );
    }
}
