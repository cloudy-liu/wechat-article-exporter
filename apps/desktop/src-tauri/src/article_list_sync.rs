use std::fmt;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_ENCODING, ORIGIN, REFERER, USER_AGENT};
use serde::Deserialize;

use crate::archive_store::{
    ArchiveStore, ArchiveStoreError, ArticleListSyncRecord, ArticleListSyncStatus,
    CollectionTaskItemInput, CollectionTaskItemStatus, CollectionTaskType, TargetArticleAlbumInfo,
    TargetArticleInput,
};
use crate::official_account_login::OfficialAccountLoginSecret;
use crate::secret_store::{SecretBackend, SecretSlot, SecretStore, SecretStoreError};

const MP_APPMSGPUBLISH_URL: &str = "https://mp.weixin.qq.com/cgi-bin/appmsgpublish";
const MP_REFERER: &str = "https://mp.weixin.qq.com/";
const MP_ORIGIN: &str = "https://mp.weixin.qq.com";
const MP_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 WAE/1.0";
const ARTICLE_LIST_PAGE_SIZE_LIMIT: u32 = 20;

pub type ArticleListSyncResult<T> = Result<T, ArticleListSyncError>;

#[derive(Debug)]
pub enum ArticleListSyncError {
    ArchiveStore(ArchiveStoreError),
    MissingOfficialAccountLogin,
    Parse(serde_json::Error),
    SecretStore(SecretStoreError),
    Transport(String),
}

impl fmt::Display for ArticleListSyncError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveStore(error) => write!(formatter, "{error}"),
            Self::MissingOfficialAccountLogin => {
                write!(
                    formatter,
                    "Official Account Login is required before synchronizing article lists"
                )
            }
            Self::Parse(error) => write!(formatter, "article list response parse failed: {error}"),
            Self::SecretStore(error) => write!(formatter, "{error}"),
            Self::Transport(error) => {
                write!(formatter, "article list sync transport failed: {error}")
            }
        }
    }
}

impl std::error::Error for ArticleListSyncError {}

impl From<ArchiveStoreError> for ArticleListSyncError {
    fn from(error: ArchiveStoreError) -> Self {
        Self::ArchiveStore(error)
    }
}

impl From<SecretStoreError> for ArticleListSyncError {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

impl From<serde_json::Error> for ArticleListSyncError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parse(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArticleListSyncRequest {
    pub fakeid: String,
    pub begin: u32,
    pub count: u32,
    pub token: String,
    pub cookie_header: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArticleListSyncPage {
    pub total_count: Option<u32>,
    pub message_count: u32,
    pub articles: Vec<TargetArticleInput>,
}

pub trait ArticleListSyncTransport {
    fn fetch_page(&self, request: ArticleListSyncRequest) -> Result<ArticleListSyncPage, String>;
}

pub struct ArticleListSyncClient<T, B> {
    transport: T,
    secret_store: SecretStore<B>,
}

impl<T, B> ArticleListSyncClient<T, B>
where
    T: ArticleListSyncTransport,
    B: SecretBackend,
{
    pub fn new(transport: T, secret_store: SecretStore<B>) -> Self {
        Self {
            transport,
            secret_store,
        }
    }

    pub fn sync(
        &self,
        archive_store: &ArchiveStore,
        fakeid: &str,
        page_size: u32,
    ) -> ArticleListSyncResult<ArticleListSyncRecord> {
        let login_secret = self.login_secret()?;
        let page_size = page_size.clamp(1, ARTICLE_LIST_PAGE_SIZE_LIMIT);
        let mut begin = 0_u32;
        let mut total_count = None;
        let mut fetched_count = 0_u32;
        let mut requested_count = 0_u32;
        let task = archive_store.create_collection_task(
            CollectionTaskType::AccountArticleSync,
            Some(fakeid),
            vec![CollectionTaskItemInput {
                item_id: "article-list-sync".to_string(),
                item_type: "article-list-sync".to_string(),
                payload_json: serde_json::json!({
                    "fakeid": fakeid,
                    "pageSize": page_size
                })
                .to_string(),
            }],
        )?;
        archive_store.update_collection_task_item_status(
            &task.task_id,
            "article-list-sync",
            CollectionTaskItemStatus::Running,
            None,
        )?;

        loop {
            let page = match self.transport.fetch_page(ArticleListSyncRequest {
                fakeid: fakeid.to_string(),
                begin,
                count: page_size,
                token: login_secret.token.clone(),
                cookie_header: login_secret.cookie_header.clone(),
            }) {
                Ok(page) => page,
                Err(error) => {
                    archive_store.update_collection_task_item_status(
                        &task.task_id,
                        "article-list-sync",
                        CollectionTaskItemStatus::Failed,
                        Some(error.clone()),
                    )?;
                    archive_store.record_article_list_sync(
                        fakeid,
                        requested_count,
                        fetched_count,
                        total_count,
                        ArticleListSyncStatus::Failed,
                        Some(error.clone()),
                    )?;
                    return Err(ArticleListSyncError::Transport(error));
                }
            };

            total_count = total_count.or(page.total_count);
            if page.articles.is_empty() {
                break;
            }

            let message_count = page.message_count;
            for article in page.articles {
                archive_store.upsert_target_article(&article)?;
                fetched_count += 1;
            }

            requested_count += message_count;
            if message_count == 0 {
                break;
            }

            begin += message_count;
        }

        let record = archive_store.record_article_list_sync(
            fakeid,
            requested_count,
            fetched_count,
            total_count,
            ArticleListSyncStatus::Completed,
            None,
        )?;
        archive_store.update_collection_task_item_status(
            &task.task_id,
            "article-list-sync",
            CollectionTaskItemStatus::Succeeded,
            None,
        )?;

        Ok(record)
    }

    fn login_secret(&self) -> ArticleListSyncResult<OfficialAccountLoginSecret> {
        let login_secret = self
            .secret_store
            .read(SecretSlot::OfficialAccountLogin)?
            .ok_or(ArticleListSyncError::MissingOfficialAccountLogin)?;

        Ok(serde_json::from_str(&login_secret)?)
    }
}

#[derive(Clone)]
pub struct WeChatArticleListSyncTransport {
    client: Client,
}

impl Default for WeChatArticleListSyncTransport {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::limited(10))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("create Target Official Account article list HTTP client"),
        }
    }
}

impl ArticleListSyncTransport for WeChatArticleListSyncTransport {
    fn fetch_page(&self, request: ArticleListSyncRequest) -> Result<ArticleListSyncPage, String> {
        let response = self
            .client
            .get(MP_APPMSGPUBLISH_URL)
            .header(REFERER, MP_REFERER)
            .header(ORIGIN, MP_ORIGIN)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .header("Cookie", &request.cookie_header)
            .query(&[
                ("sub", "list".to_string()),
                ("search_field", "null".to_string()),
                ("begin", request.begin.to_string()),
                ("count", request.count.to_string()),
                ("query", String::new()),
                ("fakeid", request.fakeid.clone()),
                ("type", "101_1".to_string()),
                ("free_publish_type", "1".to_string()),
                ("sub_action", "list_ex".to_string()),
                ("token", request.token),
                ("lang", "zh_CN".to_string()),
                ("f", "json".to_string()),
                ("ajax", "1".to_string()),
            ])
            .send()
            .map_err(|error| error.to_string())?
            .text()
            .map_err(|error| error.to_string())?;

        parse_appmsgpublish_response(&request.fakeid, &response).map_err(|error| error.to_string())
    }
}

#[derive(Clone, Default)]
pub struct ArticleListSyncState {
    pub transport: WeChatArticleListSyncTransport,
}

pub fn parse_appmsgpublish_response(
    fakeid: &str,
    raw_response: &str,
) -> ArticleListSyncResult<ArticleListSyncPage> {
    let response: WeChatAppMsgPublishResponse = serde_json::from_str(raw_response)?;
    ensure_base_response_ok(response.base_resp.as_ref())
        .map_err(ArticleListSyncError::Transport)?;

    let publish_page = response.publish_page.unwrap_or_else(|| "{}".to_string());
    let publish_page: WeChatPublishPage = serde_json::from_str(&publish_page)?;
    let mut articles = Vec::new();
    let mut message_count = 0_u32;

    for item in publish_page.publish_list {
        if item.publish_info.trim().is_empty() {
            continue;
        }

        let publish_info: WeChatPublishInfo = serde_json::from_str(&item.publish_info)?;
        for article in publish_info.appmsgex {
            if article.itemidx.unwrap_or_default() == 1 {
                message_count += 1;
            }
            articles.push(article.into_target_article(fakeid));
        }
    }

    Ok(ArticleListSyncPage {
        total_count: publish_page.total_count,
        message_count,
        articles,
    })
}

#[derive(Deserialize)]
struct WeChatBaseResponse {
    ret: i64,
    #[serde(default)]
    err_msg: String,
}

#[derive(Deserialize)]
struct WeChatAppMsgPublishResponse {
    base_resp: Option<WeChatBaseResponse>,
    publish_page: Option<String>,
}

#[derive(Deserialize)]
struct WeChatPublishPage {
    total_count: Option<u32>,
    #[serde(default)]
    publish_list: Vec<WeChatPublishListItem>,
}

#[derive(Deserialize)]
struct WeChatPublishListItem {
    #[serde(default)]
    publish_info: String,
}

#[derive(Deserialize)]
struct WeChatPublishInfo {
    #[serde(default)]
    appmsgex: Vec<WeChatArticle>,
}

#[derive(Deserialize)]
struct WeChatArticle {
    aid: Option<String>,
    album_id: Option<String>,
    #[serde(default)]
    appmsg_album_infos: Vec<WeChatArticleAlbumInfo>,
    appmsgid: Option<i64>,
    itemidx: Option<i64>,
    title: Option<String>,
    link: Option<String>,
    digest: Option<String>,
    author_name: Option<String>,
    cover: Option<String>,
    create_time: Option<i64>,
    update_time: Option<i64>,
    item_show_type: Option<i64>,
    is_deleted: Option<bool>,
    copyright_type: Option<i64>,
}

#[derive(Deserialize)]
struct WeChatArticleAlbumInfo {
    album_id: Option<i64>,
    id: Option<String>,
    #[serde(default, rename = "tagSource")]
    tag_source: i64,
    title: Option<String>,
}

impl WeChatArticle {
    fn into_target_article(self, fakeid: &str) -> TargetArticleInput {
        let appmsgid = self.appmsgid.unwrap_or_default();
        let itemidx = self.itemidx.unwrap_or_default();
        let album_infos = self.target_article_album_infos();
        let article_id = self
            .aid
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("{appmsgid}_{itemidx}"));

        TargetArticleInput {
            article_id,
            target_account_id: fakeid.to_string(),
            title: self.title.unwrap_or_default(),
            source_url: self.link.unwrap_or_default(),
            digest: self.digest.unwrap_or_default(),
            author_name: self.author_name.unwrap_or_default(),
            cover: self.cover.unwrap_or_default(),
            appmsgid,
            itemidx,
            item_show_type: self.item_show_type.unwrap_or_default(),
            create_time: self.create_time.unwrap_or_default(),
            update_time: self.update_time.unwrap_or_default(),
            is_deleted: self.is_deleted.unwrap_or(false),
            copyright_type: self.copyright_type.unwrap_or_default(),
            album_infos,
        }
    }

    fn target_article_album_infos(&self) -> Vec<TargetArticleAlbumInfo> {
        let mut album_infos: Vec<TargetArticleAlbumInfo> = self
            .appmsg_album_infos
            .iter()
            .filter_map(WeChatArticleAlbumInfo::to_target_article_album_info)
            .collect();

        if album_infos.is_empty() {
            if let Some(album_id) = self
                .album_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                album_infos.push(TargetArticleAlbumInfo {
                    album_id: album_id.parse::<i64>().unwrap_or_default(),
                    id: album_id.to_string(),
                    tag_source: 0,
                    title: String::new(),
                });
            }
        }

        album_infos
    }
}

impl WeChatArticleAlbumInfo {
    fn to_target_article_album_info(&self) -> Option<TargetArticleAlbumInfo> {
        let id = self
            .id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| self.album_id.map(|album_id| album_id.to_string()))?;

        Some(TargetArticleAlbumInfo {
            album_id: self.album_id.unwrap_or_default(),
            id,
            tag_source: self.tag_source,
            title: self.title.clone().unwrap_or_default(),
        })
    }
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
