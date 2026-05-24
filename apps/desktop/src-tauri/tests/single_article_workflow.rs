use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{ArchiveStore, ArchiveStoreConfig};
use wechat_article_exporter_desktop_lib::article_export::{
    ArticleArchivePreviewService, ArticleExportFormat, ArticleExportRequest, ArticleExportService,
};
use wechat_article_exporter_desktop_lib::article_html_download::{
    ArticleHtmlDownloadClient, ArticleHtmlDownloadResourceRequest,
    ArticleHtmlDownloadResourceResponse, ArticleHtmlDownloadTransport,
};
use wechat_article_exporter_desktop_lib::official_account_login::{
    OfficialAccountLoginProfile, OfficialAccountLoginSecret,
};
use wechat_article_exporter_desktop_lib::secret_store::{
    MemorySecretBackend, SecretSlot, SecretStore,
};
use wechat_article_exporter_desktop_lib::single_article_workflow::{
    SingleArticleSaveRequest, SingleArticleWorkflowError, SingleArticleWorkflowService,
    SINGLE_ARTICLE_TARGET_ACCOUNT_ID,
};

#[test]
fn saves_valid_wechat_article_url_as_persistent_single_article_archive() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let service = SingleArticleWorkflowService::new();

    let saved = service
        .save_article(
            &store,
            SingleArticleSaveRequest {
                source_url: " mp.weixin.qq.com/s/example-slug#wechat_redirect ".to_string(),
                title: Some("Reference Article".to_string()),
            },
        )
        .expect("save single article");

    assert_eq!(saved.article_id, "single-s-example-slug");
    assert_eq!(saved.target_account_id, SINGLE_ARTICLE_TARGET_ACCOUNT_ID);
    assert_eq!(saved.title, "Reference Article");
    assert_eq!(saved.source_url, "https://mp.weixin.qq.com/s/example-slug");
    assert_eq!(saved.html_file, None);

    let reopened = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("reopen archive store");
    let articles = service
        .list_articles(&reopened)
        .expect("list single articles");

    assert_eq!(articles, vec![saved]);
}

#[test]
fn rejects_non_wechat_article_urls() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");

    let error = SingleArticleWorkflowService::new()
        .save_article(
            &store,
            SingleArticleSaveRequest {
                source_url: "https://example.com/not-wechat".to_string(),
                title: None,
            },
        )
        .expect_err("non WeChat article URL should be rejected");

    assert!(matches!(
        error,
        SingleArticleWorkflowError::InvalidArticleUrl(_)
    ));
}

#[test]
fn downloads_single_article_archive_and_keeps_preview_and_export_available() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let backend = MemorySecretBackend::default();
    save_login_secret(&backend);
    let article = SingleArticleWorkflowService::new()
        .save_article(
            &store,
            SingleArticleSaveRequest {
                source_url: "https://mp.weixin.qq.com/s/example-slug".to_string(),
                title: Some("Reference Article".to_string()),
            },
        )
        .expect("save single article");
    let transport = FixtureArticleHtmlDownloadTransport::default()
        .with_html(
            r#"
            <!doctype html>
            <html>
              <body>
                <article id="js_article">
                  <div id="js_content"><p>Readable single article body</p></div>
                  <img data-src="https://mmbiz.qpic.cn/mmbiz_png/single.png" />
                </article>
              </body>
            </html>
            "#,
        )
        .with_asset(
            "https://mmbiz.qpic.cn/mmbiz_png/single.png",
            b"single-image",
        );
    let client = ArticleHtmlDownloadClient::new(transport.clone(), SecretStore::new(backend));

    let outcome = client
        .download_single_article(&store, &article.article_id, None)
        .expect("download single article");

    assert_eq!(outcome.article_id, article.article_id);
    assert_eq!(outcome.target_account_id, SINGLE_ARTICLE_TARGET_ACCOUNT_ID);
    assert!(store.archive_dir().join(&outcome.html_file).is_file());
    assert_eq!(outcome.asset_files.len(), 1);
    assert_eq!(
        transport.requests.lock().expect("read requests")[0].url,
        "https://mp.weixin.qq.com/s/example-slug"
    );

    let preview = ArticleArchivePreviewService::new()
        .preview_article(
            &store,
            wechat_article_exporter_desktop_lib::article_export::ArticleArchivePreviewRequest {
                article_id: article.article_id.clone(),
            },
        )
        .expect("preview downloaded article");
    assert!(preview.html.contains("Readable single article body"));

    let export = ArticleExportService::new()
        .export_article(
            &store,
            ArticleExportRequest {
                article_id: article.article_id.clone(),
                formats: vec![ArticleExportFormat::Markdown, ArticleExportFormat::Html],
                output_file: None,
                output_dir: None,
            },
        )
        .expect("export downloaded single article");
    assert!(export.markdown_file.is_some());
    assert!(export.html_file.is_some());
}

fn save_login_secret(backend: &MemorySecretBackend) {
    let login_secret = OfficialAccountLoginSecret {
        token: "token-123".to_string(),
        cookie_header: "wxuin=1; rand_info=secret".to_string(),
        profile: OfficialAccountLoginProfile {
            nickname: "Operator Official Account".to_string(),
            avatar_url: "https://example.test/operator.png".to_string(),
        },
        expires_at: "2026-05-27T00:00:00Z".to_string(),
    };
    SecretStore::new(backend.clone())
        .save(
            SecretSlot::OfficialAccountLogin,
            &serde_json::to_string(&login_secret).expect("serialize login secret"),
        )
        .expect("save login secret");
}

#[derive(Clone, Default)]
struct FixtureArticleHtmlDownloadTransport {
    requests: Arc<Mutex<Vec<ArticleHtmlDownloadResourceRequest>>>,
    html_results: Arc<Mutex<VecDeque<Result<String, String>>>>,
    assets: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl FixtureArticleHtmlDownloadTransport {
    fn with_html(self, html: &str) -> Self {
        self.html_results
            .lock()
            .expect("lock html results")
            .push_back(Ok(html.to_string()));
        self
    }

    fn with_asset(self, url: &str, bytes: &[u8]) -> Self {
        self.assets
            .lock()
            .expect("lock assets")
            .insert(url.to_string(), bytes.to_vec());
        self
    }
}

impl ArticleHtmlDownloadTransport for FixtureArticleHtmlDownloadTransport {
    fn fetch(
        &self,
        request: ArticleHtmlDownloadResourceRequest,
    ) -> Result<ArticleHtmlDownloadResourceResponse, String> {
        self.requests
            .lock()
            .expect("lock requests")
            .push(request.clone());

        if request.is_article_html {
            let result = self
                .html_results
                .lock()
                .expect("lock html results")
                .pop_front()
                .unwrap_or_else(|| Err("missing fixture html".to_string()))?;
            return Ok(ArticleHtmlDownloadResourceResponse {
                bytes: result.into_bytes(),
                content_type: Some("text/html; charset=utf-8".to_string()),
            });
        }

        let bytes = self
            .assets
            .lock()
            .expect("lock assets")
            .get(&request.url)
            .cloned()
            .ok_or_else(|| format!("missing fixture asset: {}", request.url))?;
        Ok(ArticleHtmlDownloadResourceResponse {
            bytes,
            content_type: Some("image/png".to_string()),
        })
    }
}
