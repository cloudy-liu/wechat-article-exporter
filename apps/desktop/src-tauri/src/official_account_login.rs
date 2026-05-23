use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT_ENCODING, CONTENT_TYPE, ORIGIN, REFERER, SET_COOKIE, USER_AGENT};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::secret_store::{SecretBackend, SecretSlot, SecretStore, SecretStoreError};

const MP_BASE_URL: &str = "https://mp.weixin.qq.com";
const MP_REFERER: &str = "https://mp.weixin.qq.com/";
const MP_ORIGIN: &str = "https://mp.weixin.qq.com";
const MP_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 WAE/1.0";
const OFFICIAL_ACCOUNT_LOGIN_MESSAGE: &str =
    "Scan this QR code with a WeChat Official Account operator account.";

pub type OfficialAccountLoginResult<T> = Result<T, OfficialAccountLoginError>;

#[derive(Debug)]
pub enum OfficialAccountLoginError {
    InvalidResponse(String),
    LoginSessionNotFound(String),
    SecretStore(SecretStoreError),
    Serialization(serde_json::Error),
    Transport(String),
}

impl fmt::Display for OfficialAccountLoginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidResponse(message) => {
                write!(formatter, "invalid WeChat login response: {message}")
            }
            Self::LoginSessionNotFound(session_id) => {
                write!(
                    formatter,
                    "login session was not found or has expired: {session_id}"
                )
            }
            Self::SecretStore(error) => write!(formatter, "{error}"),
            Self::Serialization(error) => {
                write!(formatter, "login secret serialization failed: {error}")
            }
            Self::Transport(message) => {
                write!(formatter, "WeChat login transport failed: {message}")
            }
        }
    }
}

impl std::error::Error for OfficialAccountLoginError {}

impl From<SecretStoreError> for OfficialAccountLoginError {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

impl From<serde_json::Error> for OfficialAccountLoginError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialAccountLoginSession {
    pub session_id: String,
    pub qr_code_data_url: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartLoginPayload {
    pub session_id: String,
    pub qr_code_data_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginPollPayload {
    pub status: LoginScanStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginFinalizePayload {
    pub secret: OfficialAccountLoginSecret,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialAccountLoginAccount {
    pub nickname: String,
    pub avatar_url: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialAccountLoginSecret {
    pub token: String,
    pub cookie_header: String,
    pub profile: OfficialAccountLoginProfile,
    pub expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialAccountLoginProfile {
    pub nickname: String,
    pub avatar_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LoginScanStatus {
    Waiting,
    Scanned { account_count: u32 },
    Confirmed { account_count: u32 },
    Expired,
    Error { message: String },
}

pub trait OfficialAccountLoginTransport {
    fn start_login(&self) -> Result<StartLoginPayload, String>;
    fn poll_scan_status(&self, session_id: &str) -> Result<LoginPollPayload, String>;
    fn finalize_login(&self, session_id: &str) -> Result<LoginFinalizePayload, String>;
}

#[derive(Clone, Default)]
pub struct InMemoryLoginSessionStore {
    sessions: Arc<Mutex<HashSet<String>>>,
}

impl InMemoryLoginSessionStore {
    fn insert(&self, session_id: &str) -> OfficialAccountLoginResult<()> {
        self.sessions
            .lock()
            .map_err(|_| {
                OfficialAccountLoginError::Transport(
                    "login session store lock was poisoned".to_string(),
                )
            })?
            .insert(session_id.to_string());

        Ok(())
    }

    fn contains(&self, session_id: &str) -> OfficialAccountLoginResult<bool> {
        Ok(self
            .sessions
            .lock()
            .map_err(|_| {
                OfficialAccountLoginError::Transport(
                    "login session store lock was poisoned".to_string(),
                )
            })?
            .contains(session_id))
    }

    fn remove(&self, session_id: &str) -> OfficialAccountLoginResult<()> {
        self.sessions
            .lock()
            .map_err(|_| {
                OfficialAccountLoginError::Transport(
                    "login session store lock was poisoned".to_string(),
                )
            })?
            .remove(session_id);

        Ok(())
    }
}

pub struct OfficialAccountLoginClient<T, B> {
    transport: T,
    session_store: InMemoryLoginSessionStore,
    secret_store: SecretStore<B>,
}

impl<T, B> OfficialAccountLoginClient<T, B>
where
    T: OfficialAccountLoginTransport,
    B: SecretBackend,
{
    pub fn new(
        transport: T,
        session_store: InMemoryLoginSessionStore,
        secret_store: SecretStore<B>,
    ) -> Self {
        Self {
            transport,
            session_store,
            secret_store,
        }
    }

    pub fn start_login(&self) -> OfficialAccountLoginResult<OfficialAccountLoginSession> {
        let payload = self
            .transport
            .start_login()
            .map_err(OfficialAccountLoginError::Transport)?;
        self.session_store.insert(&payload.session_id)?;

        Ok(OfficialAccountLoginSession {
            session_id: payload.session_id,
            qr_code_data_url: payload.qr_code_data_url,
            message: OFFICIAL_ACCOUNT_LOGIN_MESSAGE.to_string(),
        })
    }

    pub fn poll_scan_status(
        &self,
        session_id: &str,
    ) -> OfficialAccountLoginResult<LoginScanStatus> {
        self.ensure_session_exists(session_id)?;

        let payload = self
            .transport
            .poll_scan_status(session_id)
            .map_err(OfficialAccountLoginError::Transport)?;

        Ok(payload.status)
    }

    pub fn finalize_login(
        &self,
        session_id: &str,
    ) -> OfficialAccountLoginResult<OfficialAccountLoginAccount> {
        self.ensure_session_exists(session_id)?;

        let payload = self
            .transport
            .finalize_login(session_id)
            .map_err(OfficialAccountLoginError::Transport)?;
        let account = OfficialAccountLoginAccount {
            nickname: payload.secret.profile.nickname.clone(),
            avatar_url: payload.secret.profile.avatar_url.clone(),
            expires_at: payload.secret.expires_at.clone(),
        };
        let serialized_secret = serde_json::to_string(&payload.secret)?;
        self.secret_store
            .save(SecretSlot::OfficialAccountLogin, &serialized_secret)?;
        self.session_store.remove(session_id)?;

        Ok(account)
    }

    pub fn logout(&self) -> OfficialAccountLoginResult<()> {
        self.secret_store.delete(SecretSlot::OfficialAccountLogin)?;

        Ok(())
    }

    fn ensure_session_exists(&self, session_id: &str) -> OfficialAccountLoginResult<()> {
        if self.session_store.contains(session_id)? {
            Ok(())
        } else {
            Err(OfficialAccountLoginError::LoginSessionNotFound(
                session_id.to_string(),
            ))
        }
    }
}

#[derive(Clone)]
pub struct WeChatOfficialAccountLoginTransport {
    client: Client,
    session_cookies: Arc<Mutex<HashMap<String, String>>>,
}

impl Default for WeChatOfficialAccountLoginTransport {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::limited(10))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("create WeChat Official Account login HTTP client"),
            session_cookies: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl OfficialAccountLoginTransport for WeChatOfficialAccountLoginTransport {
    fn start_login(&self) -> Result<StartLoginPayload, String> {
        let session_id = generate_session_id();
        let start_response = self
            .client
            .post(format!("{MP_BASE_URL}/cgi-bin/bizlogin?action=startlogin"))
            .header(REFERER, MP_REFERER)
            .header(ORIGIN, MP_ORIGIN)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .form(&[
                ("userlang", "zh_CN"),
                ("redirect_url", ""),
                ("login_type", "3"),
                ("sessionid", session_id.as_str()),
                ("token", ""),
                ("lang", "zh_CN"),
                ("f", "json"),
                ("ajax", "1"),
            ])
            .send()
            .map_err(|error| error.to_string())?;
        let start_cookies = cookie_header_from_response(&start_response);
        let start_body: WeChatStartLoginResponse =
            start_response.json().map_err(|error| error.to_string())?;
        ensure_base_response_ok(start_body.base_resp.as_ref())?;
        if start_cookies.is_empty() {
            return Err("WeChat did not return a login session cookie".to_string());
        }
        self.set_session_cookie(&session_id, start_cookies.clone())?;

        let qr_response = self
            .client
            .get(format!("{MP_BASE_URL}/cgi-bin/scanloginqrcode"))
            .header(REFERER, MP_REFERER)
            .header(ORIGIN, MP_ORIGIN)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .header("Cookie", start_cookies)
            .query(&[
                ("action", "getqrcode"),
                ("random", &current_unix_millis().to_string()),
            ])
            .send()
            .map_err(|error| error.to_string())?;
        let content_type = qr_response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("image/png")
            .split(';')
            .next()
            .unwrap_or("image/png")
            .to_string();
        let qr_bytes = qr_response.bytes().map_err(|error| error.to_string())?;
        let qr_code_data_url = format!(
            "data:{content_type};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(qr_bytes)
        );

        Ok(StartLoginPayload {
            session_id,
            qr_code_data_url,
        })
    }

    fn poll_scan_status(&self, session_id: &str) -> Result<LoginPollPayload, String> {
        let cookie_header = self.session_cookie(session_id)?;
        let response = self
            .client
            .get(format!("{MP_BASE_URL}/cgi-bin/scanloginqrcode"))
            .header(REFERER, MP_REFERER)
            .header(ORIGIN, MP_ORIGIN)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .header("Cookie", cookie_header)
            .query(&[
                ("action", "ask"),
                ("token", ""),
                ("lang", "zh_CN"),
                ("f", "json"),
                ("ajax", "1"),
            ])
            .send()
            .map_err(|error| error.to_string())?;
        let scan: WeChatScanResponse = response.json().map_err(|error| error.to_string())?;
        let status = match ensure_base_response_ok(scan.base_resp.as_ref()) {
            Ok(()) => map_scan_status(scan.status, scan.acct_size),
            Err(error) => LoginScanStatus::Error { message: error },
        };

        Ok(LoginPollPayload { status })
    }

    fn finalize_login(&self, session_id: &str) -> Result<LoginFinalizePayload, String> {
        let cookie_header = self.session_cookie(session_id)?;
        let response = self
            .client
            .post(format!("{MP_BASE_URL}/cgi-bin/bizlogin?action=login"))
            .header(REFERER, MP_REFERER)
            .header(ORIGIN, MP_ORIGIN)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .header("Cookie", &cookie_header)
            .form(&[
                ("userlang", "zh_CN"),
                ("redirect_url", ""),
                ("cookie_forbidden", "0"),
                ("cookie_cleaned", "0"),
                ("plugin_used", "0"),
                ("login_type", "3"),
                ("token", ""),
                ("lang", "zh_CN"),
                ("f", "json"),
                ("ajax", "1"),
            ])
            .send()
            .map_err(|error| error.to_string())?;
        let login_cookie_header = merged_cookie_header(&cookie_header, &response);
        let login_body: WeChatBizLoginResponse =
            response.json().map_err(|error| error.to_string())?;
        ensure_base_response_ok(login_body.base_resp.as_ref())?;
        let redirect_url = login_body
            .redirect_url
            .ok_or_else(|| "login response did not include redirect_url".to_string())?;
        let token = token_from_redirect_url(&redirect_url)?;
        let profile = self.fetch_profile(&token, &login_cookie_header)?;
        self.remove_session_cookie(session_id)?;

        Ok(LoginFinalizePayload {
            secret: OfficialAccountLoginSecret {
                token,
                cookie_header: login_cookie_header,
                profile,
                expires_at: four_days_from_now(),
            },
        })
    }
}

impl WeChatOfficialAccountLoginTransport {
    fn fetch_profile(
        &self,
        token: &str,
        cookie_header: &str,
    ) -> Result<OfficialAccountLoginProfile, String> {
        let html = self
            .client
            .get(format!("{MP_BASE_URL}/cgi-bin/home"))
            .header(REFERER, MP_REFERER)
            .header(ORIGIN, MP_ORIGIN)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .header("Cookie", cookie_header)
            .query(&[("t", "home/index"), ("token", token), ("lang", "zh_CN")])
            .send()
            .map_err(|error| error.to_string())?
            .text()
            .map_err(|error| error.to_string())?;

        let nickname = extract_quoted_js_value(&html, "wx.cgiData.nick_name")
            .ok_or_else(|| "could not extract Official Account nickname".to_string())?;
        let avatar_url = extract_quoted_js_value(&html, "wx.cgiData.head_img").unwrap_or_default();

        Ok(OfficialAccountLoginProfile {
            nickname,
            avatar_url,
        })
    }

    fn set_session_cookie(&self, session_id: &str, cookie_header: String) -> Result<(), String> {
        self.session_cookies
            .lock()
            .map_err(|_| "login cookie store lock was poisoned".to_string())?
            .insert(session_id.to_string(), cookie_header);

        Ok(())
    }

    fn session_cookie(&self, session_id: &str) -> Result<String, String> {
        self.session_cookies
            .lock()
            .map_err(|_| "login cookie store lock was poisoned".to_string())?
            .get(session_id)
            .cloned()
            .ok_or_else(|| format!("login session cookie was not found: {session_id}"))
    }

    fn remove_session_cookie(&self, session_id: &str) -> Result<(), String> {
        self.session_cookies
            .lock()
            .map_err(|_| "login cookie store lock was poisoned".to_string())?
            .remove(session_id);

        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct OfficialAccountLoginState {
    pub transport: WeChatOfficialAccountLoginTransport,
    pub session_store: InMemoryLoginSessionStore,
}

#[derive(Deserialize)]
struct WeChatBaseResponse {
    ret: i64,
    #[serde(default)]
    err_msg: String,
}

#[derive(Deserialize)]
struct WeChatStartLoginResponse {
    base_resp: Option<WeChatBaseResponse>,
}

#[derive(Deserialize)]
struct WeChatScanResponse {
    base_resp: Option<WeChatBaseResponse>,
    status: Option<i64>,
    #[serde(default)]
    acct_size: u32,
}

#[derive(Deserialize)]
struct WeChatBizLoginResponse {
    base_resp: Option<WeChatBaseResponse>,
    redirect_url: Option<String>,
}

fn ensure_base_response_ok(base_resp: Option<&WeChatBaseResponse>) -> Result<(), String> {
    match base_resp {
        Some(response) if response.ret == 0 => Ok(()),
        Some(response) => Err(if response.err_msg.is_empty() {
            format!("WeChat returned ret={}", response.ret)
        } else {
            response.err_msg.clone()
        }),
        None => Err("WeChat response did not include base_resp".to_string()),
    }
}

fn map_scan_status(status: Option<i64>, account_count: u32) -> LoginScanStatus {
    match status {
        Some(0) => LoginScanStatus::Waiting,
        Some(1) => LoginScanStatus::Confirmed { account_count },
        Some(2) | Some(3) => LoginScanStatus::Expired,
        Some(4) | Some(6) => LoginScanStatus::Scanned { account_count },
        Some(5) => LoginScanStatus::Error {
            message: "This WeChat account is not eligible for Official Account platform login."
                .to_string(),
        },
        Some(value) => LoginScanStatus::Error {
            message: format!("unsupported WeChat scan status: {value}"),
        },
        None => LoginScanStatus::Error {
            message: "WeChat scan response did not include status".to_string(),
        },
    }
}

fn token_from_redirect_url(redirect_url: &str) -> Result<String, String> {
    let url = if redirect_url.starts_with("http://") || redirect_url.starts_with("https://") {
        Url::parse(redirect_url).map_err(|error| error.to_string())?
    } else {
        Url::parse(&format!("http://localhost{redirect_url}")).map_err(|error| error.to_string())?
    };

    url.query_pairs()
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("redirect_url did not include token: {redirect_url}"))
}

fn cookie_header_from_response(response: &Response) -> String {
    let mut values = Vec::new();
    for value in response.headers().get_all(SET_COOKIE) {
        if let Ok(cookie) = value.to_str() {
            if let Some(name_value) = cookie.split(';').next() {
                if !name_value.ends_with("=EXPIRED") && name_value.contains('=') {
                    values.push(name_value.trim().to_string());
                }
            }
        }
    }

    values.join("; ")
}

fn merged_cookie_header(existing_cookie_header: &str, response: &Response) -> String {
    let response_cookie_header = cookie_header_from_response(response);
    if response_cookie_header.is_empty() {
        existing_cookie_header.to_string()
    } else if existing_cookie_header.is_empty() {
        response_cookie_header
    } else {
        format!("{existing_cookie_header}; {response_cookie_header}")
    }
}

fn extract_quoted_js_value(html: &str, key: &str) -> Option<String> {
    let key_position = html.find(key)?;
    let after_key = &html[key_position + key.len()..];
    let equals_position = after_key.find('=')?;
    let after_equals = after_key[equals_position + 1..].trim_start();
    let after_open_quote = after_equals.strip_prefix('"')?;
    let close_quote_position = after_open_quote.find('"')?;

    Some(after_open_quote[..close_quote_position].to_string())
}

fn generate_session_id() -> String {
    format!("desktop-{}", current_unix_millis())
}

fn current_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn four_days_from_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 4 * 24 * 60 * 60;

    format!("unix:{seconds}")
}
