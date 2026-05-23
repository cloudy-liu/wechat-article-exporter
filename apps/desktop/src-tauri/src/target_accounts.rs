use std::fmt;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_ENCODING, ORIGIN, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};

use crate::archive_store::TargetAccountInput;
use crate::official_account_login::OfficialAccountLoginSecret;
use crate::secret_store::{SecretBackend, SecretSlot, SecretStore, SecretStoreError};

const MP_SEARCHBIZ_URL: &str = "https://mp.weixin.qq.com/cgi-bin/searchbiz";
const MP_REFERER: &str = "https://mp.weixin.qq.com/";
const MP_ORIGIN: &str = "https://mp.weixin.qq.com";
const MP_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 WAE/1.0";

pub type TargetAccountSearchResult<T> = Result<T, TargetAccountSearchError>;

#[derive(Debug)]
pub enum TargetAccountSearchError {
    MissingOfficialAccountLogin,
    SecretStore(SecretStoreError),
    Serialization(serde_json::Error),
    Transport(String),
}

impl fmt::Display for TargetAccountSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOfficialAccountLogin => {
                write!(
                    formatter,
                    "Official Account Login is required before searching Target Official Accounts"
                )
            }
            Self::SecretStore(error) => write!(formatter, "{error}"),
            Self::Serialization(error) => write!(formatter, "login secret parse failed: {error}"),
            Self::Transport(error) => write!(formatter, "target account search failed: {error}"),
        }
    }
}

impl std::error::Error for TargetAccountSearchError {}

impl From<SecretStoreError> for TargetAccountSearchError {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

impl From<serde_json::Error> for TargetAccountSearchError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetAccountSearchRequest {
    pub keyword: String,
    pub begin: u32,
    pub count: u32,
    pub token: String,
    pub cookie_header: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TargetAccountSearchResponse {
    pub total: u32,
    pub list: Vec<TargetAccountInput>,
}

pub trait TargetAccountSearchTransport {
    fn search(
        &self,
        request: TargetAccountSearchRequest,
    ) -> Result<TargetAccountSearchResponse, String>;
}

pub struct TargetAccountSearchClient<T, B> {
    transport: T,
    secret_store: SecretStore<B>,
}

impl<T, B> TargetAccountSearchClient<T, B>
where
    T: TargetAccountSearchTransport,
    B: SecretBackend,
{
    pub fn new(transport: T, secret_store: SecretStore<B>) -> Self {
        Self {
            transport,
            secret_store,
        }
    }

    pub fn search(
        &self,
        keyword: &str,
        begin: u32,
        count: u32,
    ) -> TargetAccountSearchResult<TargetAccountSearchResponse> {
        let login_secret = self
            .secret_store
            .read(SecretSlot::OfficialAccountLogin)?
            .ok_or(TargetAccountSearchError::MissingOfficialAccountLogin)?;
        let login_secret: OfficialAccountLoginSecret = serde_json::from_str(&login_secret)?;

        self.transport
            .search(TargetAccountSearchRequest {
                keyword: keyword.trim().to_string(),
                begin,
                count,
                token: login_secret.token,
                cookie_header: login_secret.cookie_header,
            })
            .map_err(TargetAccountSearchError::Transport)
    }
}

#[derive(Clone)]
pub struct WeChatTargetAccountSearchTransport {
    client: Client,
}

impl Default for WeChatTargetAccountSearchTransport {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::limited(10))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("create Target Official Account search HTTP client"),
        }
    }
}

impl TargetAccountSearchTransport for WeChatTargetAccountSearchTransport {
    fn search(
        &self,
        request: TargetAccountSearchRequest,
    ) -> Result<TargetAccountSearchResponse, String> {
        if request.keyword.trim().is_empty() {
            return Err("search keyword cannot be empty".to_string());
        }

        let response = self
            .client
            .get(MP_SEARCHBIZ_URL)
            .header(REFERER, MP_REFERER)
            .header(ORIGIN, MP_ORIGIN)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .header("Cookie", request.cookie_header)
            .query(&[
                ("action", "search_biz".to_string()),
                ("begin", request.begin.to_string()),
                ("count", request.count.to_string()),
                ("query", request.keyword),
                ("token", request.token),
                ("lang", "zh_CN".to_string()),
                ("f", "json".to_string()),
                ("ajax", "1".to_string()),
            ])
            .send()
            .map_err(|error| error.to_string())?;
        let body: WeChatSearchBizResponse = response.json().map_err(|error| error.to_string())?;
        ensure_base_response_ok(body.base_resp.as_ref())?;

        Ok(TargetAccountSearchResponse {
            total: body.total.unwrap_or(body.list.len() as u32),
            list: body.list,
        })
    }
}

#[derive(Clone, Default)]
pub struct TargetAccountSearchState {
    pub transport: WeChatTargetAccountSearchTransport,
}

#[derive(Deserialize)]
struct WeChatBaseResponse {
    ret: i64,
    #[serde(default)]
    err_msg: String,
}

#[derive(Deserialize)]
struct WeChatSearchBizResponse {
    base_resp: Option<WeChatBaseResponse>,
    #[serde(default)]
    list: Vec<TargetAccountInput>,
    total: Option<u32>,
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
