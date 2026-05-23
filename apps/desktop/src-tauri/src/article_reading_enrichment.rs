use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_ENCODING, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::archive_store::{
    ArchiveArticleInput, ArchiveStore, ArchiveStoreError, ArticleReadingCommentInput,
    ArticleReadingEnrichment, ArticleReadingEnrichmentInput,
};
use crate::secret_store::{SecretBackend, SecretSlot, SecretStore, SecretStoreError};

const MP_REFERER: &str = "https://mp.weixin.qq.com/";
const MP_COMMENT_URL: &str = "https://mp.weixin.qq.com/mp/appmsg_comment";
const MP_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 MicroMessenger/6.8.0 NetType/WIFI MiniProgramEnv/Mac MacWechat/WECHAT/WeChatBrowser";

pub type ArticleReadingEnrichmentResult<T> = Result<T, ArticleReadingEnrichmentError>;

#[derive(Debug)]
pub enum ArticleReadingEnrichmentError {
    ArchiveStore(ArchiveStoreError),
    MissingValidReadingCredential,
    SecretStore(SecretStoreError),
    Serialization(serde_json::Error),
    Transport(String),
}

impl fmt::Display for ArticleReadingEnrichmentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveStore(error) => write!(formatter, "{error}"),
            Self::MissingValidReadingCredential => write!(
                formatter,
                "a valid Article Reading Credential is required for reading enrichment"
            ),
            Self::SecretStore(error) => write!(formatter, "{error}"),
            Self::Serialization(error) => {
                write!(
                    formatter,
                    "article reading credential parse failed: {error}"
                )
            }
            Self::Transport(error) => {
                write!(formatter, "article reading enrichment failed: {error}")
            }
        }
    }
}

impl std::error::Error for ArticleReadingEnrichmentError {}

impl From<ArchiveStoreError> for ArticleReadingEnrichmentError {
    fn from(error: ArchiveStoreError) -> Self {
        Self::ArchiveStore(error)
    }
}

impl From<SecretStoreError> for ArticleReadingEnrichmentError {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

impl From<serde_json::Error> for ArticleReadingEnrichmentError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingCredential {
    pub biz: String,
    pub uin: String,
    pub key: String,
    pub pass_ticket: String,
    pub appmsg_token: String,
    pub cookie: Option<String>,
    pub expires_at_unix: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingCredentialStatus {
    pub configured: bool,
    pub valid: bool,
    pub expired: bool,
    pub expires_at_unix: Option<i64>,
}

pub struct ArticleReadingCredentialService<B> {
    secret_store: SecretStore<B>,
    now_unix: i64,
}

impl<B> ArticleReadingCredentialService<B>
where
    B: SecretBackend,
{
    pub fn new(secret_store: SecretStore<B>, now_unix: i64) -> Self {
        Self {
            secret_store,
            now_unix,
        }
    }

    pub fn status(&self) -> ArticleReadingEnrichmentResult<ArticleReadingCredentialStatus> {
        self.credential()
            .map(|credential| self.status_for_credential(credential.as_ref()))
    }

    pub fn save(
        &self,
        credential: ArticleReadingCredential,
    ) -> ArticleReadingEnrichmentResult<ArticleReadingCredentialStatus> {
        let payload = serde_json::to_string(&credential)?;
        self.secret_store
            .save(SecretSlot::ArticleReadingCredential, &payload)?;

        Ok(self.status_for_credential(Some(&credential)))
    }

    pub fn mark_expired(&self) -> ArticleReadingEnrichmentResult<ArticleReadingCredentialStatus> {
        let mut credential = self.valid_or_configured_credential()?;
        credential.expires_at_unix = self.now_unix - 1;
        self.save(credential)
    }

    pub fn delete(&self) -> ArticleReadingEnrichmentResult<ArticleReadingCredentialStatus> {
        self.secret_store
            .delete(SecretSlot::ArticleReadingCredential)?;

        Ok(self.status_for_credential(None))
    }

    fn credential(&self) -> ArticleReadingEnrichmentResult<Option<ArticleReadingCredential>> {
        self.secret_store
            .read(SecretSlot::ArticleReadingCredential)?
            .map(|payload| serde_json::from_str(&payload).map_err(Into::into))
            .transpose()
    }

    fn valid_or_configured_credential(
        &self,
    ) -> ArticleReadingEnrichmentResult<ArticleReadingCredential> {
        self.credential()?
            .ok_or(ArticleReadingEnrichmentError::MissingValidReadingCredential)
    }

    fn status_for_credential(
        &self,
        credential: Option<&ArticleReadingCredential>,
    ) -> ArticleReadingCredentialStatus {
        match credential {
            Some(credential) => {
                let expired = credential.expires_at_unix <= self.now_unix;
                ArticleReadingCredentialStatus {
                    configured: true,
                    valid: !expired,
                    expired,
                    expires_at_unix: Some(credential.expires_at_unix),
                }
            }
            None => ArticleReadingCredentialStatus {
                configured: false,
                valid: false,
                expired: false,
                expires_at_unix: None,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingEnrichmentRequest {
    pub fakeid: String,
    pub article_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArticleReadingEnrichmentFetchRequest {
    pub credential: ArticleReadingCredential,
    pub article_id: String,
    pub source_url: String,
    pub appmsgid: i64,
    pub itemidx: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingEnrichmentFetchResponse {
    pub read_count: Option<u32>,
    pub like_count: Option<u32>,
    pub share_count: Option<u32>,
    pub comment_count: Option<u32>,
    pub paid_content: Option<String>,
    pub comments: Vec<ArticleReadingCommentInput>,
}

pub trait ArticleReadingEnrichmentTransport {
    fn fetch(
        &self,
        request: ArticleReadingEnrichmentFetchRequest,
    ) -> Result<ArticleReadingEnrichmentFetchResponse, String>;
}

pub struct ArticleReadingEnrichmentClient<T, B> {
    transport: T,
    secret_store: SecretStore<B>,
    now_unix: i64,
}

impl<T, B> ArticleReadingEnrichmentClient<T, B>
where
    T: ArticleReadingEnrichmentTransport,
    B: SecretBackend,
{
    pub fn new(transport: T, secret_store: SecretStore<B>, now_unix: i64) -> Self {
        Self {
            transport,
            secret_store,
            now_unix,
        }
    }

    pub fn enrich_selected_articles(
        &self,
        archive_store: &ArchiveStore,
        request: ArticleReadingEnrichmentRequest,
    ) -> ArticleReadingEnrichmentResult<ArticleReadingEnrichmentOutcome> {
        let credential = self.valid_credential()?;
        let target_articles = archive_store
            .list_target_articles(&request.fakeid)?
            .into_iter()
            .map(|article| (article.article_id.clone(), article))
            .collect::<HashMap<_, _>>();
        let mut articles = Vec::new();
        let mut skipped_count = 0_u32;

        for article_id in request.article_ids {
            let Some(target_article) = target_articles.get(&article_id) else {
                skipped_count += 1;
                continue;
            };
            let existing = archive_store.get_article(&article_id)?;
            archive_store.upsert_article(&ArchiveArticleInput {
                article_id: target_article.article_id.clone(),
                target_account_id: target_article.target_account_id.clone(),
                title: target_article.title.clone(),
                source_url: target_article.source_url.clone(),
                html_file: existing
                    .as_ref()
                    .and_then(|article| article.html_file.clone()),
                markdown_file: existing
                    .as_ref()
                    .and_then(|article| article.markdown_file.clone()),
            })?;
            let fetched = self
                .transport
                .fetch(ArticleReadingEnrichmentFetchRequest {
                    credential: credential.clone(),
                    article_id: target_article.article_id.clone(),
                    source_url: target_article.source_url.clone(),
                    appmsgid: target_article.appmsgid,
                    itemidx: target_article.itemidx,
                })
                .map_err(ArticleReadingEnrichmentError::Transport)?;
            let input = ArticleReadingEnrichmentInput {
                read_count: fetched.read_count,
                like_count: fetched.like_count,
                share_count: fetched.share_count,
                comment_count: fetched.comment_count,
                paid_content: fetched.paid_content,
                comments: fetched.comments,
            };
            let reading_enrichment = archive_store.upsert_article_reading_enrichment(
                &target_article.article_id,
                &input,
                self.now_unix,
            )?;
            articles.push(ArticleReadingEnrichmentArticleOutcome {
                article_id: target_article.article_id.clone(),
                reading_enrichment,
            });
        }

        Ok(ArticleReadingEnrichmentOutcome {
            enriched_count: articles.len() as u32,
            skipped_count,
            articles,
        })
    }

    fn valid_credential(&self) -> ArticleReadingEnrichmentResult<ArticleReadingCredential> {
        let credential: ArticleReadingCredential = self
            .secret_store
            .read(SecretSlot::ArticleReadingCredential)?
            .map(|payload| serde_json::from_str(&payload))
            .transpose()?
            .ok_or(ArticleReadingEnrichmentError::MissingValidReadingCredential)?;
        if credential.expires_at_unix <= self.now_unix {
            return Err(ArticleReadingEnrichmentError::MissingValidReadingCredential);
        }

        Ok(credential)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingEnrichmentArticleOutcome {
    pub article_id: String,
    pub reading_enrichment: ArticleReadingEnrichment,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingEnrichmentOutcome {
    pub enriched_count: u32,
    pub skipped_count: u32,
    pub articles: Vec<ArticleReadingEnrichmentArticleOutcome>,
}

#[derive(Clone)]
pub struct WeChatArticleReadingEnrichmentTransport {
    client: Client,
}

impl Default for WeChatArticleReadingEnrichmentTransport {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::limited(10))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("create Article Reading enrichment HTTP client"),
        }
    }
}

impl ArticleReadingEnrichmentTransport for WeChatArticleReadingEnrichmentTransport {
    fn fetch(
        &self,
        request: ArticleReadingEnrichmentFetchRequest,
    ) -> Result<ArticleReadingEnrichmentFetchResponse, String> {
        let html = self.fetch_article_html(&request)?;
        let mut response = ArticleReadingEnrichmentFetchResponse {
            read_count: extract_u32_after_any(&html, &["read_num_new", "read_num"]),
            like_count: extract_u32_after_any(&html, &["old_like_count", "like_num"]),
            share_count: extract_u32_after_any(&html, &["share_count", "share_num"]),
            comment_count: extract_u32_after_any(&html, &["comment_count", "comment_num"]),
            paid_content: None,
            comments: Vec::new(),
        };

        if let Some(comment_id) = extract_comment_id(&html) {
            let comments = self.fetch_comments(&request, &comment_id)?;
            if response.comment_count.is_none() {
                response.comment_count = comments.comment_count;
            }
            response.comments = comments.comments;
        }

        Ok(response)
    }
}

impl WeChatArticleReadingEnrichmentTransport {
    fn fetch_article_html(
        &self,
        request: &ArticleReadingEnrichmentFetchRequest,
    ) -> Result<String, String> {
        let mut builder = self
            .client
            .get(&request.source_url)
            .header(REFERER, MP_REFERER)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity");
        if let Some(cookie) = request.credential.cookie.as_deref() {
            builder = builder.header("Cookie", cookie);
        }
        let response = builder.send().map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response.text().map_err(|error| error.to_string())
    }

    fn fetch_comments(
        &self,
        request: &ArticleReadingEnrichmentFetchRequest,
        comment_id: &str,
    ) -> Result<CommentFetchOutcome, String> {
        let mut builder = self
            .client
            .get(MP_COMMENT_URL)
            .header(REFERER, MP_REFERER)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .query(&[
                ("action", "getcomment".to_string()),
                ("scene", "0".to_string()),
                ("appmsgid", request.appmsgid.to_string()),
                ("idx", request.itemidx.to_string()),
                ("__biz", request.credential.biz.clone()),
                ("comment_id", comment_id.to_string()),
                ("uin", request.credential.uin.clone()),
                ("key", request.credential.key.clone()),
                ("pass_ticket", request.credential.pass_ticket.clone()),
                ("appmsg_token", request.credential.appmsg_token.clone()),
                ("wxtoken", "777".to_string()),
                ("devicetype", "UnifiedPCMac".to_string()),
                ("comment_scene", "0".to_string()),
                ("offset", "0".to_string()),
                ("limit", "100".to_string()),
                ("x5", "0".to_string()),
                ("f", "json".to_string()),
            ]);
        if let Some(cookie) = request.credential.cookie.as_deref() {
            builder = builder.header("Cookie", cookie);
        }
        let response = builder.send().map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }
        let body = response.text().map_err(|error| error.to_string())?;
        let value: Value = serde_json::from_str(&body).map_err(|error| error.to_string())?;
        let comment_count = value
            .get("elected_comment_total_cnt")
            .and_then(value_to_u32);
        let comments = value
            .get("elected_comment")
            .and_then(Value::as_array)
            .map(|comments| {
                comments
                    .iter()
                    .filter_map(comment_input_from_value)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(CommentFetchOutcome {
            comment_count,
            comments,
        })
    }
}

#[derive(Clone, Default)]
pub struct ArticleReadingEnrichmentState {
    pub transport: WeChatArticleReadingEnrichmentTransport,
}

pub fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

struct CommentFetchOutcome {
    comment_count: Option<u32>,
    comments: Vec<ArticleReadingCommentInput>,
}

fn comment_input_from_value(value: &Value) -> Option<ArticleReadingCommentInput> {
    let comment_id = value
        .get("content_id")
        .or_else(|| value.get("id"))
        .and_then(value_to_string)?;
    let author_name = value
        .get("nick_name")
        .and_then(value_to_string)
        .unwrap_or_default();
    let content = value
        .get("content")
        .and_then(value_to_string)
        .unwrap_or_default();

    Some(ArticleReadingCommentInput {
        comment_id,
        author_name,
        content,
        like_count: value.get("like_num").and_then(value_to_u32).unwrap_or(0),
        created_at_unix: value.get("create_time").and_then(value_to_i64).unwrap_or(0),
        raw_json: serde_json::to_string(value).ok(),
    })
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_to_u32(value: &Value) -> Option<u32> {
    match value {
        Value::Number(number) => number.as_u64().and_then(|value| u32::try_from(value).ok()),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn extract_comment_id(html: &str) -> Option<String> {
    extract_quoted_after(html, "var comment_id = '")
        .or_else(|| extract_quoted_after(html, "comment_id: JsDecode('"))
        .or_else(|| extract_quoted_after(html, "window.comment_id = '"))
}

fn extract_quoted_after(source: &str, marker: &str) -> Option<String> {
    let start = source.find(marker)? + marker.len();
    let end = source[start..].find('\'')?;

    Some(source[start..start + end].to_string())
}

fn extract_u32_after_any(source: &str, keys: &[&str]) -> Option<u32> {
    keys.iter().find_map(|key| extract_u32_after(source, key))
}

fn extract_u32_after(source: &str, key: &str) -> Option<u32> {
    let start = source.find(key)? + key.len();
    let tail = &source[start..source.len().min(start + 80)];
    let digit_start = tail.find(|character: char| character.is_ascii_digit())?;
    let digits = tail[digit_start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();

    digits.parse().ok()
}
