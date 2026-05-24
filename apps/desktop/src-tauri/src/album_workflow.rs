use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT_ENCODING, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};

use crate::archive_store::{
    ArchiveArticle, ArchiveStore, ArchiveStoreError, CollectionTaskItemInput,
    CollectionTaskItemStatus, CollectionTaskType, TargetArticleAlbumInfo, TargetArticleInput,
};
use crate::article_export::{
    ArticleExportError, ArticleExportFormat, ArticleExportOutcome, ArticleExportService,
};
use crate::article_html_download::{
    download_archived_article_with_transport, resolve_download_proxy_setting,
    ArticleHtmlDownloadError, ArticleHtmlDownloadOutcome, ArticleHtmlDownloadResourceRequest,
    ArticleHtmlDownloadResourceResponse, ArticleHtmlDownloadTransport, NetworkProxySetting,
    WeChatArticleHtmlDownloadTransport,
};
use crate::official_account_login::OfficialAccountLoginSecret;
use crate::secret_store::{SecretBackend, SecretSlot, SecretStore, SecretStoreError};

const MP_APPMSGALBUM_URL: &str = "https://mp.weixin.qq.com/mp/appmsgalbum";
const MP_REFERER: &str = "https://mp.weixin.qq.com/";
const MP_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/117.0.0.0 Safari/537.36 WAE/1.0";

pub type AlbumWorkflowResult<T> = Result<T, AlbumWorkflowError>;

#[derive(Debug)]
pub enum AlbumWorkflowError {
    ArchiveStore(ArchiveStoreError),
    ArticleDownload(ArticleHtmlDownloadError),
    ArticleExport(ArticleExportError),
    ArticleNotDownloaded(String),
    MissingOfficialAccountLogin,
    Parse(serde_json::Error),
    SecretStore(SecretStoreError),
    Transport(String),
}

impl fmt::Display for AlbumWorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveStore(error) => write!(formatter, "{error}"),
            Self::ArticleDownload(error) => write!(formatter, "{error}"),
            Self::ArticleExport(error) => write!(formatter, "{error}"),
            Self::ArticleNotDownloaded(article_id) => {
                write!(
                    formatter,
                    "album article has not been downloaded: {article_id}"
                )
            }
            Self::MissingOfficialAccountLogin => write!(
                formatter,
                "Official Account Login is required before fetching album content"
            ),
            Self::Parse(error) => write!(formatter, "album response parse failed: {error}"),
            Self::SecretStore(error) => write!(formatter, "{error}"),
            Self::Transport(error) => write!(formatter, "album transport failed: {error}"),
        }
    }
}

impl std::error::Error for AlbumWorkflowError {}

impl From<ArchiveStoreError> for AlbumWorkflowError {
    fn from(error: ArchiveStoreError) -> Self {
        Self::ArchiveStore(error)
    }
}

impl From<ArticleHtmlDownloadError> for AlbumWorkflowError {
    fn from(error: ArticleHtmlDownloadError) -> Self {
        Self::ArticleDownload(error)
    }
}

impl From<ArticleExportError> for AlbumWorkflowError {
    fn from(error: ArticleExportError) -> Self {
        Self::ArticleExport(error)
    }
}

impl From<SecretStoreError> for AlbumWorkflowError {
    fn from(error: SecretStoreError) -> Self {
        Self::SecretStore(error)
    }
}

impl From<serde_json::Error> for AlbumWorkflowError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parse(error)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumPageRequest {
    pub fakeid: String,
    pub album_id: String,
    pub begin_msgid: Option<String>,
    pub begin_itemidx: Option<String>,
    pub count: u32,
    pub is_reverse: bool,
    pub cookie_header: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumBaseInfo {
    pub article_count: String,
    pub brand_icon: String,
    pub cover: String,
    pub description: String,
    pub nickname: String,
    pub title: String,
    pub username: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumPage {
    pub album_id: String,
    pub album_title: String,
    pub base_info: AlbumBaseInfo,
    pub articles: Vec<TargetArticleInput>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumFetchAllOutcome {
    pub album_id: String,
    pub album_title: String,
    pub articles: Vec<TargetArticleInput>,
    pub task_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumPageCommandRequest {
    pub fakeid: String,
    pub album_id: String,
    pub album_title: String,
    pub begin_msgid: Option<String>,
    pub begin_itemidx: Option<String>,
    pub count: u32,
    pub is_reverse: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumFetchAllCommandRequest {
    pub fakeid: String,
    pub album_id: String,
    pub album_title: String,
    pub page_size: u32,
    pub is_reverse: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDownloadRequest {
    pub fakeid: String,
    pub album_id: String,
    pub proxy: Option<NetworkProxySetting>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumExportRequest {
    pub fakeid: String,
    pub album_id: String,
    pub formats: Vec<ArticleExportFormat>,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
}

pub trait AlbumPageTransport {
    fn fetch_page(&self, request: AlbumPageRequest) -> Result<String, String>;
}

pub trait AlbumArticleDownloadTransport {
    fn fetch(
        &self,
        request: ArticleHtmlDownloadResourceRequest,
    ) -> Result<ArticleHtmlDownloadResourceResponse, String>;
}

pub struct AlbumPageClient<T, B> {
    transport: T,
    secret_store: SecretStore<B>,
}

impl<T, B> AlbumPageClient<T, B>
where
    T: AlbumPageTransport,
    B: SecretBackend,
{
    pub fn new(transport: T, secret_store: SecretStore<B>) -> Self {
        Self {
            transport,
            secret_store,
        }
    }

    pub fn fetch_page(
        &self,
        archive_store: &ArchiveStore,
        request: AlbumPageCommandRequest,
    ) -> AlbumWorkflowResult<AlbumPage> {
        let login_secret = self.login_secret()?;
        let page_request = AlbumPageRequest {
            fakeid: request.fakeid.clone(),
            album_id: request.album_id.clone(),
            begin_msgid: request.begin_msgid,
            begin_itemidx: request.begin_itemidx,
            count: request.count.max(1),
            is_reverse: request.is_reverse,
            cookie_header: login_secret.cookie_header,
        };
        let raw = self
            .transport
            .fetch_page(page_request)
            .map_err(AlbumWorkflowError::Transport)?;
        let page = parse_appmsgalbum_response(
            &request.fakeid,
            &request.album_id,
            &request.album_title,
            &raw,
        )?;
        for article in &page.articles {
            archive_store.upsert_target_article(article)?;
        }

        Ok(page)
    }

    pub fn fetch_all_articles(
        &self,
        archive_store: &ArchiveStore,
        fakeid: &str,
        album_id: &str,
        album_title: &str,
        page_size: u32,
        is_reverse: bool,
    ) -> AlbumWorkflowResult<AlbumFetchAllOutcome> {
        let login_secret = self.login_secret()?;
        let mut articles = Vec::new();
        let mut begin_msgid = None;
        let mut begin_itemidx = None;

        loop {
            let raw = self
                .transport
                .fetch_page(AlbumPageRequest {
                    fakeid: fakeid.to_string(),
                    album_id: album_id.to_string(),
                    begin_msgid: begin_msgid.clone(),
                    begin_itemidx: begin_itemidx.clone(),
                    count: page_size.max(1),
                    is_reverse,
                    cookie_header: login_secret.cookie_header.clone(),
                })
                .map_err(AlbumWorkflowError::Transport)?;
            let page = parse_appmsgalbum_response(fakeid, album_id, album_title, &raw)?;
            if page.articles.is_empty() {
                break;
            }

            for article in page.articles {
                archive_store.upsert_target_article(&article)?;
                begin_msgid = Some(article.appmsgid.to_string());
                begin_itemidx = Some(article.itemidx.to_string());
                articles.push(article);
            }

            if !page.has_more {
                break;
            }
        }

        let task = archive_store.create_collection_task(
            CollectionTaskType::AlbumDownload,
            Some(fakeid),
            articles
                .iter()
                .map(|article| CollectionTaskItemInput {
                    item_id: article.article_id.clone(),
                    item_type: "album-article-link".to_string(),
                    payload_json: serde_json::json!({
                        "fakeid": fakeid,
                        "albumId": album_id,
                        "articleId": article.article_id,
                        "sourceUrl": article.source_url
                    })
                    .to_string(),
                })
                .collect(),
        )?;
        for article in &articles {
            archive_store.update_collection_task_item_status(
                &task.task_id,
                &article.article_id,
                CollectionTaskItemStatus::Succeeded,
                None,
            )?;
        }

        Ok(AlbumFetchAllOutcome {
            album_id: album_id.to_string(),
            album_title: album_title.to_string(),
            articles,
            task_id: task.task_id,
        })
    }

    fn login_secret(&self) -> AlbumWorkflowResult<OfficialAccountLoginSecret> {
        let login_secret = self
            .secret_store
            .read(SecretSlot::OfficialAccountLogin)?
            .ok_or(AlbumWorkflowError::MissingOfficialAccountLogin)?;

        Ok(serde_json::from_str(&login_secret)?)
    }
}

pub struct AlbumDownloadClient<T, B> {
    transport: T,
    secret_store: SecretStore<B>,
}

impl<T, B> AlbumDownloadClient<T, B>
where
    T: AlbumArticleDownloadTransport,
    B: SecretBackend,
{
    pub fn new(transport: T, secret_store: SecretStore<B>) -> Self {
        Self {
            transport,
            secret_store,
        }
    }

    pub fn download_album_articles(
        &self,
        archive_store: &ArchiveStore,
        fakeid: &str,
        album_id: &str,
        proxy: Option<NetworkProxySetting>,
    ) -> AlbumWorkflowResult<Vec<ArticleHtmlDownloadOutcome>> {
        let login_secret = self.login_secret()?;
        let proxy = resolve_download_proxy_setting(archive_store, proxy)?;
        let articles = archive_store.list_target_articles_by_album(fakeid, album_id)?;
        let task = archive_store.create_collection_task(
            CollectionTaskType::AlbumDownload,
            Some(fakeid),
            articles
                .iter()
                .map(|article| CollectionTaskItemInput {
                    item_id: article.article_id.clone(),
                    item_type: "album-article-html".to_string(),
                    payload_json: serde_json::to_string(&AlbumDownloadRequest {
                        fakeid: fakeid.to_string(),
                        album_id: album_id.to_string(),
                        proxy: proxy.clone(),
                    })
                    .unwrap_or_else(|_| "{}".to_string()),
                })
                .collect(),
        )?;
        let mut outcomes = Vec::new();
        let adapter = AlbumDownloadTransportAdapter {
            transport: &self.transport,
        };

        for article in articles {
            let archive_article = archive_article_from_target_article(article);
            archive_store.update_collection_task_item_status(
                &task.task_id,
                &archive_article.article_id,
                CollectionTaskItemStatus::Running,
                None,
            )?;
            match download_archived_article_with_transport(
                &adapter,
                archive_store,
                &archive_article,
                &login_secret.cookie_header,
                proxy.clone(),
            ) {
                Ok(outcome) => {
                    archive_store.update_collection_task_item_status(
                        &task.task_id,
                        &archive_article.article_id,
                        CollectionTaskItemStatus::Succeeded,
                        None,
                    )?;
                    outcomes.push(outcome);
                }
                Err(error) => {
                    archive_store.update_collection_task_item_status(
                        &task.task_id,
                        &archive_article.article_id,
                        CollectionTaskItemStatus::Failed,
                        Some(error.to_string()),
                    )?;
                    return Err(error.into());
                }
            }
        }

        Ok(outcomes)
    }

    fn login_secret(&self) -> AlbumWorkflowResult<OfficialAccountLoginSecret> {
        let login_secret = self
            .secret_store
            .read(SecretSlot::OfficialAccountLogin)?
            .ok_or(AlbumWorkflowError::MissingOfficialAccountLogin)?;

        Ok(serde_json::from_str(&login_secret)?)
    }
}

pub struct AlbumWorkflowService;

impl AlbumWorkflowService {
    pub fn new() -> Self {
        Self
    }

    pub fn export_album_articles(
        &self,
        archive_store: &ArchiveStore,
        fakeid: &str,
        album_id: &str,
        formats: Vec<ArticleExportFormat>,
        output_dir: Option<PathBuf>,
    ) -> AlbumWorkflowResult<Vec<ArticleExportOutcome>> {
        if formats.is_empty() {
            return Err(ArticleExportError::EmptyFormatList.into());
        }

        let target_articles = archive_store.list_target_articles_by_album(fakeid, album_id)?;
        let task = archive_store.create_collection_task(
            CollectionTaskType::Export,
            Some(fakeid),
            target_articles
                .iter()
                .map(|article| CollectionTaskItemInput {
                    item_id: article.article_id.clone(),
                    item_type: "album-article-export".to_string(),
                    payload_json: serde_json::to_string(&AlbumExportRequest {
                        fakeid: fakeid.to_string(),
                        album_id: album_id.to_string(),
                        formats: formats.clone(),
                        output_dir: output_dir.clone(),
                    })
                    .unwrap_or_else(|_| "{}".to_string()),
                })
                .collect(),
        )?;
        let export_service = ArticleExportService::new();
        let mut outcomes = Vec::new();

        for target_article in target_articles {
            let article = archive_store
                .get_article(&target_article.article_id)?
                .ok_or_else(|| {
                    AlbumWorkflowError::ArticleNotDownloaded(target_article.article_id.clone())
                })?;
            archive_store.update_collection_task_item_status(
                &task.task_id,
                &article.article_id,
                CollectionTaskItemStatus::Running,
                None,
            )?;
            match export_service.export_archived_article_with_task(
                archive_store,
                &article,
                &formats,
                &task.task_id,
                None,
                output_dir.as_deref(),
            ) {
                Ok(outcome) => {
                    archive_store.update_collection_task_item_status(
                        &task.task_id,
                        &article.article_id,
                        CollectionTaskItemStatus::Succeeded,
                        None,
                    )?;
                    outcomes.push(outcome);
                }
                Err(error) => {
                    archive_store.update_collection_task_item_status(
                        &task.task_id,
                        &article.article_id,
                        CollectionTaskItemStatus::Failed,
                        Some(error.to_string()),
                    )?;
                    return Err(error.into());
                }
            }
        }

        Ok(outcomes)
    }
}

#[derive(Clone)]
pub struct WeChatAlbumPageTransport {
    client: Client,
}

impl Default for WeChatAlbumPageTransport {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .redirect(reqwest::redirect::Policy::limited(10))
                .timeout(Duration::from_secs(30))
                .build()
                .expect("create album page HTTP client"),
        }
    }
}

impl AlbumPageTransport for WeChatAlbumPageTransport {
    fn fetch_page(&self, request: AlbumPageRequest) -> Result<String, String> {
        let mut query = vec![
            ("action", "getalbum".to_string()),
            ("__biz", request.fakeid),
            ("album_id", request.album_id),
            ("count", request.count.to_string()),
            (
                "is_reverse",
                if request.is_reverse { "1" } else { "0" }.to_string(),
            ),
            ("f", "json".to_string()),
        ];
        if let Some(begin_msgid) = request.begin_msgid {
            query.push(("begin_msgid", begin_msgid));
        }
        if let Some(begin_itemidx) = request.begin_itemidx {
            query.push(("begin_itemidx", begin_itemidx));
        }

        self.client
            .get(MP_APPMSGALBUM_URL)
            .header(REFERER, MP_REFERER)
            .header(USER_AGENT, MP_USER_AGENT)
            .header(ACCEPT_ENCODING, "identity")
            .header("Cookie", request.cookie_header)
            .query(&query)
            .send()
            .map_err(|error| error.to_string())?
            .text()
            .map_err(|error| error.to_string())
    }
}

#[derive(Clone, Default)]
pub struct WeChatAlbumArticleDownloadTransport {
    inner: WeChatArticleHtmlDownloadTransport,
}

impl AlbumArticleDownloadTransport for WeChatAlbumArticleDownloadTransport {
    fn fetch(
        &self,
        request: ArticleHtmlDownloadResourceRequest,
    ) -> Result<ArticleHtmlDownloadResourceResponse, String> {
        self.inner.fetch(request)
    }
}

#[derive(Clone, Default)]
pub struct AlbumWorkflowState {
    pub page_transport: WeChatAlbumPageTransport,
    pub download_transport: WeChatAlbumArticleDownloadTransport,
}

struct AlbumDownloadTransportAdapter<'a, T> {
    transport: &'a T,
}

impl<T> ArticleHtmlDownloadTransport for AlbumDownloadTransportAdapter<'_, T>
where
    T: AlbumArticleDownloadTransport,
{
    fn fetch(
        &self,
        request: ArticleHtmlDownloadResourceRequest,
    ) -> Result<ArticleHtmlDownloadResourceResponse, String> {
        self.transport.fetch(request)
    }
}

pub fn parse_appmsgalbum_response(
    fakeid: &str,
    album_id: &str,
    album_title: &str,
    raw_response: &str,
) -> AlbumWorkflowResult<AlbumPage> {
    let response: WeChatAlbumResponse = serde_json::from_str(raw_response)?;
    ensure_base_response_ok(response.base_resp.as_ref()).map_err(AlbumWorkflowError::Transport)?;
    let album_response = response.getalbum_resp;
    let base_info = album_response.base_info.unwrap_or_default();
    let resolved_album_title = if base_info.title.trim().is_empty() {
        album_title.to_string()
    } else {
        base_info.title.clone()
    };
    let article_album_info = TargetArticleAlbumInfo {
        album_id: album_id.parse::<i64>().unwrap_or_default(),
        id: album_id.to_string(),
        tag_source: 0,
        title: resolved_album_title.clone(),
    };
    let articles = album_response
        .article_list
        .into_items()
        .into_iter()
        .map(|article| article.into_target_article(fakeid, article_album_info.clone()))
        .collect();

    Ok(AlbumPage {
        album_id: album_id.to_string(),
        album_title: resolved_album_title,
        base_info: base_info.into_album_base_info(),
        articles,
        has_more: album_response.continue_flag.trim() != "0",
    })
}

#[derive(Deserialize)]
struct WeChatBaseResponse {
    ret: i64,
    #[serde(default)]
    err_msg: String,
}

#[derive(Deserialize)]
struct WeChatAlbumResponse {
    base_resp: Option<WeChatBaseResponse>,
    #[serde(default)]
    getalbum_resp: WeChatGetAlbumResponse,
}

#[derive(Default, Deserialize)]
struct WeChatGetAlbumResponse {
    #[serde(default)]
    article_list: WeChatAlbumArticleList,
    #[serde(default)]
    base_info: Option<WeChatAlbumBaseInfo>,
    #[serde(default)]
    continue_flag: String,
}

#[derive(Default, Deserialize)]
#[serde(untagged)]
enum WeChatAlbumArticleList {
    Many(Vec<WeChatAlbumArticle>),
    One(WeChatAlbumArticle),
    #[default]
    Empty,
}

impl WeChatAlbumArticleList {
    fn into_items(self) -> Vec<WeChatAlbumArticle> {
        match self {
            Self::Many(items) => items,
            Self::One(item) => vec![item],
            Self::Empty => Vec::new(),
        }
    }
}

#[derive(Clone, Default, Deserialize)]
struct WeChatAlbumBaseInfo {
    #[serde(default)]
    article_count: String,
    #[serde(default)]
    brand_icon: String,
    #[serde(default)]
    cover: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    nickname: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    username: String,
}

impl WeChatAlbumBaseInfo {
    fn into_album_base_info(self) -> AlbumBaseInfo {
        AlbumBaseInfo {
            article_count: self.article_count,
            brand_icon: self.brand_icon,
            cover: self.cover,
            description: self.description,
            nickname: self.nickname,
            title: self.title,
            username: self.username,
        }
    }
}

#[derive(Clone, Default, Deserialize)]
struct WeChatAlbumArticle {
    #[serde(default)]
    cover_img_1_1: String,
    #[serde(default)]
    create_time: String,
    #[serde(default)]
    item_show_type: String,
    #[serde(default)]
    itemidx: String,
    #[serde(default)]
    msgid: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
}

impl WeChatAlbumArticle {
    fn into_target_article(
        self,
        fakeid: &str,
        album_info: TargetArticleAlbumInfo,
    ) -> TargetArticleInput {
        let appmsgid = self.msgid.parse::<i64>().unwrap_or_default();
        let itemidx = self.itemidx.parse::<i64>().unwrap_or_default();

        TargetArticleInput {
            article_id: format!("{appmsgid}_{itemidx}"),
            target_account_id: fakeid.to_string(),
            title: self.title,
            source_url: self.url,
            digest: String::new(),
            author_name: String::new(),
            cover: self.cover_img_1_1,
            appmsgid,
            itemidx,
            item_show_type: self.item_show_type.parse::<i64>().unwrap_or_default(),
            create_time: self.create_time.parse::<i64>().unwrap_or_default(),
            update_time: self.create_time.parse::<i64>().unwrap_or_default(),
            is_deleted: false,
            copyright_type: 0,
            album_infos: vec![album_info],
        }
    }
}

fn archive_article_from_target_article(article: TargetArticleInput) -> ArchiveArticle {
    ArchiveArticle {
        article_id: article.article_id,
        target_account_id: article.target_account_id,
        title: article.title,
        source_url: article.source_url,
        html_file: None,
        markdown_file: None,
        reading_enrichment: None,
        reading_comments: Vec::new(),
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
