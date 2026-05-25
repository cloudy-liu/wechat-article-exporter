use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: i64 = 7;
const DEFAULT_ARCHIVE_DIR_NAME: &str = "archive";
const DEFAULT_DATABASE_FILE_NAME: &str = "archive.sqlite";
const TARGET_ACCOUNT_EXPORT_FORMAT: &str = "wechat-article-exporter.target-accounts.v1";
static COLLECTION_TASK_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub type ArchiveStoreResult<T> = Result<T, ArchiveStoreError>;

#[derive(Debug)]
pub enum ArchiveStoreError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    InvalidRelativePath(PathBuf),
    MissingSettings,
    NonUtf8Path(PathBuf),
    InvalidArticleListSyncStatus(String),
    InvalidCollectionTaskItemStatus(String),
    InvalidCollectionTaskStatus(String),
    InvalidCollectionTaskType(String),
    UnsupportedTargetAccountExportFormat(String),
    Json(serde_json::Error),
}

impl fmt::Display for ArchiveStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "archive store filesystem error: {error}"),
            Self::Sqlite(error) => write!(formatter, "archive store sqlite error: {error}"),
            Self::InvalidRelativePath(path) => {
                write!(
                    formatter,
                    "archive file reference must be relative: {}",
                    path.display()
                )
            }
            Self::MissingSettings => {
                write!(formatter, "archive store settings have not been saved")
            }
            Self::NonUtf8Path(path) => {
                write!(
                    formatter,
                    "archive store path must be valid UTF-8: {}",
                    path.display()
                )
            }
            Self::InvalidArticleListSyncStatus(status) => {
                write!(formatter, "invalid article list sync status: {status}")
            }
            Self::InvalidCollectionTaskItemStatus(status) => {
                write!(formatter, "invalid collection task item status: {status}")
            }
            Self::InvalidCollectionTaskStatus(status) => {
                write!(formatter, "invalid collection task status: {status}")
            }
            Self::InvalidCollectionTaskType(task_type) => {
                write!(formatter, "invalid collection task type: {task_type}")
            }
            Self::UnsupportedTargetAccountExportFormat(format) => {
                write!(
                    formatter,
                    "unsupported target account export format: {format}"
                )
            }
            Self::Json(error) => write!(formatter, "archive store JSON error: {error}"),
        }
    }
}

impl std::error::Error for ArchiveStoreError {}

impl From<std::io::Error> for ArchiveStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for ArchiveStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for ArchiveStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveStoreConfig {
    pub database_path: PathBuf,
    pub archive_dir: PathBuf,
}

impl ArchiveStoreConfig {
    pub fn from_app_data_dir(app_data_dir: impl AsRef<Path>) -> Self {
        let app_data_dir = app_data_dir.as_ref();

        Self {
            database_path: app_data_dir.join(DEFAULT_DATABASE_FILE_NAME),
            archive_dir: app_data_dir.join(DEFAULT_ARCHIVE_DIR_NAME),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopExportSettings {
    pub markdown: bool,
    pub html: bool,
}

impl Default for DesktopExportSettings {
    fn default() -> Self {
        Self {
            markdown: true,
            html: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSyncDownloadSettings {
    pub history_limit: u32,
    pub page_size: u32,
    pub download_concurrency: u32,
}

impl Default for DesktopSyncDownloadSettings {
    fn default() -> Self {
        Self {
            history_limit: 20,
            page_size: 20,
            download_concurrency: 2,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopNetworkProxySetting {
    pub url: String,
    pub authorization: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveStoreSettings {
    pub archive_dir: PathBuf,
    pub export: DesktopExportSettings,
    pub sync_download: DesktopSyncDownloadSettings,
    pub network_proxy: Option<DesktopNetworkProxySetting>,
}

impl ArchiveStoreSettings {
    pub fn default_for_archive_dir(archive_dir: PathBuf) -> Self {
        Self {
            archive_dir,
            export: DesktopExportSettings::default(),
            sync_download: DesktopSyncDownloadSettings::default(),
            network_proxy: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArchiveStoreSnapshot {
    pub database_path: PathBuf,
    pub archive_dir: PathBuf,
    pub schema_version: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveArticleInput {
    pub article_id: String,
    pub target_account_id: String,
    pub title: String,
    pub source_url: String,
    pub html_file: Option<PathBuf>,
    pub markdown_file: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveArticle {
    pub article_id: String,
    pub target_account_id: String,
    pub title: String,
    pub source_url: String,
    pub html_file: Option<PathBuf>,
    pub markdown_file: Option<PathBuf>,
    pub reading_enrichment: Option<ArticleReadingEnrichment>,
    pub reading_comments: Vec<ArticleReadingComment>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingEnrichmentInput {
    pub read_count: Option<u32>,
    pub like_count: Option<u32>,
    pub share_count: Option<u32>,
    pub comment_count: Option<u32>,
    pub paid_content: Option<String>,
    pub comments: Vec<ArticleReadingCommentInput>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingEnrichment {
    pub read_count: Option<u32>,
    pub like_count: Option<u32>,
    pub share_count: Option<u32>,
    pub comment_count: Option<u32>,
    pub paid_content: Option<String>,
    pub enriched_at_unix: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingCommentInput {
    pub comment_id: String,
    pub author_name: String,
    pub content: String,
    pub like_count: u32,
    pub created_at_unix: i64,
    pub raw_json: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleReadingComment {
    pub article_id: String,
    pub comment_id: String,
    pub author_name: String,
    pub content: String,
    pub like_count: u32,
    pub created_at_unix: i64,
    pub raw_json: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TargetAccountInput {
    pub fakeid: String,
    pub nickname: String,
    pub alias: String,
    pub round_head_img: String,
    pub service_type: i64,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TargetAccountExport {
    pub format: String,
    pub accounts: Vec<TargetAccountInput>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TargetArticleAlbumInfo {
    pub album_id: i64,
    pub id: String,
    pub tag_source: i64,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TargetArticleInput {
    pub article_id: String,
    pub target_account_id: String,
    pub title: String,
    pub source_url: String,
    pub digest: String,
    pub author_name: String,
    pub cover: String,
    pub appmsgid: i64,
    pub itemidx: i64,
    pub item_show_type: i64,
    pub create_time: i64,
    pub update_time: i64,
    pub is_deleted: bool,
    pub copyright_type: i64,
    pub album_infos: Vec<TargetArticleAlbumInfo>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArticleListSyncStatus {
    Running,
    Completed,
    Failed,
}

impl ArticleListSyncStatus {
    fn as_database_value(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    fn from_database_value(value: &str) -> ArchiveStoreResult<Self> {
        match value {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(ArchiveStoreError::InvalidArticleListSyncStatus(
                value.to_string(),
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArticleListSyncRecord {
    pub target_account_id: String,
    pub requested_limit: u32,
    pub fetched_count: u32,
    pub total_count: Option<u32>,
    pub status: ArticleListSyncStatus,
    pub error_message: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CollectionTaskType {
    AccountArticleSync,
    ArticleHtmlDownload,
    AlbumDownload,
    Export,
}

impl CollectionTaskType {
    fn as_database_value(self) -> &'static str {
        match self {
            Self::AccountArticleSync => "account_article_sync",
            Self::ArticleHtmlDownload => "article_html_download",
            Self::AlbumDownload => "album_download",
            Self::Export => "export",
        }
    }

    fn from_database_value(value: &str) -> ArchiveStoreResult<Self> {
        match value {
            "account_article_sync" => Ok(Self::AccountArticleSync),
            "article_html_download" => Ok(Self::ArticleHtmlDownload),
            "album_download" => Ok(Self::AlbumDownload),
            "export" => Ok(Self::Export),
            _ => Err(ArchiveStoreError::InvalidCollectionTaskType(
                value.to_string(),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CollectionTaskStatus {
    Waiting,
    Running,
    Paused,
    Cancelled,
    Succeeded,
    Failed,
}

impl CollectionTaskStatus {
    fn as_database_value(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Cancelled => "cancelled",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    fn from_database_value(value: &str) -> ArchiveStoreResult<Self> {
        match value {
            "waiting" => Ok(Self::Waiting),
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "cancelled" => Ok(Self::Cancelled),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(ArchiveStoreError::InvalidCollectionTaskStatus(
                value.to_string(),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CollectionTaskItemStatus {
    Waiting,
    Running,
    Cancelled,
    Succeeded,
    Failed,
}

impl CollectionTaskItemStatus {
    fn as_database_value(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Running => "running",
            Self::Cancelled => "cancelled",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    fn from_database_value(value: &str) -> ArchiveStoreResult<Self> {
        match value {
            "waiting" => Ok(Self::Waiting),
            "running" => Ok(Self::Running),
            "cancelled" => Ok(Self::Cancelled),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(ArchiveStoreError::InvalidCollectionTaskItemStatus(
                value.to_string(),
            )),
        }
    }

    fn is_attempt_status(self) -> bool {
        matches!(self, Self::Running | Self::Succeeded | Self::Failed)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CollectionTaskItemInput {
    pub item_id: String,
    pub item_type: String,
    pub payload_json: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CollectionTaskItem {
    pub task_id: String,
    pub item_id: String,
    pub item_type: String,
    pub status: CollectionTaskItemStatus,
    pub payload_json: String,
    pub error_message: Option<String>,
    pub attempts: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CollectionTask {
    pub task_id: String,
    pub task_type: CollectionTaskType,
    pub target_account_id: Option<String>,
    pub status: CollectionTaskStatus,
    pub total_items: u32,
    pub waiting_items: u32,
    pub running_items: u32,
    pub succeeded_items: u32,
    pub failed_items: u32,
    pub cancelled_items: u32,
    pub error_message: Option<String>,
    pub items: Vec<CollectionTaskItem>,
}

#[derive(Clone, Copy, Debug, Default)]
struct CollectionTaskCounts {
    total: u32,
    waiting: u32,
    running: u32,
    succeeded: u32,
    failed: u32,
    cancelled: u32,
}

pub struct ArchiveStore {
    database_path: PathBuf,
    archive_dir: PathBuf,
    connection: Connection,
}

impl ArchiveStore {
    pub fn open(config: ArchiveStoreConfig) -> ArchiveStoreResult<Self> {
        if let Some(parent) = config.database_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::create_dir_all(&config.archive_dir)?;
        fs::create_dir_all(config.archive_dir.join("articles"))?;
        fs::create_dir_all(config.archive_dir.join("assets"))?;
        fs::create_dir_all(config.archive_dir.join("exports"))?;

        let connection = Connection::open(&config.database_path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;

        let store = Self {
            database_path: config.database_path,
            archive_dir: config.archive_dir,
            connection,
        };
        store.migrate()?;

        Ok(store)
    }

    pub fn initialize_from_app_data_dir(
        app_data_dir: impl AsRef<Path>,
    ) -> ArchiveStoreResult<ArchiveStoreSnapshot> {
        let store = Self::open(ArchiveStoreConfig::from_app_data_dir(app_data_dir))?;

        store.snapshot()
    }

    pub fn snapshot(&self) -> ArchiveStoreResult<ArchiveStoreSnapshot> {
        Ok(ArchiveStoreSnapshot {
            database_path: self.database_path.clone(),
            archive_dir: self.archive_dir.clone(),
            schema_version: self.schema_version()?,
        })
    }

    pub fn archive_dir(&self) -> &Path {
        &self.archive_dir
    }

    pub fn schema_version(&self) -> ArchiveStoreResult<i64> {
        Ok(self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?)
    }

    pub fn save_settings(&self, settings: &ArchiveStoreSettings) -> ArchiveStoreResult<()> {
        let archive_dir = if settings.archive_dir.as_os_str().is_empty() {
            self.archive_dir.clone()
        } else {
            settings.archive_dir.clone()
        };

        fs::create_dir_all(&archive_dir)?;
        fs::create_dir_all(archive_dir.join("articles"))?;
        fs::create_dir_all(archive_dir.join("assets"))?;
        fs::create_dir_all(archive_dir.join("exports"))?;

        let proxy_url = settings
            .network_proxy
            .as_ref()
            .map(|proxy| proxy.url.trim())
            .filter(|url| !url.is_empty())
            .unwrap_or_default()
            .to_string();
        let proxy_authorization = settings
            .network_proxy
            .as_ref()
            .and_then(|proxy| proxy.authorization.as_deref())
            .map(str::trim)
            .filter(|authorization| !authorization.is_empty())
            .map(ToOwned::to_owned);

        self.connection.execute(
            r#"
            INSERT INTO app_settings (
              id,
              archive_dir,
              export_markdown,
              export_html,
              sync_history_limit,
              sync_page_size,
              download_concurrency,
              network_proxy_url,
              network_proxy_authorization,
              updated_at
            )
            VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
              archive_dir = excluded.archive_dir,
              export_markdown = excluded.export_markdown,
              export_html = excluded.export_html,
              sync_history_limit = excluded.sync_history_limit,
              sync_page_size = excluded.sync_page_size,
              download_concurrency = excluded.download_concurrency,
              network_proxy_url = excluded.network_proxy_url,
              network_proxy_authorization = excluded.network_proxy_authorization,
              updated_at = excluded.updated_at
            "#,
            params![
                path_to_database_text(&archive_dir)?,
                bool_to_database_int(settings.export.markdown),
                bool_to_database_int(settings.export.html),
                i64::from(settings.sync_download.history_limit.max(1)),
                i64::from(settings.sync_download.page_size.max(1)),
                i64::from(settings.sync_download.download_concurrency.max(1)),
                proxy_url,
                proxy_authorization,
            ],
        )?;

        Ok(())
    }

    pub fn load_settings(&self) -> ArchiveStoreResult<ArchiveStoreSettings> {
        let settings = self
            .connection
            .query_row(
                r#"
                SELECT
                  archive_dir,
                  export_markdown,
                  export_html,
                  sync_history_limit,
                  sync_page_size,
                  download_concurrency,
                  network_proxy_url,
                  network_proxy_authorization
                FROM app_settings
                WHERE id = 1
                "#,
                [],
                settings_from_row,
            )
            .optional()?
            .ok_or(ArchiveStoreError::MissingSettings)?;

        Ok(settings)
    }

    pub fn load_or_create_settings(&self) -> ArchiveStoreResult<ArchiveStoreSettings> {
        match self.load_settings() {
            Ok(settings) => Ok(settings),
            Err(ArchiveStoreError::MissingSettings) => {
                let settings =
                    ArchiveStoreSettings::default_for_archive_dir(self.archive_dir.clone());
                self.save_settings(&settings)?;
                Ok(settings)
            }
            Err(error) => Err(error),
        }
    }

    pub fn upsert_article(&self, article: &ArchiveArticleInput) -> ArchiveStoreResult<()> {
        let html_file = optional_relative_path_to_text(article.html_file.as_deref())?;
        let markdown_file = optional_relative_path_to_text(article.markdown_file.as_deref())?;

        self.connection.execute(
            r#"
            INSERT INTO article_archives (
              article_id,
              target_account_id,
              title,
              source_url,
              html_file,
              markdown_file,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
            ON CONFLICT(article_id) DO UPDATE SET
              target_account_id = excluded.target_account_id,
              title = excluded.title,
              source_url = excluded.source_url,
              html_file = excluded.html_file,
              markdown_file = excluded.markdown_file,
              updated_at = excluded.updated_at
            "#,
            params![
                article.article_id,
                article.target_account_id,
                article.title,
                article.source_url,
                html_file,
                markdown_file
            ],
        )?;

        Ok(())
    }

    pub fn get_article(&self, article_id: &str) -> ArchiveStoreResult<Option<ArchiveArticle>> {
        let article = self
            .connection
            .query_row(
                r#"
                SELECT
                  article_id,
                  target_account_id,
                  title,
                  source_url,
                  html_file,
                  markdown_file,
                  reading_read_count,
                  reading_like_count,
                  reading_share_count,
                  reading_comment_count,
                  reading_paid_content,
                  reading_enriched_at_unix
                FROM article_archives
                WHERE article_id = ?1
                "#,
                params![article_id],
                archive_article_from_row,
            )
            .optional()?;

        article
            .map(|article| self.attach_reading_comments(article))
            .transpose()
    }

    pub fn list_articles_by_target_account(
        &self,
        target_account_id: &str,
    ) -> ArchiveStoreResult<Vec<ArchiveArticle>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT
              article_id,
              target_account_id,
              title,
              source_url,
              html_file,
              markdown_file,
              reading_read_count,
              reading_like_count,
              reading_share_count,
              reading_comment_count,
              reading_paid_content,
              reading_enriched_at_unix
            FROM article_archives
            WHERE target_account_id = ?1
            ORDER BY updated_at DESC, created_at DESC, article_id
            "#,
        )?;
        let articles = statement
            .query_map(params![target_account_id], archive_article_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        articles
            .into_iter()
            .map(|article| self.attach_reading_comments(article))
            .collect()
    }

    pub fn upsert_article_reading_enrichment(
        &self,
        article_id: &str,
        enrichment: &ArticleReadingEnrichmentInput,
        enriched_at_unix: i64,
    ) -> ArchiveStoreResult<ArticleReadingEnrichment> {
        let updated_rows = self.connection.execute(
            r#"
            UPDATE article_archives
            SET
              reading_read_count = ?2,
              reading_like_count = ?3,
              reading_share_count = ?4,
              reading_comment_count = ?5,
              reading_paid_content = ?6,
              reading_enriched_at_unix = ?7,
              updated_at = CURRENT_TIMESTAMP
            WHERE article_id = ?1
            "#,
            params![
                article_id,
                optional_u32_to_database_i64(enrichment.read_count),
                optional_u32_to_database_i64(enrichment.like_count),
                optional_u32_to_database_i64(enrichment.share_count),
                optional_u32_to_database_i64(enrichment.comment_count),
                enrichment.paid_content,
                enriched_at_unix
            ],
        )?;
        if updated_rows == 0 {
            return Err(ArchiveStoreError::Sqlite(
                rusqlite::Error::QueryReturnedNoRows,
            ));
        }

        self.connection.execute(
            "DELETE FROM article_reading_comments WHERE article_id = ?1",
            params![article_id],
        )?;
        for comment in &enrichment.comments {
            self.connection.execute(
                r#"
                INSERT INTO article_reading_comments (
                  article_id,
                  comment_id,
                  author_name,
                  content,
                  like_count,
                  created_at_unix,
                  raw_json,
                  updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)
                ON CONFLICT(article_id, comment_id) DO UPDATE SET
                  author_name = excluded.author_name,
                  content = excluded.content,
                  like_count = excluded.like_count,
                  created_at_unix = excluded.created_at_unix,
                  raw_json = excluded.raw_json,
                  updated_at = excluded.updated_at
                "#,
                params![
                    article_id,
                    comment.comment_id,
                    comment.author_name,
                    comment.content,
                    i64::from(comment.like_count),
                    comment.created_at_unix,
                    comment.raw_json
                ],
            )?;
        }

        Ok(ArticleReadingEnrichment {
            read_count: enrichment.read_count,
            like_count: enrichment.like_count,
            share_count: enrichment.share_count,
            comment_count: enrichment.comment_count,
            paid_content: enrichment.paid_content.clone(),
            enriched_at_unix,
        })
    }

    fn attach_reading_comments(
        &self,
        mut article: ArchiveArticle,
    ) -> ArchiveStoreResult<ArchiveArticle> {
        article.reading_comments = self.list_article_reading_comments(&article.article_id)?;

        Ok(article)
    }

    fn list_article_reading_comments(
        &self,
        article_id: &str,
    ) -> ArchiveStoreResult<Vec<ArticleReadingComment>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT
              article_id,
              comment_id,
              author_name,
              content,
              like_count,
              created_at_unix,
              raw_json
            FROM article_reading_comments
            WHERE article_id = ?1
            ORDER BY created_at_unix DESC, comment_id
            "#,
        )?;
        let comments = statement
            .query_map(params![article_id], |row| {
                let like_count = row.get::<_, i64>(4)?;

                Ok(ArticleReadingComment {
                    article_id: row.get(0)?,
                    comment_id: row.get(1)?,
                    author_name: row.get(2)?,
                    content: row.get(3)?,
                    like_count: database_i64_to_u32(like_count, 0),
                    created_at_unix: row.get(5)?,
                    raw_json: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(comments)
    }

    pub fn upsert_target_account(&self, account: &TargetAccountInput) -> ArchiveStoreResult<()> {
        self.connection.execute(
            r#"
            INSERT INTO target_accounts (
              fakeid,
              nickname,
              alias,
              round_head_img,
              service_type,
              signature,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
            ON CONFLICT(fakeid) DO UPDATE SET
              nickname = excluded.nickname,
              alias = excluded.alias,
              round_head_img = excluded.round_head_img,
              service_type = excluded.service_type,
              signature = excluded.signature,
              updated_at = excluded.updated_at
            "#,
            params![
                account.fakeid,
                account.nickname,
                account.alias,
                account.round_head_img,
                account.service_type,
                account.signature
            ],
        )?;

        Ok(())
    }

    pub fn list_target_accounts(&self) -> ArchiveStoreResult<Vec<TargetAccountInput>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT fakeid, nickname, alias, round_head_img, service_type, signature
            FROM target_accounts
            ORDER BY nickname COLLATE NOCASE, fakeid
            "#,
        )?;
        let accounts = statement
            .query_map([], |row| {
                Ok(TargetAccountInput {
                    fakeid: row.get(0)?,
                    nickname: row.get(1)?,
                    alias: row.get(2)?,
                    round_head_img: row.get(3)?,
                    service_type: row.get(4)?,
                    signature: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(accounts)
    }

    pub fn delete_target_account(&self, fakeid: &str) -> ArchiveStoreResult<()> {
        self.connection.execute(
            "DELETE FROM target_accounts WHERE fakeid = ?1",
            params![fakeid],
        )?;

        Ok(())
    }

    pub fn export_target_accounts(&self) -> ArchiveStoreResult<TargetAccountExport> {
        Ok(TargetAccountExport {
            format: TARGET_ACCOUNT_EXPORT_FORMAT.to_string(),
            accounts: self.list_target_accounts()?,
        })
    }

    pub fn import_target_accounts(&self, export: &TargetAccountExport) -> ArchiveStoreResult<()> {
        if export.format != TARGET_ACCOUNT_EXPORT_FORMAT {
            return Err(ArchiveStoreError::UnsupportedTargetAccountExportFormat(
                export.format.clone(),
            ));
        }

        for account in &export.accounts {
            self.upsert_target_account(account)?;
        }

        Ok(())
    }

    pub fn upsert_target_article(&self, article: &TargetArticleInput) -> ArchiveStoreResult<()> {
        let album_infos_json = serde_json::to_string(&article.album_infos)?;
        self.connection.execute(
            r#"
            INSERT INTO target_articles (
              article_id,
              target_account_id,
              title,
              source_url,
              digest,
              author_name,
              cover,
              appmsgid,
              itemidx,
              item_show_type,
              create_time,
              update_time,
              is_deleted,
              copyright_type,
              album_infos_json,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, CURRENT_TIMESTAMP)
            ON CONFLICT(target_account_id, article_id) DO UPDATE SET
              title = excluded.title,
              source_url = excluded.source_url,
              digest = excluded.digest,
              author_name = excluded.author_name,
              cover = excluded.cover,
              appmsgid = excluded.appmsgid,
              itemidx = excluded.itemidx,
              item_show_type = excluded.item_show_type,
              create_time = excluded.create_time,
              update_time = excluded.update_time,
              is_deleted = excluded.is_deleted,
              copyright_type = excluded.copyright_type,
              album_infos_json = excluded.album_infos_json,
              updated_at = excluded.updated_at
            "#,
            params![
                article.article_id,
                article.target_account_id,
                article.title,
                article.source_url,
                article.digest,
                article.author_name,
                article.cover,
                article.appmsgid,
                article.itemidx,
                article.item_show_type,
                article.create_time,
                article.update_time,
                article.is_deleted,
                article.copyright_type,
                album_infos_json
            ],
        )?;

        Ok(())
    }

    pub fn list_target_articles(
        &self,
        target_account_id: &str,
    ) -> ArchiveStoreResult<Vec<TargetArticleInput>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT
              article_id,
              target_account_id,
              title,
              source_url,
              digest,
              author_name,
              cover,
              appmsgid,
              itemidx,
              item_show_type,
              create_time,
              update_time,
              is_deleted,
              copyright_type,
              album_infos_json
            FROM target_articles
            WHERE target_account_id = ?1
            ORDER BY create_time DESC, update_time DESC, article_id
            "#,
        )?;
        let articles = statement
            .query_map(params![target_account_id], |row| {
                Ok(TargetArticleInput {
                    article_id: row.get(0)?,
                    target_account_id: row.get(1)?,
                    title: row.get(2)?,
                    source_url: row.get(3)?,
                    digest: row.get(4)?,
                    author_name: row.get(5)?,
                    cover: row.get(6)?,
                    appmsgid: row.get(7)?,
                    itemidx: row.get(8)?,
                    item_show_type: row.get(9)?,
                    create_time: row.get(10)?,
                    update_time: row.get(11)?,
                    is_deleted: row.get(12)?,
                    copyright_type: row.get(13)?,
                    album_infos: parse_target_article_album_infos(&row.get::<_, String>(14)?)
                        .map_err(to_from_sql_conversion_failure(14))?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(articles)
    }

    pub fn list_target_account_albums(
        &self,
        target_account_id: &str,
    ) -> ArchiveStoreResult<Vec<TargetArticleAlbumInfo>> {
        let mut albums = Vec::new();

        for article in self.list_target_articles(target_account_id)? {
            for album in article.album_infos {
                if !album.id.trim().is_empty()
                    && !albums
                        .iter()
                        .any(|existing: &TargetArticleAlbumInfo| existing.id == album.id)
                {
                    albums.push(album);
                }
            }
        }

        Ok(albums)
    }

    pub fn list_target_articles_by_album(
        &self,
        target_account_id: &str,
        album_id: &str,
    ) -> ArchiveStoreResult<Vec<TargetArticleInput>> {
        Ok(self
            .list_target_articles(target_account_id)?
            .into_iter()
            .filter(|article| {
                article.album_infos.iter().any(|album| {
                    album.id == album_id
                        || (album.id.is_empty() && album.album_id.to_string() == album_id)
                })
            })
            .collect())
    }

    pub fn record_article_list_sync(
        &self,
        target_account_id: &str,
        requested_limit: u32,
        fetched_count: u32,
        total_count: Option<u32>,
        status: ArticleListSyncStatus,
        error_message: Option<String>,
    ) -> ArchiveStoreResult<ArticleListSyncRecord> {
        self.connection.execute(
            r#"
            INSERT INTO article_list_sync_runs (
              target_account_id,
              requested_limit,
              fetched_count,
              total_count,
              status,
              error_message
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                target_account_id,
                i64::from(requested_limit),
                i64::from(fetched_count),
                total_count.map(i64::from),
                status.as_database_value(),
                error_message
            ],
        )?;

        Ok(ArticleListSyncRecord {
            target_account_id: target_account_id.to_string(),
            requested_limit,
            fetched_count,
            total_count,
            status,
            error_message,
        })
    }

    pub fn latest_article_list_sync(
        &self,
        target_account_id: &str,
    ) -> ArchiveStoreResult<Option<ArticleListSyncRecord>> {
        let record = self
            .connection
            .query_row(
                r#"
                SELECT
                  target_account_id,
                  requested_limit,
                  fetched_count,
                  total_count,
                  status,
                  error_message
                FROM article_list_sync_runs
                WHERE target_account_id = ?1
                ORDER BY id DESC
                LIMIT 1
                "#,
                params![target_account_id],
                |row| {
                    let status = row.get::<_, String>(4)?;
                    let requested_limit = row.get::<_, i64>(1)?;
                    let fetched_count = row.get::<_, i64>(2)?;
                    let total_count = row.get::<_, Option<i64>>(3)?;

                    Ok(ArticleListSyncRecord {
                        target_account_id: row.get(0)?,
                        requested_limit: requested_limit as u32,
                        fetched_count: fetched_count as u32,
                        total_count: total_count.map(|value| value as u32),
                        status: ArticleListSyncStatus::from_database_value(&status).map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    4,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            },
                        )?,
                        error_message: row.get(5)?,
                    })
                },
            )
            .optional()?;

        Ok(record)
    }

    pub fn create_collection_task(
        &self,
        task_type: CollectionTaskType,
        target_account_id: Option<&str>,
        items: Vec<CollectionTaskItemInput>,
    ) -> ArchiveStoreResult<CollectionTask> {
        let task_id = generate_collection_task_id();
        self.connection.execute(
            r#"
            INSERT INTO collection_tasks (
              task_id,
              task_type,
              target_account_id,
              status,
              total_items,
              waiting_items,
              running_items,
              succeeded_items,
              failed_items,
              cancelled_items
            )
            VALUES (?1, ?2, ?3, ?4, 0, 0, 0, 0, 0, 0)
            "#,
            params![
                task_id,
                task_type.as_database_value(),
                target_account_id,
                CollectionTaskStatus::Waiting.as_database_value()
            ],
        )?;

        for item in items {
            self.connection.execute(
                r#"
                INSERT INTO collection_task_items (
                  task_id,
                  item_id,
                  item_type,
                  status,
                  payload_json
                )
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    task_id,
                    item.item_id,
                    item.item_type,
                    CollectionTaskItemStatus::Waiting.as_database_value(),
                    item.payload_json
                ],
            )?;
        }

        self.roll_up_collection_task(&task_id)
    }

    pub fn list_collection_tasks(&self) -> ArchiveStoreResult<Vec<CollectionTask>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT task_id
            FROM collection_tasks
            ORDER BY updated_at DESC, created_at DESC, task_id DESC
            "#,
        )?;
        let task_ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);

        task_ids
            .iter()
            .map(|task_id| {
                self.get_collection_task(task_id)?
                    .ok_or_else(|| ArchiveStoreError::from(rusqlite::Error::QueryReturnedNoRows))
            })
            .collect()
    }

    pub fn get_collection_task(&self, task_id: &str) -> ArchiveStoreResult<Option<CollectionTask>> {
        let task = self
            .connection
            .query_row(
                r#"
                SELECT
                  task_id,
                  task_type,
                  target_account_id,
                  status,
                  total_items,
                  waiting_items,
                  running_items,
                  succeeded_items,
                  failed_items,
                  cancelled_items,
                  error_message
                FROM collection_tasks
                WHERE task_id = ?1
                "#,
                params![task_id],
                collection_task_from_row,
            )
            .optional()?;

        match task {
            Some(mut task) => {
                task.items = self.list_collection_task_items(&task.task_id)?;
                Ok(Some(task))
            }
            None => Ok(None),
        }
    }

    pub fn update_collection_task_item_status(
        &self,
        task_id: &str,
        item_id: &str,
        status: CollectionTaskItemStatus,
        error_message: Option<String>,
    ) -> ArchiveStoreResult<CollectionTask> {
        let (previous_status, previous_attempts) = self.connection.query_row(
            r#"
            SELECT status, attempts
            FROM collection_task_items
            WHERE task_id = ?1 AND item_id = ?2
            "#,
            params![task_id, item_id],
            |row| {
                let status = row.get::<_, String>(0)?;
                let status = CollectionTaskItemStatus::from_database_value(&status)
                    .map_err(to_from_sql_conversion_failure(0))?;
                let attempts = row.get::<_, i64>(1)?;

                Ok((status, attempts))
            },
        )?;
        let attempts = if status.is_attempt_status() && !previous_status.is_attempt_status() {
            previous_attempts + 1
        } else {
            previous_attempts
        };

        self.connection.execute(
            r#"
            UPDATE collection_task_items
            SET
              status = ?3,
              error_message = ?4,
              attempts = ?5,
              updated_at = CURRENT_TIMESTAMP
            WHERE task_id = ?1 AND item_id = ?2
            "#,
            params![
                task_id,
                item_id,
                status.as_database_value(),
                error_message,
                attempts
            ],
        )?;

        self.roll_up_collection_task(task_id)
    }

    pub fn retry_failed_collection_task_items(
        &self,
        task_id: &str,
    ) -> ArchiveStoreResult<CollectionTask> {
        self.connection.execute(
            r#"
            UPDATE collection_task_items
            SET
              status = ?2,
              error_message = NULL,
              updated_at = CURRENT_TIMESTAMP
            WHERE task_id = ?1 AND status = ?3
            "#,
            params![
                task_id,
                CollectionTaskItemStatus::Waiting.as_database_value(),
                CollectionTaskItemStatus::Failed.as_database_value()
            ],
        )?;

        self.roll_up_collection_task(task_id)
    }

    pub fn set_collection_task_status(
        &self,
        task_id: &str,
        status: CollectionTaskStatus,
        error_message: Option<String>,
    ) -> ArchiveStoreResult<CollectionTask> {
        if status == CollectionTaskStatus::Cancelled {
            self.connection.execute(
                r#"
                UPDATE collection_task_items
                SET
                  status = ?2,
                  error_message = NULL,
                  updated_at = CURRENT_TIMESTAMP
                WHERE task_id = ?1 AND status IN (?3, ?4)
                "#,
                params![
                    task_id,
                    CollectionTaskItemStatus::Cancelled.as_database_value(),
                    CollectionTaskItemStatus::Waiting.as_database_value(),
                    CollectionTaskItemStatus::Running.as_database_value()
                ],
            )?;
        }

        let counts = self.collection_task_counts(task_id)?;
        self.update_collection_task_summary(task_id, status, counts, error_message)?;
        self.get_collection_task(task_id)?
            .ok_or_else(|| ArchiveStoreError::from(rusqlite::Error::QueryReturnedNoRows))
    }

    fn list_collection_task_items(
        &self,
        task_id: &str,
    ) -> ArchiveStoreResult<Vec<CollectionTaskItem>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT
              task_id,
              item_id,
              item_type,
              status,
              payload_json,
              error_message,
              attempts
            FROM collection_task_items
            WHERE task_id = ?1
            ORDER BY rowid
            "#,
        )?;
        let items = statement
            .query_map(params![task_id], |row| {
                let status = row.get::<_, String>(3)?;
                let attempts = row.get::<_, i64>(6)?;

                Ok(CollectionTaskItem {
                    task_id: row.get(0)?,
                    item_id: row.get(1)?,
                    item_type: row.get(2)?,
                    status: CollectionTaskItemStatus::from_database_value(&status)
                        .map_err(to_from_sql_conversion_failure(3))?,
                    payload_json: row.get(4)?,
                    error_message: row.get(5)?,
                    attempts: attempts as u32,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    fn roll_up_collection_task(&self, task_id: &str) -> ArchiveStoreResult<CollectionTask> {
        let counts = self.collection_task_counts(task_id)?;
        let status = infer_collection_task_status(counts);
        let error_message = self.latest_collection_task_error(task_id)?;

        self.update_collection_task_summary(task_id, status, counts, error_message)?;
        self.get_collection_task(task_id)?
            .ok_or_else(|| ArchiveStoreError::from(rusqlite::Error::QueryReturnedNoRows))
    }

    fn collection_task_counts(&self, task_id: &str) -> ArchiveStoreResult<CollectionTaskCounts> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT status, COUNT(*)
            FROM collection_task_items
            WHERE task_id = ?1
            GROUP BY status
            "#,
        )?;
        let rows = statement
            .query_map(params![task_id], |row| {
                let status = row.get::<_, String>(0)?;
                let count = row.get::<_, i64>(1)?;
                let status = CollectionTaskItemStatus::from_database_value(&status)
                    .map_err(to_from_sql_conversion_failure(0))?;

                Ok((status, count as u32))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut counts = CollectionTaskCounts::default();
        for (status, count) in rows {
            counts.total += count;
            match status {
                CollectionTaskItemStatus::Waiting => counts.waiting += count,
                CollectionTaskItemStatus::Running => counts.running += count,
                CollectionTaskItemStatus::Succeeded => counts.succeeded += count,
                CollectionTaskItemStatus::Failed => counts.failed += count,
                CollectionTaskItemStatus::Cancelled => counts.cancelled += count,
            }
        }

        Ok(counts)
    }

    fn latest_collection_task_error(&self, task_id: &str) -> ArchiveStoreResult<Option<String>> {
        Ok(self
            .connection
            .query_row(
                r#"
                SELECT error_message
                FROM collection_task_items
                WHERE task_id = ?1
                  AND status = ?2
                  AND error_message IS NOT NULL
                ORDER BY updated_at DESC, rowid DESC
                LIMIT 1
                "#,
                params![
                    task_id,
                    CollectionTaskItemStatus::Failed.as_database_value()
                ],
                |row| row.get(0),
            )
            .optional()?)
    }

    fn update_collection_task_summary(
        &self,
        task_id: &str,
        status: CollectionTaskStatus,
        counts: CollectionTaskCounts,
        error_message: Option<String>,
    ) -> ArchiveStoreResult<()> {
        self.connection.execute(
            r#"
            UPDATE collection_tasks
            SET
              status = ?2,
              total_items = ?3,
              waiting_items = ?4,
              running_items = ?5,
              succeeded_items = ?6,
              failed_items = ?7,
              cancelled_items = ?8,
              error_message = ?9,
              updated_at = CURRENT_TIMESTAMP
            WHERE task_id = ?1
            "#,
            params![
                task_id,
                status.as_database_value(),
                i64::from(counts.total),
                i64::from(counts.waiting),
                i64::from(counts.running),
                i64::from(counts.succeeded),
                i64::from(counts.failed),
                i64::from(counts.cancelled),
                error_message
            ],
        )?;

        Ok(())
    }

    fn migrate(&self) -> ArchiveStoreResult<()> {
        self.connection.execute_batch(
            r#"
            BEGIN;

            CREATE TABLE IF NOT EXISTS schema_migrations (
              version INTEGER PRIMARY KEY,
              applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS app_settings (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              archive_dir TEXT NOT NULL,
              export_markdown INTEGER NOT NULL DEFAULT 1,
              export_html INTEGER NOT NULL DEFAULT 1,
              sync_history_limit INTEGER NOT NULL DEFAULT 20,
              sync_page_size INTEGER NOT NULL DEFAULT 5,
              download_concurrency INTEGER NOT NULL DEFAULT 2,
              network_proxy_url TEXT NOT NULL DEFAULT '',
              network_proxy_authorization TEXT,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS article_archives (
              article_id TEXT PRIMARY KEY,
              target_account_id TEXT NOT NULL,
              title TEXT NOT NULL,
              source_url TEXT NOT NULL,
              html_file TEXT,
              markdown_file TEXT,
              reading_read_count INTEGER,
              reading_like_count INTEGER,
              reading_share_count INTEGER,
              reading_comment_count INTEGER,
              reading_paid_content TEXT,
              reading_enriched_at_unix INTEGER,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS article_reading_comments (
              article_id TEXT NOT NULL,
              comment_id TEXT NOT NULL,
              author_name TEXT NOT NULL,
              content TEXT NOT NULL,
              like_count INTEGER NOT NULL DEFAULT 0,
              created_at_unix INTEGER NOT NULL DEFAULT 0,
              raw_json TEXT,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              PRIMARY KEY (article_id, comment_id),
              FOREIGN KEY (article_id) REFERENCES article_archives(article_id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS target_accounts (
              fakeid TEXT PRIMARY KEY,
              nickname TEXT NOT NULL,
              alias TEXT NOT NULL,
              round_head_img TEXT NOT NULL,
              service_type INTEGER NOT NULL,
              signature TEXT NOT NULL,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS target_articles (
              article_id TEXT NOT NULL,
              target_account_id TEXT NOT NULL,
              title TEXT NOT NULL,
              source_url TEXT NOT NULL,
              digest TEXT NOT NULL,
              author_name TEXT NOT NULL,
              cover TEXT NOT NULL,
              appmsgid INTEGER NOT NULL,
              itemidx INTEGER NOT NULL,
              item_show_type INTEGER NOT NULL,
              create_time INTEGER NOT NULL,
              update_time INTEGER NOT NULL,
              is_deleted INTEGER NOT NULL,
              copyright_type INTEGER NOT NULL,
              album_infos_json TEXT NOT NULL DEFAULT '[]',
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              PRIMARY KEY (target_account_id, article_id)
            );

            CREATE TABLE IF NOT EXISTS article_list_sync_runs (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              target_account_id TEXT NOT NULL,
              requested_limit INTEGER NOT NULL,
              fetched_count INTEGER NOT NULL,
              total_count INTEGER,
              status TEXT NOT NULL,
              error_message TEXT,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS collection_tasks (
              task_id TEXT PRIMARY KEY,
              task_type TEXT NOT NULL,
              target_account_id TEXT,
              status TEXT NOT NULL,
              total_items INTEGER NOT NULL DEFAULT 0,
              waiting_items INTEGER NOT NULL DEFAULT 0,
              running_items INTEGER NOT NULL DEFAULT 0,
              succeeded_items INTEGER NOT NULL DEFAULT 0,
              failed_items INTEGER NOT NULL DEFAULT 0,
              cancelled_items INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              error_message TEXT
            );

            CREATE TABLE IF NOT EXISTS collection_task_items (
              task_id TEXT NOT NULL,
              item_id TEXT NOT NULL,
              item_type TEXT NOT NULL,
              status TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              error_message TEXT,
              attempts INTEGER NOT NULL DEFAULT 0,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
              PRIMARY KEY (task_id, item_id),
              FOREIGN KEY (task_id) REFERENCES collection_tasks(task_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_collection_tasks_target_account
            ON collection_tasks(target_account_id, updated_at);

            CREATE INDEX IF NOT EXISTS idx_collection_task_items_status
            ON collection_task_items(task_id, status);

            CREATE INDEX IF NOT EXISTS idx_article_reading_comments_article
            ON article_reading_comments(article_id, created_at_unix DESC);

            COMMIT;
            "#,
        )?;
        if !self.table_has_column("target_articles", "album_infos_json")? {
            self.connection.execute(
                "ALTER TABLE target_articles ADD COLUMN album_infos_json TEXT NOT NULL DEFAULT '[]'",
                [],
            )?;
        }
        self.add_column_if_missing(
            "app_settings",
            "export_markdown",
            "INTEGER NOT NULL DEFAULT 1",
        )?;
        self.add_column_if_missing("app_settings", "export_html", "INTEGER NOT NULL DEFAULT 1")?;
        self.add_column_if_missing(
            "app_settings",
            "sync_history_limit",
            "INTEGER NOT NULL DEFAULT 20",
        )?;
        self.add_column_if_missing(
            "app_settings",
            "sync_page_size",
            "INTEGER NOT NULL DEFAULT 5",
        )?;
        self.add_column_if_missing(
            "app_settings",
            "download_concurrency",
            "INTEGER NOT NULL DEFAULT 2",
        )?;
        self.add_column_if_missing(
            "app_settings",
            "network_proxy_url",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        self.add_column_if_missing("app_settings", "network_proxy_authorization", "TEXT")?;
        self.add_column_if_missing("article_archives", "reading_read_count", "INTEGER")?;
        self.add_column_if_missing("article_archives", "reading_like_count", "INTEGER")?;
        self.add_column_if_missing("article_archives", "reading_share_count", "INTEGER")?;
        self.add_column_if_missing("article_archives", "reading_comment_count", "INTEGER")?;
        self.add_column_if_missing("article_archives", "reading_paid_content", "TEXT")?;
        self.add_column_if_missing("article_archives", "reading_enriched_at_unix", "INTEGER")?;
        self.connection.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;

        Ok(())
    }

    fn add_column_if_missing(
        &self,
        table_name: &str,
        column_name: &str,
        column_definition: &str,
    ) -> ArchiveStoreResult<()> {
        if !self.table_has_column(table_name, column_name)? {
            self.connection.execute(
                &format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {column_definition}"),
                [],
            )?;
        }

        Ok(())
    }

    fn table_has_column(&self, table_name: &str, column_name: &str) -> ArchiveStoreResult<bool> {
        let mut statement = self
            .connection
            .prepare(&format!("PRAGMA table_info({table_name})"))?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(columns.iter().any(|column| column == column_name))
    }
}

pub fn initialize_archive_store_from_app_data_dir(
    app_data_dir: impl AsRef<Path>,
) -> ArchiveStoreResult<ArchiveStoreSnapshot> {
    ArchiveStore::initialize_from_app_data_dir(app_data_dir)
}

fn path_to_database_text(path: &Path) -> ArchiveStoreResult<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| ArchiveStoreError::NonUtf8Path(path.to_path_buf()))
}

fn optional_relative_path_to_text(path: Option<&Path>) -> ArchiveStoreResult<Option<String>> {
    path.map(relative_path_to_text).transpose()
}

fn parse_target_article_album_infos(
    raw_json: &str,
) -> ArchiveStoreResult<Vec<TargetArticleAlbumInfo>> {
    Ok(serde_json::from_str(raw_json)?)
}

fn settings_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArchiveStoreSettings> {
    let archive_dir = row.get::<_, String>(0)?;
    let export_markdown = row.get::<_, i64>(1)?;
    let export_html = row.get::<_, i64>(2)?;
    let history_limit = row.get::<_, i64>(3)?;
    let page_size = row.get::<_, i64>(4)?;
    let download_concurrency = row.get::<_, i64>(5)?;
    let proxy_url = row.get::<_, String>(6)?;
    let proxy_authorization = row.get::<_, Option<String>>(7)?;
    let proxy_url = proxy_url.trim().to_string();
    let proxy_authorization = proxy_authorization
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let network_proxy = (!proxy_url.is_empty()).then_some(DesktopNetworkProxySetting {
        url: proxy_url,
        authorization: proxy_authorization,
    });

    Ok(ArchiveStoreSettings {
        archive_dir: PathBuf::from(archive_dir),
        export: DesktopExportSettings {
            markdown: database_int_to_bool(export_markdown),
            html: database_int_to_bool(export_html),
        },
        sync_download: DesktopSyncDownloadSettings {
            history_limit: database_i64_to_u32(history_limit, 20),
            page_size: database_i64_to_u32(page_size, 5),
            download_concurrency: database_i64_to_u32(download_concurrency, 2),
        },
        network_proxy,
    })
}

fn archive_article_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArchiveArticle> {
    let html_file = row.get::<_, Option<String>>(4)?.map(PathBuf::from);
    let markdown_file = row.get::<_, Option<String>>(5)?.map(PathBuf::from);
    let read_count = row.get::<_, Option<i64>>(6)?;
    let like_count = row.get::<_, Option<i64>>(7)?;
    let share_count = row.get::<_, Option<i64>>(8)?;
    let comment_count = row.get::<_, Option<i64>>(9)?;
    let paid_content = row.get::<_, Option<String>>(10)?;
    let enriched_at_unix = row.get::<_, Option<i64>>(11)?;
    let reading_enrichment = enriched_at_unix.map(|enriched_at_unix| ArticleReadingEnrichment {
        read_count: database_i64_to_optional_u32(read_count),
        like_count: database_i64_to_optional_u32(like_count),
        share_count: database_i64_to_optional_u32(share_count),
        comment_count: database_i64_to_optional_u32(comment_count),
        paid_content,
        enriched_at_unix,
    });

    Ok(ArchiveArticle {
        article_id: row.get(0)?,
        target_account_id: row.get(1)?,
        title: row.get(2)?,
        source_url: row.get(3)?,
        html_file,
        markdown_file,
        reading_enrichment,
        reading_comments: Vec::new(),
    })
}

fn bool_to_database_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn database_int_to_bool(value: i64) -> bool {
    value != 0
}

fn database_i64_to_u32(value: i64, fallback: u32) -> u32 {
    u32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn database_i64_to_optional_u32(value: Option<i64>) -> Option<u32> {
    value.and_then(|value| u32::try_from(value).ok())
}

fn optional_u32_to_database_i64(value: Option<u32>) -> Option<i64> {
    value.map(i64::from)
}

fn collection_task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CollectionTask> {
    let task_type = row.get::<_, String>(1)?;
    let status = row.get::<_, String>(3)?;
    let total_items = row.get::<_, i64>(4)?;
    let waiting_items = row.get::<_, i64>(5)?;
    let running_items = row.get::<_, i64>(6)?;
    let succeeded_items = row.get::<_, i64>(7)?;
    let failed_items = row.get::<_, i64>(8)?;
    let cancelled_items = row.get::<_, i64>(9)?;

    Ok(CollectionTask {
        task_id: row.get(0)?,
        task_type: CollectionTaskType::from_database_value(&task_type)
            .map_err(to_from_sql_conversion_failure(1))?,
        target_account_id: row.get(2)?,
        status: CollectionTaskStatus::from_database_value(&status)
            .map_err(to_from_sql_conversion_failure(3))?,
        total_items: total_items as u32,
        waiting_items: waiting_items as u32,
        running_items: running_items as u32,
        succeeded_items: succeeded_items as u32,
        failed_items: failed_items as u32,
        cancelled_items: cancelled_items as u32,
        error_message: row.get(10)?,
        items: Vec::new(),
    })
}

fn infer_collection_task_status(counts: CollectionTaskCounts) -> CollectionTaskStatus {
    if counts.total == 0 {
        CollectionTaskStatus::Waiting
    } else if counts.failed > 0 {
        CollectionTaskStatus::Failed
    } else if counts.running > 0 || counts.succeeded > 0 {
        if counts.succeeded + counts.cancelled == counts.total {
            CollectionTaskStatus::Succeeded
        } else {
            CollectionTaskStatus::Running
        }
    } else if counts.cancelled == counts.total {
        CollectionTaskStatus::Cancelled
    } else {
        CollectionTaskStatus::Waiting
    }
}

fn generate_collection_task_id() -> String {
    let epoch_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let counter = COLLECTION_TASK_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("task-{epoch_millis}-{counter}")
}

fn to_from_sql_conversion_failure(
    column: usize,
) -> impl FnOnce(ArchiveStoreError) -> rusqlite::Error {
    move |error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    }
}

fn relative_path_to_text(path: &Path) -> ArchiveStoreResult<String> {
    if !is_safe_relative_path(path) {
        return Err(ArchiveStoreError::InvalidRelativePath(path.to_path_buf()));
    }

    path_to_database_text(path)
}

fn is_safe_relative_path(path: &Path) -> bool {
    let mut has_component = false;

    for component in path.components() {
        match component {
            Component::Normal(_) => has_component = true,
            Component::CurDir
            | Component::ParentDir
            | Component::Prefix(_)
            | Component::RootDir => return false,
        }
    }

    has_component && !path.is_absolute()
}
