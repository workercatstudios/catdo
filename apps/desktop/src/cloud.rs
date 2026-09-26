//! Clerk public-client device authorization. Credentials stay in Secret Service;
//! task storage contains only the Clerk user ID and durable sync state.
use anyhow::{Context, Result, bail, ensure};
use catdo_core::sync::{Pending, Snapshot};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub issuer: String,
    pub desktop_client_id: String,
}
#[derive(Clone, Deserialize)]
pub struct Discovery {
    pub device_authorization_endpoint: String,
    pub token_endpoint: String,
    pub revocation_endpoint: Option<String>,
}
#[derive(Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: Option<u64>,
}
#[derive(Clone)]
pub struct Login {
    pub api: String,
    pub config: Config,
    pub discovery: Discovery,
    pub code: DeviceCode,
}
#[derive(Serialize, Deserialize)]
struct Credentials {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
    config: ConfigSaved,
    discovery: DiscoverySaved,
}
#[derive(Serialize, Deserialize)]
struct ConfigSaved {
    client_id: String,
}
#[derive(Serialize, Deserialize)]
struct DiscoverySaved {
    token_endpoint: String,
    revocation_endpoint: Option<String>,
}
#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}
#[derive(Deserialize)]
struct Identity {
    #[serde(rename = "userId")]
    user_id: String,
}

#[derive(Debug)]
pub struct TermsRequired;

impl std::fmt::Display for TermsRequired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Review WorkerCat terms and confirm you are 13+ in your browser using this CatDo account. Then retry sync. Your tasks are saved here.")
    }
}

impl std::error::Error for TermsRequired {}

fn check_sync_status(status: reqwest::StatusCode) -> Result<()> {
    if status == reqwest::StatusCode::PRECONDITION_REQUIRED {
        return Err(TermsRequired.into());
    }
    ensure!(
        status.is_success() || status == reqwest::StatusCode::CONFLICT,
        "Sync is unavailable ({}). Your changes are saved on this device.",
        status.as_u16()
    );
    Ok(())
}

pub fn api_url() -> String {
    std::env::var("CATDO_API_URL")
        .unwrap_or_else(|_| "https://catdo.workercat.com".into())
        .trim_end_matches('/')
        .into()
}
fn client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn safe_url(value: &str, local: bool) -> Result<()> {
    let url = reqwest::Url::parse(value)?;
    ensure!(
        url.scheme() == "https"
            || (local
                && url.scheme() == "http"
                && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))),
        "Sign-in requires a secure server URL."
    );
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "Invalid server URL."
    );
    Ok(())
}
fn entry(api: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new("com.workercat.catdo", api)?)
}
fn store(api: &str, credentials: &Credentials) -> Result<()> {
    entry(api)?
        .set_password(&serde_json::to_string(credentials)?)
        .context(
            "Could not save sign-in to your desktop keyring. Unlock Secret Service and try again.",
        )
}
fn load(api: &str) -> Result<Credentials> {
    let secret = entry(api)?
        .get_password()
        .context("Sign in to sync. Your tasks are saved on this device.")?;
    Ok(serde_json::from_str(&secret)?)
}
fn identity(client: &Client, api: &str, token: &str) -> Result<String> {
    safe_url(api, true)?;
    let response = client
        .get(format!("{api}/api/me"))
        .bearer_auth(token)
        .send()?;
    ensure!(response.status().is_success(), "Sign in again to sync.");
    Ok(response.json::<Identity>()?.user_id)
}

pub fn begin_login(api: &str) -> Result<Login> {
    safe_url(api, true)?;
    let client = client()?;
    let config: Config = client
        .get(format!("{api}/api/config"))
        .send()?
        .error_for_status()?
        .json()?;
    ensure!(
        !config.desktop_client_id.is_empty(),
        "Desktop sign-in is not configured on this server yet."
    );
    safe_url(&config.issuer, false)?;
    let discovery: Discovery = client
        .get(format!(
            "{}/.well-known/oauth-authorization-server",
            config.issuer.trim_end_matches('/')
        ))
        .send()?
        .error_for_status()?
        .json()?;
    for endpoint in [
        &discovery.device_authorization_endpoint,
        &discovery.token_endpoint,
    ] {
        safe_url(endpoint, false)?;
        ensure!(
            reqwest::Url::parse(endpoint)?.origin()
                == reqwest::Url::parse(&config.issuer)?.origin(),
            "Unexpected sign-in endpoint."
        );
    }
    let response = client
        .post(&discovery.device_authorization_endpoint)
        .form(&[
            ("client_id", config.desktop_client_id.as_str()),
            ("scope", "openid profile email offline_access"),
        ])
        .send()?;
    ensure!(
        response.status().is_success(),
        "Could not start Clerk sign-in. Please try again."
    );
    let code: DeviceCode = response.json()?;
    safe_url(&code.verification_uri, false)?;
    if let Some(url) = &code.verification_uri_complete {
        safe_url(url, false)?;
    }
    Ok(Login {
        api: api.into(),
        config,
        discovery,
        code,
    })
}
pub fn finish_login(login: Login) -> Result<String> {
    let client = client()?;
    let deadline = Instant::now() + Duration::from_secs(login.code.expires_in.min(3600));
    let mut interval = login.code.interval.unwrap_or(5).max(1);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_secs(interval));
        if Instant::now() >= deadline {
            break;
        }
        let response = client
            .post(&login.discovery.token_endpoint)
            .form(&[
                ("client_id", login.config.desktop_client_id.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", login.code.device_code.as_str()),
            ])
            .send();
        let response = match response {
            Ok(r) => r,
            Err(_) => {
                interval = (interval * 2).min(60);
                continue;
            }
        };
        if response.status().is_success() {
            let tokens: Tokens = response.json()?;
            let user = identity(&client, &login.api, &tokens.access_token)?;
            let credentials = Credentials {
                access_token: tokens.access_token,
                refresh_token: tokens
                    .refresh_token
                    .context("Clerk did not provide offline access. Please sign in again.")?,
                expires_at: now() + tokens.expires_in,
                config: ConfigSaved {
                    client_id: login.config.desktop_client_id,
                },
                discovery: DiscoverySaved {
                    token_endpoint: login.discovery.token_endpoint,
                    revocation_endpoint: login.discovery.revocation_endpoint,
                },
            };
            store(&login.api, &credentials)?;
            return Ok(user);
        }
        let error: serde_json::Value = response.json()?;
        match error["error"].as_str() {
            Some("authorization_pending") => {}
            Some("slow_down") => interval += 5,
            Some("access_denied") => bail!("Sign-in was declined. Your local tasks are unchanged."),
            Some("expired_token") => break,
            _ => bail!("Clerk could not complete sign-in. Please try again."),
        }
    }
    bail!("The sign-in code expired. Start sign-in again.")
}
fn authorized(api: &str) -> Result<(Client, Credentials)> {
    let client = client()?;
    let mut credentials = load(api)?;
    if credentials.expires_at <= now() + 60 {
        safe_url(&credentials.discovery.token_endpoint, false)?;
        let response = client
            .post(&credentials.discovery.token_endpoint)
            .form(&[
                ("client_id", credentials.config.client_id.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", credentials.refresh_token.as_str()),
            ])
            .send()?;
        ensure!(
            response.status().is_success(),
            "Your sign-in expired. Sign in again; your changes are saved locally."
        );
        let tokens: Tokens = response.json()?;
        credentials.access_token = tokens.access_token;
        if let Some(refresh) = tokens.refresh_token {
            credentials.refresh_token = refresh;
        }
        credentials.expires_at = now() + tokens.expires_in;
        store(api, &credentials)?;
    }
    Ok((client, credentials))
}
pub fn sync(api: &str, owner: &str, pending: Option<Pending>) -> Result<(Snapshot, bool)> {
    safe_url(api, true)?;
    let (client, credentials) = authorized(api)?;
    ensure!(
        identity(&client, api, &credentials.access_token)? == owner,
        "This device's tasks belong to a different account. Sign in to the original account."
    );
    let request = if let Some(pending) = pending {
        client.post(format!("{api}/api/sync")).json(&pending)
    } else {
        client.get(format!("{api}/api/sync"))
    };
    let response = request.bearer_auth(&credentials.access_token).send()?;
    let status = response.status();
    check_sync_status(status)?;
    Ok((response.json()?, status.as_u16() != 409))
}

pub fn sign_out(api: &str) -> Result<()> {
    // Local removal always works offline. Revocation is best effort, without keeping
    // credentials in the task database or blocking the UI.
    if let Ok(credentials) = load(api)
        && let Some(url) = credentials.discovery.revocation_endpoint
        && safe_url(&url, false).is_ok()
        && let Ok(client) = client()
    {
        let _ = client
            .post(url)
            .form(&[
                ("client_id", credentials.config.client_id),
                ("token", credentials.refresh_token),
                ("token_type_hint", "refresh_token".into()),
            ])
            .send();
    }
    match entry(api)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn terms_required_is_actionable_without_accepting_a_snapshot() {
        let error = check_sync_status(StatusCode::PRECONDITION_REQUIRED).unwrap_err();
        assert!(error.is::<TermsRequired>());
        assert!(error.to_string().contains("this CatDo account"));
        assert!(error.to_string().contains("Your tasks are saved here"));
    }

    #[test]
    fn sync_success_and_conflicts_remain_readable_but_other_failures_do_not() {
        assert!(check_sync_status(StatusCode::OK).is_ok());
        assert!(check_sync_status(StatusCode::CONFLICT).is_ok());
        let error = check_sync_status(StatusCode::UNAUTHORIZED).unwrap_err();
        assert!(!error.is::<TermsRequired>());
        assert!(check_sync_status(StatusCode::SERVICE_UNAVAILABLE).is_err());
    }
}
