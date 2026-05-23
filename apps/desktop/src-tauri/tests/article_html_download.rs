use std::collections::{HashMap, VecDeque};
use std::fs;
use std::sync::{Arc, Mutex};

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveStore, ArchiveStoreConfig, CollectionTaskStatus, CollectionTaskType, TargetArticleInput,
};
use wechat_article_exporter_desktop_lib::article_html_download::{
    ArticleHtmlDownloadClient, ArticleHtmlDownloadError, ArticleHtmlDownloadResourceRequest,
    ArticleHtmlDownloadResourceResponse, ArticleHtmlDownloadTransport, NetworkProxySetting,
};
use wechat_article_exporter_desktop_lib::official_account_login::{
    OfficialAccountLoginProfile, OfficialAccountLoginSecret,
};
use wechat_article_exporter_desktop_lib::secret_store::{
    MemorySecretBackend, SecretSlot, SecretStore,
};

#[test]
fn downloads_article_html_writes_archive_record_and_discovers_assets() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let backend = MemorySecretBackend::default();
    save_login_secret(&backend);
    store
        .upsert_target_article(&target_article(
            "fakeid-1",
            "aid-1",
            "First Article",
            "https://mp.weixin.qq.com/s/first",
        ))
        .expect("store target article");
    let transport = FixtureArticleHtmlDownloadTransport::default()
        .with_html(
            r#"
            <!doctype html>
            <html>
              <head><title>First Article</title></head>
              <body>
                <article id="js_article">
                  <img data-src="https://mmbiz.qpic.cn/mmbiz_png/cover.png" />
                  <img src="//mmbiz.qpic.cn/mmbiz_jpg/inline.jpg" />
                </article>
              </body>
            </html>
            "#,
        )
        .with_asset("https://mmbiz.qpic.cn/mmbiz_png/cover.png", b"cover")
        .with_asset("https://mmbiz.qpic.cn/mmbiz_jpg/inline.jpg", b"inline");
    let client = ArticleHtmlDownloadClient::new(transport.clone(), SecretStore::new(backend));

    let outcome = client
        .download_article(&store, "fakeid-1", "aid-1", None)
        .expect("download article html");

    assert_eq!(outcome.article_id, "aid-1");
    assert!(outcome.html_file.ends_with("aid-1.html"));
    assert_eq!(outcome.asset_files.len(), 2);
    assert!(store.archive_dir().join(&outcome.html_file).is_file());
    assert!(store.archive_dir().join(&outcome.asset_files[0]).is_file());
    let html = fs::read_to_string(store.archive_dir().join(&outcome.html_file))
        .expect("read archived html");
    assert!(html.contains("First Article"));

    let archived = store
        .get_article("aid-1")
        .expect("load archived article")
        .expect("article archive exists");
    assert_eq!(
        archived.html_file.as_deref(),
        Some(outcome.html_file.as_path())
    );
    assert_eq!(archived.target_account_id, "fakeid-1");

    let tasks = store
        .list_collection_tasks()
        .expect("list collection tasks");
    let task = tasks
        .iter()
        .find(|task| task.task_type == CollectionTaskType::ArticleHtmlDownload)
        .expect("html download task exists");
    assert_eq!(task.status, CollectionTaskStatus::Succeeded);
    assert_eq!(task.succeeded_items, 1);

    let requests = transport.requests.lock().expect("read requests");
    assert_eq!(requests[0].url, "https://mp.weixin.qq.com/s/first");
    assert_eq!(
        requests[0].cookie_header.as_deref(),
        Some("wxuin=1; rand_info=secret")
    );
    assert_eq!(requests[0].proxy, None);
}

#[test]
fn retries_transient_failures_and_records_failed_task_state() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let backend = MemorySecretBackend::default();
    save_login_secret(&backend);
    store
        .upsert_target_article(&target_article(
            "fakeid-1",
            "aid-1",
            "First Article",
            "https://mp.weixin.qq.com/s/first",
        ))
        .expect("store target article");
    let transport = FixtureArticleHtmlDownloadTransport::default()
        .with_html_error("temporary timeout")
        .with_html_error("still timeout")
        .with_html("<html><body>Recovered</body></html>");
    let client = ArticleHtmlDownloadClient::new(transport.clone(), SecretStore::new(backend));

    let outcome = client
        .download_article(&store, "fakeid-1", "aid-1", None)
        .expect("download article after retries");

    assert!(store.archive_dir().join(outcome.html_file).is_file());
    assert_eq!(transport.requests.lock().expect("read requests").len(), 3);

    let failing_transport = FixtureArticleHtmlDownloadTransport::default()
        .with_html_error("permanent timeout")
        .with_html_error("permanent timeout")
        .with_html_error("permanent timeout");
    let failing_client = ArticleHtmlDownloadClient::new(
        failing_transport.clone(),
        SecretStore::new({
            let backend = MemorySecretBackend::default();
            save_login_secret(&backend);
            backend
        }),
    );

    let error = failing_client
        .download_article(&store, "fakeid-1", "aid-1", None)
        .expect_err("download should fail");

    assert!(matches!(error, ArticleHtmlDownloadError::Transport(_)));
    let tasks = store
        .list_collection_tasks()
        .expect("list collection tasks");
    let failed = tasks
        .iter()
        .find(|task| {
            task.task_type == CollectionTaskType::ArticleHtmlDownload
                && task.status == CollectionTaskStatus::Failed
        })
        .expect("failed task exists");
    assert_eq!(failed.failed_items, 1);
    assert!(failed
        .error_message
        .as_deref()
        .expect("error message")
        .contains("permanent timeout"));
}

#[test]
fn optional_proxy_setting_is_passed_to_outbound_requests() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let backend = MemorySecretBackend::default();
    save_login_secret(&backend);
    store
        .upsert_target_article(&target_article(
            "fakeid-1",
            "aid-1",
            "First Article",
            "https://mp.weixin.qq.com/s/first",
        ))
        .expect("store target article");
    let transport = FixtureArticleHtmlDownloadTransport::default()
        .with_html(r#"<html><body><img src="https://mmbiz.qpic.cn/mmbiz_png/a.png"></body></html>"#)
        .with_asset("https://mmbiz.qpic.cn/mmbiz_png/a.png", b"a");
    let client = ArticleHtmlDownloadClient::new(transport.clone(), SecretStore::new(backend));

    client
        .download_article(
            &store,
            "fakeid-1",
            "aid-1",
            Some(NetworkProxySetting {
                url: "http://127.0.0.1:7890".to_string(),
                authorization: Some("Bearer local-token".to_string()),
            }),
        )
        .expect("download with proxy setting");

    let requests = transport.requests.lock().expect("read requests");
    assert_eq!(
        requests[0].proxy.as_ref().map(|proxy| proxy.url.as_str()),
        Some("http://127.0.0.1:7890")
    );
    assert_eq!(
        requests[1]
            .proxy
            .as_ref()
            .and_then(|proxy| proxy.authorization.as_deref()),
        Some("Bearer local-token")
    );
}

#[test]
fn missing_official_account_login_blocks_download_before_network() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    store
        .upsert_target_article(&target_article(
            "fakeid-1",
            "aid-1",
            "First Article",
            "https://mp.weixin.qq.com/s/first",
        ))
        .expect("store target article");
    let transport = FixtureArticleHtmlDownloadTransport::default();
    let client = ArticleHtmlDownloadClient::new(
        transport.clone(),
        SecretStore::new(MemorySecretBackend::default()),
    );

    let error = client
        .download_article(&store, "fakeid-1", "aid-1", None)
        .expect_err("download should require login");

    assert!(matches!(
        error,
        ArticleHtmlDownloadError::MissingOfficialAccountLogin
    ));
    assert!(transport.requests.lock().expect("read requests").is_empty());
}

fn target_article(
    target_account_id: &str,
    article_id: &str,
    title: &str,
    source_url: &str,
) -> TargetArticleInput {
    TargetArticleInput {
        article_id: article_id.to_string(),
        target_account_id: target_account_id.to_string(),
        title: title.to_string(),
        source_url: source_url.to_string(),
        digest: format!("{title} digest"),
        author_name: "Fixture Author".to_string(),
        cover: "https://example.test/cover.jpg".to_string(),
        appmsgid: 100,
        itemidx: 1,
        item_show_type: 0,
        create_time: 1_700_000_010,
        update_time: 1_700_000_020,
        is_deleted: false,
        copyright_type: 0,
        album_infos: Vec::new(),
    }
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

    fn with_html_error(self, error: &str) -> Self {
        self.html_results
            .lock()
            .expect("lock html results")
            .push_back(Err(error.to_string()));
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
