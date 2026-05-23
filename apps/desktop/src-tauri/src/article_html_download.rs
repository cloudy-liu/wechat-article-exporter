use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_ENCODING, CONTENT_TYPE, REFERER, USER_AGENT};
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::archive_store::{
    ArchiveArticle, ArchiveArticleInput, ArchiveStore, ArchiveStoreError, CollectionTaskItemInput,
    CollectionTaskItemStatus, CollectionTaskType, DesktopNetworkProxySetting, TargetArticleInput,
};
use crate::official_account_login::OfficialAccountLoginSecret;
use crate::secret_store::{SecretBackend, SecretSlot, SecretStore, SecretStoreError};
use crate::single_article_workflow::SINGLE_ARTICLE_TARGET_ACCOUNT_ID;

const MP_REFERER: &str = "https://mp.weixin.qq.com/";
const MP_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 WAE/1.0";
const MAX_DOWNLOAD_ATTEMPTS: usize = 3;

pub type ArticleHtmlDownloadResult<T> = Result<T, ArticleHtmlDownloadError>;

#[derive(Debug)]
pub enum ArticleHtmlDownloadError {
    ArchiveStore(ArchiveStoreError),
    ArticleNotFound { fakeid: String, article_id: String },
    Io(std::io::Error),
    MissingOfficialAccountLogin,
    Parse(serde_json::Error),
    SecretStore(SecretStoreError),
    Transport(String),
}

impl fmt::Display for ArticleHtmlDownloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveStore(error) => write!(formatter, "{error}"),
            Self::ArticleNotFound { fakeid, article_id } => write!(
                formatter,
                "target article was not found in the local archive: {fakeid}/{article_id}"
            ),
            Self::Io(error) => write!(formatter, "article html archive write failed: {error}"),
            Self::MissingOfficialAccountLogin => write!(
                formatter,
                "Official Account Login is required before downloading article HTML"
            ),
            Self::Parse(error) => write!(formatter, "article html download parse failed: {error}"),
            Self::SecretStore(error) => write!(formatter, "{error}"),
            Self::Transport(error) => {
                write!(formatter, "article html download transport failed: {error}")
            }
        }
    }
}

impl std::error::Error for ArticleHtmlDownloadError {}

impl From<ArchiveStoreError> for ArticleHtmlDownloadError {
    fn from(error: ArchiveStoreError) -> Self {
        Self::ArchiveStore(error)
    }
}

impl From<std::io::Error> for ArticleHtmlDownloadError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<SecretStoreError> for ArticleHtmlDownloadError {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

impl From<serde_json::Error> for ArticleHtmlDownloadError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parse(error)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkProxySetting {
    pub url: String,
    pub authorization: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleHtmlDownloadRequest {
    pub fakeid: String,
    pub article_id: String,
    pub proxy: Option<NetworkProxySetting>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SingleArticleHtmlDownloadRequest {
    pub article_id: String,
    pub proxy: Option<NetworkProxySetting>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArticleHtmlDownloadResourceRequest {
    pub url: String,
    pub referer: String,
    pub user_agent: String,
    pub cookie_header: Option<String>,
    pub proxy: Option<NetworkProxySetting>,
    pub is_article_html: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArticleHtmlDownloadResourceResponse {
    pub bytes: Vec<u8>,
    pub content_type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleHtmlDownloadOutcome {
    pub article_id: String,
    pub target_account_id: String,
    pub source_url: String,
    pub html_file: PathBuf,
    pub asset_files: Vec<PathBuf>,
}

pub trait ArticleHtmlDownloadTransport {
    fn fetch(
        &self,
        request: ArticleHtmlDownloadResourceRequest,
    ) -> Result<ArticleHtmlDownloadResourceResponse, String>;
}

pub struct ArticleHtmlDownloadClient<T, B> {
    transport: T,
    secret_store: SecretStore<B>,
}

impl<T, B> ArticleHtmlDownloadClient<T, B>
where
    T: ArticleHtmlDownloadTransport,
    B: SecretBackend,
{
    pub fn new(transport: T, secret_store: SecretStore<B>) -> Self {
        Self {
            transport,
            secret_store,
        }
    }

    pub fn download_article(
        &self,
        archive_store: &ArchiveStore,
        fakeid: &str,
        article_id: &str,
        proxy: Option<NetworkProxySetting>,
    ) -> ArticleHtmlDownloadResult<ArticleHtmlDownloadOutcome> {
        let login_secret = self.login_secret()?;
        let proxy = resolve_download_proxy_setting(archive_store, proxy)?;
        let article = archive_store
            .list_target_articles(fakeid)?
            .into_iter()
            .find(|article| article.article_id == article_id)
            .ok_or_else(|| ArticleHtmlDownloadError::ArticleNotFound {
                fakeid: fakeid.to_string(),
                article_id: article_id.to_string(),
            })?;
        let article = archive_article_from_target_article(article);
        let task = archive_store.create_collection_task(
            CollectionTaskType::ArticleHtmlDownload,
            Some(fakeid),
            vec![CollectionTaskItemInput {
                item_id: article.article_id.clone(),
                item_type: "article-html".to_string(),
                payload_json: serde_json::to_string(&ArticleHtmlDownloadRequest {
                    fakeid: fakeid.to_string(),
                    article_id: article.article_id.clone(),
                    proxy: proxy.clone(),
                })?,
            }],
        )?;
        archive_store.update_collection_task_item_status(
            &task.task_id,
            &article.article_id,
            CollectionTaskItemStatus::Running,
            None,
        )?;

        match self.download_archived_article(
            archive_store,
            &article,
            &login_secret.cookie_header,
            proxy,
        ) {
            Ok(outcome) => {
                archive_store.update_collection_task_item_status(
                    &task.task_id,
                    &article.article_id,
                    CollectionTaskItemStatus::Succeeded,
                    None,
                )?;
                Ok(outcome)
            }
            Err(error) => {
                archive_store.update_collection_task_item_status(
                    &task.task_id,
                    &article.article_id,
                    CollectionTaskItemStatus::Failed,
                    Some(error.to_string()),
                )?;
                Err(error)
            }
        }
    }

    pub fn download_single_article(
        &self,
        archive_store: &ArchiveStore,
        article_id: &str,
        proxy: Option<NetworkProxySetting>,
    ) -> ArticleHtmlDownloadResult<ArticleHtmlDownloadOutcome> {
        let login_secret = self.login_secret()?;
        let proxy = resolve_download_proxy_setting(archive_store, proxy)?;
        let article = archive_store.get_article(article_id)?.ok_or_else(|| {
            ArticleHtmlDownloadError::ArticleNotFound {
                fakeid: SINGLE_ARTICLE_TARGET_ACCOUNT_ID.to_string(),
                article_id: article_id.to_string(),
            }
        })?;
        let task = archive_store.create_collection_task(
            CollectionTaskType::ArticleHtmlDownload,
            Some(SINGLE_ARTICLE_TARGET_ACCOUNT_ID),
            vec![CollectionTaskItemInput {
                item_id: article.article_id.clone(),
                item_type: "single-article-html".to_string(),
                payload_json: serde_json::to_string(&SingleArticleHtmlDownloadRequest {
                    article_id: article.article_id.clone(),
                    proxy: proxy.clone(),
                })?,
            }],
        )?;
        archive_store.update_collection_task_item_status(
            &task.task_id,
            &article.article_id,
            CollectionTaskItemStatus::Running,
            None,
        )?;

        match self.download_archived_article(
            archive_store,
            &article,
            &login_secret.cookie_header,
            proxy,
        ) {
            Ok(outcome) => {
                archive_store.update_collection_task_item_status(
                    &task.task_id,
                    &article.article_id,
                    CollectionTaskItemStatus::Succeeded,
                    None,
                )?;
                Ok(outcome)
            }
            Err(error) => {
                archive_store.update_collection_task_item_status(
                    &task.task_id,
                    &article.article_id,
                    CollectionTaskItemStatus::Failed,
                    Some(error.to_string()),
                )?;
                Err(error)
            }
        }
    }

    pub fn download_archived_article(
        &self,
        archive_store: &ArchiveStore,
        article: &ArchiveArticle,
        cookie_header: &str,
        proxy: Option<NetworkProxySetting>,
    ) -> ArticleHtmlDownloadResult<ArticleHtmlDownloadOutcome> {
        download_archived_article_with_transport(
            &self.transport,
            archive_store,
            article,
            cookie_header,
            proxy,
        )
    }

    fn login_secret(&self) -> ArticleHtmlDownloadResult<OfficialAccountLoginSecret> {
        let login_secret = self
            .secret_store
            .read(SecretSlot::OfficialAccountLogin)?
            .ok_or(ArticleHtmlDownloadError::MissingOfficialAccountLogin)?;

        Ok(serde_json::from_str(&login_secret)?)
    }
}

impl From<DesktopNetworkProxySetting> for NetworkProxySetting {
    fn from(value: DesktopNetworkProxySetting) -> Self {
        Self {
            url: value.url,
            authorization: value.authorization,
        }
    }
}

pub fn resolve_download_proxy_setting(
    archive_store: &ArchiveStore,
    explicit_proxy: Option<NetworkProxySetting>,
) -> Result<Option<NetworkProxySetting>, ArchiveStoreError> {
    if explicit_proxy
        .as_ref()
        .is_some_and(|proxy| !proxy.url.trim().is_empty())
    {
        return Ok(explicit_proxy);
    }

    Ok(archive_store
        .load_or_create_settings()?
        .network_proxy
        .map(Into::into))
}

pub fn download_archived_article_with_transport<T>(
    transport: &T,
    archive_store: &ArchiveStore,
    article: &ArchiveArticle,
    cookie_header: &str,
    proxy: Option<NetworkProxySetting>,
) -> ArticleHtmlDownloadResult<ArticleHtmlDownloadOutcome>
where
    T: ArticleHtmlDownloadTransport,
{
    let html_response = fetch_with_retries(
        transport,
        ArticleHtmlDownloadResourceRequest {
            url: article.source_url.clone(),
            referer: MP_REFERER.to_string(),
            user_agent: MP_USER_AGENT.to_string(),
            cookie_header: Some(cookie_header.to_string()),
            proxy: proxy.clone(),
            is_article_html: true,
        },
    )?;
    let html = String::from_utf8_lossy(&html_response.bytes).to_string();
    let html_file = article_html_file_path(&article.target_account_id, &article.article_id);
    let html_path = archive_store.archive_dir().join(&html_file);
    if let Some(parent) = html_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&html_path, html.as_bytes())?;

    let mut asset_files = Vec::new();
    for (index, asset_url) in discover_asset_urls(&html).into_iter().enumerate() {
        let asset_response = fetch_with_retries(
            transport,
            ArticleHtmlDownloadResourceRequest {
                url: asset_url.clone(),
                referer: article.source_url.clone(),
                user_agent: MP_USER_AGENT.to_string(),
                cookie_header: Some(cookie_header.to_string()),
                proxy: proxy.clone(),
                is_article_html: false,
            },
        )?;
        let asset_file = article_asset_file_path(
            &article.target_account_id,
            &article.article_id,
            index,
            &asset_url,
            asset_response.content_type.as_deref(),
        );
        let asset_path = archive_store.archive_dir().join(&asset_file);
        if let Some(parent) = asset_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(asset_path, &asset_response.bytes)?;
        asset_files.push(asset_file);
    }

    archive_store.upsert_article(&ArchiveArticleInput {
        article_id: article.article_id.clone(),
        target_account_id: article.target_account_id.clone(),
        title: article.title.clone(),
        source_url: article.source_url.clone(),
        html_file: Some(html_file.clone()),
        markdown_file: article.markdown_file.clone(),
    })?;

    Ok(ArticleHtmlDownloadOutcome {
        article_id: article.article_id.clone(),
        target_account_id: article.target_account_id.clone(),
        source_url: article.source_url.clone(),
        html_file,
        asset_files,
    })
}

fn fetch_with_retries<T>(
    transport: &T,
    request: ArticleHtmlDownloadResourceRequest,
) -> ArticleHtmlDownloadResult<ArticleHtmlDownloadResourceResponse>
where
    T: ArticleHtmlDownloadTransport,
{
    let mut last_error = None;

    for _attempt in 0..MAX_DOWNLOAD_ATTEMPTS {
        match transport.fetch(request.clone()) {
            Ok(response) => return Ok(response),
            Err(error) => last_error = Some(error),
        }
    }

    Err(ArticleHtmlDownloadError::Transport(
        last_error.unwrap_or_else(|| "unknown transport error".to_string()),
    ))
}

fn archive_article_from_target_article(article: TargetArticleInput) -> ArchiveArticle {
    ArchiveArticle {
        article_id: article.article_id,
        target_account_id: article.target_account_id,
        title: article.title,
        source_url: article.source_url,
        html_file: None,
        markdown_file: None,
    }
}

#[derive(Clone)]
pub struct WeChatArticleHtmlDownloadTransport {
    client: Client,
}

impl Default for WeChatArticleHtmlDownloadTransport {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::limited(10))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("create article HTML download HTTP client"),
        }
    }
}

impl ArticleHtmlDownloadTransport for WeChatArticleHtmlDownloadTransport {
    fn fetch(
        &self,
        request: ArticleHtmlDownloadResourceRequest,
    ) -> Result<ArticleHtmlDownloadResourceResponse, String> {
        let client = client_for_proxy(&self.client, request.proxy.as_ref())?;
        let mut request_builder = client
            .get(&request.url)
            .header(REFERER, request.referer)
            .header(USER_AGENT, request.user_agent)
            .header(ACCEPT_ENCODING, "identity");

        if let Some(cookie_header) = request.cookie_header {
            request_builder = request_builder.header("Cookie", cookie_header);
        }
        if let Some(proxy) = request
            .proxy
            .as_ref()
            .and_then(|proxy| proxy.authorization.as_ref())
        {
            request_builder = request_builder.header("Proxy-Authorization", proxy);
        }

        let response = request_builder.send().map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let bytes = response.bytes().map_err(|error| error.to_string())?;

        Ok(ArticleHtmlDownloadResourceResponse {
            bytes: bytes.to_vec(),
            content_type,
        })
    }
}

#[derive(Clone, Default)]
pub struct ArticleHtmlDownloadState {
    pub transport: WeChatArticleHtmlDownloadTransport,
}

fn client_for_proxy(
    default_client: &Client,
    proxy: Option<&NetworkProxySetting>,
) -> Result<Client, String> {
    match proxy {
        Some(proxy) if !proxy.url.trim().is_empty() => Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(Duration::from_secs(30))
            .proxy(Proxy::all(proxy.url.trim()).map_err(|error| error.to_string())?)
            .build()
            .map_err(|error| error.to_string()),
        _ => Ok(default_client.clone()),
    }
}

fn article_html_file_path(target_account_id: &str, article_id: &str) -> PathBuf {
    Path::new("articles")
        .join(safe_path_segment(target_account_id))
        .join(format!("{}.html", safe_path_segment(article_id)))
}

fn article_asset_file_path(
    target_account_id: &str,
    article_id: &str,
    index: usize,
    asset_url: &str,
    content_type: Option<&str>,
) -> PathBuf {
    Path::new("assets")
        .join(safe_path_segment(target_account_id))
        .join(safe_path_segment(article_id))
        .join(format!(
            "{index:03}-{}",
            asset_filename(asset_url, content_type)
        ))
}

fn asset_filename(asset_url: &str, content_type: Option<&str>) -> String {
    let fallback_extension = extension_from_content_type(content_type).unwrap_or("bin");
    let filename = Url::parse(asset_url)
        .ok()
        .and_then(|url| {
            url.path_segments()
                .and_then(|mut segments| segments.next_back().map(ToOwned::to_owned))
        })
        .filter(|value| !value.trim().is_empty())
        .map(|value| safe_path_segment(&value))
        .unwrap_or_else(|| format!("asset.{fallback_extension}"));

    if filename.contains('.') {
        filename
    } else {
        format!("{filename}.{fallback_extension}")
    }
}

fn extension_from_content_type(content_type: Option<&str>) -> Option<&'static str> {
    let content_type = content_type?.split(';').next()?.trim();
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "text/css" => Some("css"),
        _ => None,
    }
}

fn discover_asset_urls(html: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for attribute in ["data-src", "src"] {
        let mut start = 0;
        let pattern = format!("{attribute}=");
        while let Some(position) = html[start..].find(&pattern) {
            let after_pattern = start + position + pattern.len();
            let Some((url, consumed)) = read_quoted_attribute_value(&html[after_pattern..]) else {
                start = after_pattern;
                continue;
            };
            if let Some(url) = normalize_asset_url(&url) {
                if !urls.contains(&url) {
                    urls.push(url);
                }
            }
            start = after_pattern + consumed;
        }
    }

    urls
}

fn read_quoted_attribute_value(input: &str) -> Option<(String, usize)> {
    let mut chars = input.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value_start = quote.len_utf8();
    let value_end = input[value_start..].find(quote)? + value_start;

    Some((
        input[value_start..value_end].to_string(),
        value_end + quote.len_utf8(),
    ))
}

fn normalize_asset_url(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        Some(trimmed.to_string())
    } else if trimmed.starts_with("//") {
        Some(format!("https:{trimmed}"))
    } else {
        None
    }
}

fn safe_path_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches(['.', '_']);

    if sanitized.is_empty() {
        "item".to_string()
    } else {
        sanitized.to_string()
    }
}
