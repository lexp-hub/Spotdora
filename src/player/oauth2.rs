//! Provides a Spotify access token using the OAuth authorization code flow
//! with PKCE.
//!
//! Assuming sufficient scopes, the returned access token may be used with Spotify's
//! Web API, and/or to establish a new Session with [`librespot_core`].
//!
//! The authorization code flow is an interactive process which requires a web browser
//! to complete. The resulting code must then be provided back from the browser to this
//! library for exchange into an access token. Providing the code can be automatic via
//! a spawned http server (mimicking Spotify's client), or manually via stdin. The latter
//! is appropriate for headless systems.

use crate::app::credentials::Credentials;

use log::{error, info, trace};
use oauth2::reqwest::async_http_client;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use oauth2::{PkceCodeVerifier, RefreshToken, RequestTokenError};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::task::JoinHandle;
use url::Url;

use super::TokenStore;

pub const WEB_CLIENT_ID: &str = "782ae96ea60f4cdf986a766049607005";
pub const PLAYER_CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
pub const REDIRECT_URI: &str = "http://127.0.0.1:8898/login";
pub const SCOPES: &str = "user-read-private,\
playlist-read-private,\
playlist-read-collaborative,\
user-library-read,\
user-library-modify,\
user-top-read,\
user-read-recently-played,\
user-read-playback-state,\
playlist-modify-public,\
playlist-modify-private,\
user-modify-playback-state";

pub struct SpotOauthClient {
    web_client: BasicClient,
    player_client: BasicClient,
    token_store: Arc<TokenStore>,
}

pub struct AuthcodeChallenge {
    web_pkce_verifier: PkceCodeVerifier,
    player_pkce_verifier: PkceCodeVerifier,
    pub auth_url: Url,
    listener: JoinHandle<Result<(AuthorizationCode, AuthorizationCode), OAuthError>>,
}

impl SpotOauthClient {
    pub fn new(token_store: Arc<TokenStore>) -> Self {
        let auth_url = AuthUrl::new("https://accounts.spotify.com/authorize".to_string())
            .expect("Malformed URL");
        let token_url = TokenUrl::new("https://accounts.spotify.com/api/token".to_string())
            .expect("Malformed URL");
        let redirect_url = RedirectUrl::new(REDIRECT_URI.to_string()).expect("Malformed URL");

        let web_client = BasicClient::new(
            ClientId::new(WEB_CLIENT_ID.to_string()),
            None,
            auth_url.clone(),
            Some(token_url.clone()),
        )
        .set_redirect_uri(redirect_url.clone());

        let player_client = BasicClient::new(
            ClientId::new(PLAYER_CLIENT_ID.to_string()),
            None,
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(redirect_url);

        Self {
            web_client,
            player_client,
            token_store,
        }
    }

    pub async fn spawn_authcode_listener(
        &self,
        notify_complete: impl FnOnce() + 'static,
    ) -> Result<AuthcodeChallenge, OAuthError> {
        let (web_pkce_challenge, web_pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (player_pkce_challenge, player_pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let request_scopes: Vec<oauth2::Scope> =
            SCOPES.split(",").map(|s| Scope::new(s.into())).collect();

        let (auth_url, web_csrf_token) = self
            .web_client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(request_scopes)
            .set_pkce_challenge(web_pkce_challenge)
            .url();

        let (player_auth_url, player_csrf_token) = self
            .player_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("streaming".into()))
            .set_pkce_challenge(player_pkce_challenge)
            .url();

        Ok(AuthcodeChallenge {
            web_pkce_verifier,
            player_pkce_verifier,
            auth_url,
            listener: tokio::task::spawn_local(async move {
                let result =
                    wait_for_authcodes(web_csrf_token, player_auth_url, player_csrf_token).await;
                notify_complete();
                result
            }),
        })
    }

    /// Obtain Spotify access tokens using authorization code with PKCE OAuth flow for both Web API and streaming.
    pub async fn exchange_authcode(
        &self,
        challenge: AuthcodeChallenge,
    ) -> Result<Credentials, OAuthError> {
        let (web_code, player_code) = challenge
            .listener
            .await
            .map_err(|_| OAuthError::AuthCodeListenerTerminated)??;

        let web_token = self
            .web_client
            .exchange_code(web_code)
            .set_pkce_verifier(challenge.web_pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| match e {
                RequestTokenError::ServerResponse(res) => {
                    error!(
                        "An error occured while exchanging web code: {}",
                        res.to_string()
                    );
                    OAuthError::ExchangeCode { e: res.to_string() }
                }
                e => OAuthError::ExchangeCode { e: e.to_string() },
            })?;

        let player_token = self
            .player_client
            .exchange_code(player_code)
            .set_pkce_verifier(challenge.player_pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| match e {
                RequestTokenError::ServerResponse(res) => {
                    error!(
                        "An error occured while exchanging player code: {}",
                        res.to_string()
                    );
                    OAuthError::ExchangeCode { e: res.to_string() }
                }
                e => OAuthError::ExchangeCode { e: e.to_string() },
            })?;

        trace!("Obtained new web token: {web_token:?}");
        trace!("Obtained new player token: {player_token:?}");

        let refresh_token = web_token
            .refresh_token()
            .ok_or(OAuthError::NoRefreshToken)?
            .secret()
            .to_string();

        let player_refresh = player_token
            .refresh_token()
            .map(|t| t.secret().to_string());

        let token = Credentials {
            access_token: web_token.access_token().secret().to_string(),
            refresh_token,
            token_expiry_time: Some(
                SystemTime::now()
                    + web_token
                        .expires_in()
                        .unwrap_or_else(|| Duration::from_secs(3600)),
            ),
            player_access_token: Some(player_token.access_token().secret().to_string()),
            player_refresh_token: player_refresh,
            player_token_expiry_time: Some(
                SystemTime::now()
                    + player_token
                        .expires_in()
                        .unwrap_or_else(|| Duration::from_secs(3600)),
            ),
        };

        self.token_store.set(token.clone()).await;
        Ok(token)
    }

    pub async fn get_valid_token(&self) -> Result<Credentials, OAuthError> {
        let token = self.token_store.get().await.ok_or(OAuthError::LoggedOut)?;
        if token.token_expired() {
            self.refresh_token(token).await
        } else {
            Ok(token)
        }
    }

    pub async fn refresh_token(&self, mut old_token: Credentials) -> Result<Credentials, OAuthError> {
        let Ok(token) = self
            .web_client
            .exchange_refresh_token(&RefreshToken::new(old_token.refresh_token.clone()))
            .request_async(async_http_client)
            .await
            .inspect_err(|e| {
                if let RequestTokenError::ServerResponse(res) = e {
                    error!(
                        "An error occured while refreshing the web token: {}",
                        res.to_string()
                    );
                }
            })
        else {
            self.token_store.clear().await;
            return Err(OAuthError::NoRefreshToken);
        };

        let refresh_token = token
            .refresh_token()
            .map(|r| r.secret().to_string())
            .unwrap_or(old_token.refresh_token);

        old_token.access_token = token.access_token().secret().to_string();
        old_token.refresh_token = refresh_token;
        old_token.token_expiry_time = Some(
            SystemTime::now()
                + token
                    .expires_in()
                    .unwrap_or_else(|| Duration::from_secs(3600)),
        );

        if let Some(player_refresh) = old_token.player_refresh_token.clone() {
            if let Ok(p_token) = self
                .player_client
                .exchange_refresh_token(&RefreshToken::new(player_refresh))
                .request_async(async_http_client)
                .await
            {
                old_token.player_access_token = Some(p_token.access_token().secret().to_string());
                if let Some(r) = p_token.refresh_token() {
                    old_token.player_refresh_token = Some(r.secret().to_string());
                }
                old_token.player_token_expiry_time = Some(
                    SystemTime::now()
                        + p_token
                            .expires_in()
                            .unwrap_or_else(|| Duration::from_secs(3600)),
                );
            }
        }

        self.token_store.set(old_token.clone()).await;
        Ok(old_token)
    }

    pub async fn refresh_token_at_expiry(&self) -> Result<Credentials, OAuthError> {
        let Some(old_token) = self.token_store.get_cached().await.take() else {
            return Err(OAuthError::NoRefreshToken);
        };

        let duration = old_token
            .token_expiry_time
            .and_then(|d| d.duration_since(SystemTime::now()).ok())
            .unwrap_or(Duration::from_secs(120));

        info!(
            "Refreshing token in approx {}min",
            duration.as_secs().div_euclid(60)
        );
        tokio::time::sleep(duration.saturating_sub(Duration::from_secs(10))).await;

        info!("Refreshing token...");
        self.refresh_token(old_token).await
    }
}

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Auth code param not found in URI")]
    AuthCodeNotFound,

    #[error("CSRF token param not found in URI")]
    CsrfTokenNotFound,

    #[error("Failed to bind server to {addr} ({e})")]
    AuthCodeListenerBind { addr: SocketAddr, e: io::Error },

    #[error("Listener terminated without accepting a connection")]
    AuthCodeListenerTerminated,

    #[error("Failed to parse redirect URI from HTTP request")]
    AuthCodeListenerParse,

    #[error("Failed to write HTTP response")]
    AuthCodeListenerWrite,

    #[error("Failed to exchange code for access token ({e})")]
    ExchangeCode { e: String },

    #[error("Spotify did not provide a refresh token")]
    NoRefreshToken,

    #[error("No saved token")]
    LoggedOut,

    #[error("Mismatched state during auth code exchange")]
    InvalidState,
}

/// Spawn HTTP server to accept OAuth callbacks for both Web API and Player, automatically chaining them.
async fn wait_for_authcodes(
    web_csrf_token: CsrfToken,
    player_auth_url: Url,
    player_csrf_token: CsrfToken,
) -> Result<(AuthorizationCode, AuthorizationCode), OAuthError> {
    let addr = get_socket_address(REDIRECT_URI).expect("Invalid redirect uri");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| OAuthError::AuthCodeListenerBind { addr, e })?;

    // Wait for the Web API authorization code, then redirect the browser to the Player auth URL
    let web_code = wait_for_single_code(&listener, &web_csrf_token, Some(&player_auth_url)).await?;

    // Wait for the Player authorization code, then finish with login.html
    let player_code = wait_for_single_code(&listener, &player_csrf_token, None).await?;

    Ok((web_code, player_code))
}

async fn wait_for_single_code(
    listener: &tokio::net::TcpListener,
    expected_state: &CsrfToken,
    next_url: Option<&Url>,
) -> Result<AuthorizationCode, OAuthError> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|_| OAuthError::AuthCodeListenerTerminated)?;

        let mut request_line = String::new();
        let mut reader = BufReader::new(&mut stream);
        if reader.read_line(&mut request_line).await.is_err() {
            continue;
        }

        let (state, code) = match parse_query(&request_line) {
            Ok(res) => res,
            Err(_) => {
                let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(resp.as_bytes()).await;
                continue;
            }
        };

        if *expected_state.secret() != *state.secret() {
            let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(resp.as_bytes()).await;
            continue;
        }

        let response = match next_url {
            Some(url) => {
                let html = format!(
                    r#"<!DOCTYPE html><html><head><meta http-equiv="refresh" content="0;url={}"></head><body style="background:#121212;color:#ffffff;font-family:sans-serif;text-align:center;padding-top:60px;"><p>Connecting audio streaming engine...</p></body></html>"#,
                    url
                );
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    html.len(),
                    html
                )
            }
            None => {
                let message = include_str!("./login.html");
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    message.len(),
                    message
                )
            }
        };

        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.flush().await;
        return Ok(code);
    }
}

fn parse_query(request_line: &str) -> Result<(CsrfToken, AuthorizationCode), OAuthError> {
    let query = request_line
        .split_whitespace()
        .nth(1)
        .ok_or(OAuthError::AuthCodeListenerParse)?
        .split("?")
        .nth(1)
        .ok_or(OAuthError::AuthCodeListenerParse)?;

    let mut query_params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();

    let csrf_token = query_params
        .remove("state")
        .map(CsrfToken::new)
        .ok_or(OAuthError::CsrfTokenNotFound)?;
    let code = query_params
        .remove("code")
        .map(AuthorizationCode::new)
        .ok_or(OAuthError::AuthCodeNotFound)?;

    Ok((csrf_token, code))
}

// If the specified `redirect_uri` is HTTP, loopback, and contains a port,
// then the corresponding socket address is returned.
fn get_socket_address(redirect_uri: &str) -> Option<SocketAddr> {
    let url = match Url::parse(redirect_uri) {
        Ok(u) if u.scheme() == "http" && u.port().is_some() => u,
        _ => return None,
    };
    let socket_addr = match url.socket_addrs(|| None) {
        Ok(mut addrs) => addrs.pop(),
        _ => None,
    };
    if let Some(s) = socket_addr {
        if s.ip().is_loopback() {
            return socket_addr;
        }
    }
    None
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    use super::*;

    #[test]
    fn get_socket_address_none() {
        // No port
        assert_eq!(get_socket_address("http://127.0.0.1/foo"), None);
        assert_eq!(get_socket_address("http://127.0.0.1:/foo"), None);
        assert_eq!(get_socket_address("http://[::1]/foo"), None);
        // Not localhost
        assert_eq!(get_socket_address("http://56.0.0.1:1234/foo"), None);
        assert_eq!(
            get_socket_address("http://[3ffe:2a00:100:7031::1]:1234/foo"),
            None
        );
        // Not http
        assert_eq!(get_socket_address("https://127.0.0.1/foo"), None);
    }

    #[test]
    fn get_socket_address_localhost() {
        let localhost_v4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234);
        let localhost_v6 = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 8888);

        assert_eq!(
            get_socket_address("http://127.0.0.1:1234/foo"),
            Some(localhost_v4)
        );
        assert_eq!(
            get_socket_address("http://[0:0:0:0:0:0:0:1]:8888/foo"),
            Some(localhost_v6)
        );
        assert_eq!(
            get_socket_address("http://[::1]:8888/foo"),
            Some(localhost_v6)
        );
    }
}
