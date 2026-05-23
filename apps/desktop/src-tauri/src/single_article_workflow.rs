use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::archive_store::{ArchiveArticle, ArchiveArticleInput, ArchiveStore, ArchiveStoreError};

pub const SINGLE_ARTICLE_TARGET_ACCOUNT_ID: &str = "single-article";

pub type SingleArticleWorkflowResult<T> = Result<T, SingleArticleWorkflowError>;

#[derive(Debug)]
pub enum SingleArticleWorkflowError {
    ArchiveStore(ArchiveStoreError),
    InvalidArticleUrl(String),
}

impl fmt::Display for SingleArticleWorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArchiveStore(error) => write!(formatter, "{error}"),
            Self::InvalidArticleUrl(value) => {
                write!(formatter, "invalid WeChat article URL: {value}")
            }
        }
    }
}

impl std::error::Error for SingleArticleWorkflowError {}

impl From<ArchiveStoreError> for SingleArticleWorkflowError {
    fn from(error: ArchiveStoreError) -> Self {
        Self::ArchiveStore(error)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SingleArticleSaveRequest {
    pub source_url: String,
    pub title: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SingleArticleArchive {
    pub article_id: String,
    pub target_account_id: String,
    pub title: String,
    pub source_url: String,
    pub html_file: Option<PathBuf>,
    pub markdown_file: Option<PathBuf>,
}

pub struct SingleArticleWorkflowService;

impl SingleArticleWorkflowService {
    pub fn new() -> Self {
        Self
    }

    pub fn save_article(
        &self,
        archive_store: &ArchiveStore,
        request: SingleArticleSaveRequest,
    ) -> SingleArticleWorkflowResult<SingleArticleArchive> {
        let normalized_url = normalize_wechat_article_url(&request.source_url)?;
        let article_id = article_id_from_url(&normalized_url)?;
        let existing = archive_store.get_article(&article_id)?;
        let title = request
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| existing.as_ref().map(|article| article.title.clone()))
            .unwrap_or_else(|| {
                format!(
                    "Single Article {}",
                    article_id.trim_start_matches("single-")
                )
            });

        archive_store.upsert_article(&ArchiveArticleInput {
            article_id: article_id.clone(),
            target_account_id: SINGLE_ARTICLE_TARGET_ACCOUNT_ID.to_string(),
            title,
            source_url: normalized_url.to_string(),
            html_file: existing
                .as_ref()
                .and_then(|article| article.html_file.clone()),
            markdown_file: existing
                .as_ref()
                .and_then(|article| article.markdown_file.clone()),
        })?;

        archive_store
            .get_article(&article_id)?
            .map(SingleArticleArchive::from)
            .ok_or_else(|| {
                SingleArticleWorkflowError::ArchiveStore(ArchiveStoreError::Sqlite(
                    rusqlite::Error::QueryReturnedNoRows,
                ))
            })
    }

    pub fn list_articles(
        &self,
        archive_store: &ArchiveStore,
    ) -> SingleArticleWorkflowResult<Vec<SingleArticleArchive>> {
        Ok(archive_store
            .list_articles_by_target_account(SINGLE_ARTICLE_TARGET_ACCOUNT_ID)?
            .into_iter()
            .map(SingleArticleArchive::from)
            .collect::<Vec<_>>())
    }
}

impl From<ArchiveArticle> for SingleArticleArchive {
    fn from(article: ArchiveArticle) -> Self {
        Self {
            article_id: article.article_id,
            target_account_id: article.target_account_id,
            title: article.title,
            source_url: article.source_url,
            html_file: article.html_file,
            markdown_file: article.markdown_file,
        }
    }
}

fn normalize_wechat_article_url(raw_url: &str) -> SingleArticleWorkflowResult<Url> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Err(SingleArticleWorkflowError::InvalidArticleUrl(
            raw_url.to_string(),
        ));
    }
    let candidate = if trimmed.starts_with("//") {
        format!("https:{trimmed}")
    } else if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let mut url = Url::parse(&candidate)
        .map_err(|_| SingleArticleWorkflowError::InvalidArticleUrl(raw_url.to_string()))?;

    if url.host_str() != Some("mp.weixin.qq.com") {
        return Err(SingleArticleWorkflowError::InvalidArticleUrl(
            raw_url.to_string(),
        ));
    }
    if url.scheme() != "https" {
        url.set_scheme("https")
            .map_err(|_| SingleArticleWorkflowError::InvalidArticleUrl(raw_url.to_string()))?;
    }
    url.set_fragment(None);

    if article_identity(&url).is_none() {
        return Err(SingleArticleWorkflowError::InvalidArticleUrl(
            raw_url.to_string(),
        ));
    }

    Ok(url)
}

fn article_id_from_url(url: &Url) -> SingleArticleWorkflowResult<String> {
    let identity = article_identity(url)
        .ok_or_else(|| SingleArticleWorkflowError::InvalidArticleUrl(url.to_string()))?;

    Ok(format!("single-{}", safe_article_id_segment(&identity)))
}

fn article_identity(url: &Url) -> Option<String> {
    let segments = url
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .unwrap_or_default();

    if segments.first() == Some(&"s") {
        if let Some(slug) = segments.get(1).filter(|value| !value.trim().is_empty()) {
            return Some(format!("s-{slug}"));
        }
    }

    let query = url.query_pairs().into_owned().collect::<HashMap<_, _>>();
    if let Some(sn) = query.get("sn").filter(|value| !value.trim().is_empty()) {
        let mut parts = Vec::new();
        if let Some(mid) = query.get("mid").filter(|value| !value.trim().is_empty()) {
            parts.push(mid.as_str());
        }
        if let Some(idx) = query.get("idx").filter(|value| !value.trim().is_empty()) {
            parts.push(idx.as_str());
        }
        parts.push(sn);
        return Some(format!("s-{}", parts.join("-")));
    }

    None
}

fn safe_article_id_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let sanitized = sanitized.trim_matches(['.', '_']);

    if sanitized.is_empty() {
        "article".to_string()
    } else {
        sanitized.to_string()
    }
}
