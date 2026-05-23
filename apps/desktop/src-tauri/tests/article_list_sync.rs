use std::sync::{Arc, Mutex};

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveStore, ArchiveStoreConfig, ArticleListSyncStatus, CollectionTaskStatus,
    CollectionTaskType, TargetArticleInput,
};
use wechat_article_exporter_desktop_lib::article_list_sync::{
    parse_appmsgpublish_response, ArticleListSyncClient, ArticleListSyncPage,
    ArticleListSyncRequest, ArticleListSyncTransport,
};
use wechat_article_exporter_desktop_lib::official_account_login::{
    OfficialAccountLoginProfile, OfficialAccountLoginSecret,
};
use wechat_article_exporter_desktop_lib::secret_store::{
    MemorySecretBackend, SecretSlot, SecretStore,
};

#[test]
fn target_account_articles_are_stored_and_listed_after_reopen() {
    let temp = tempdir().expect("temp dir");
    let config = ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    };
    let store = ArchiveStore::open(config.clone()).expect("open archive store");

    store
        .upsert_target_article(&target_article(
            "fakeid-1",
            "aid-1",
            "First Article",
            1_700_000_010,
        ))
        .expect("store first article");
    store
        .upsert_target_article(&target_article(
            "fakeid-1",
            "aid-2",
            "Second Article",
            1_700_000_020,
        ))
        .expect("store second article");
    drop(store);

    let reopened = ArchiveStore::open(config).expect("reopen archive store");
    let articles = reopened
        .list_target_articles("fakeid-1")
        .expect("list synced articles");

    assert_eq!(articles.len(), 2);
    assert_eq!(articles[0].article_id, "aid-2");
    assert_eq!(articles[1].article_id, "aid-1");
    assert_eq!(articles[0].title, "Second Article");
}

#[test]
fn records_article_list_sync_completion_and_failure_state() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");

    store
        .record_article_list_sync(
            "fakeid-1",
            20,
            12,
            Some(38),
            ArticleListSyncStatus::Completed,
            None,
        )
        .expect("record completed sync");
    store
        .record_article_list_sync(
            "fakeid-1",
            20,
            0,
            None,
            ArticleListSyncStatus::Failed,
            Some("network timeout".to_string()),
        )
        .expect("record failed sync");

    let latest = store
        .latest_article_list_sync("fakeid-1")
        .expect("load latest sync")
        .expect("sync exists");

    assert_eq!(latest.status, ArticleListSyncStatus::Failed);
    assert_eq!(latest.requested_limit, 20);
    assert_eq!(latest.fetched_count, 0);
    assert_eq!(latest.error_message.as_deref(), Some("network timeout"));
}

#[test]
fn parses_representative_appmsgpublish_response() {
    let raw = r#"
    {
      "base_resp": {"ret": 0, "err_msg": "ok"},
      "publish_page": "{\"total_count\":2,\"publish_list\":[{\"publish_info\":\"{\\\"appmsgex\\\":[{\\\"aid\\\":\\\"aid-1\\\",\\\"appmsgid\\\":101,\\\"itemidx\\\":1,\\\"title\\\":\\\"First Article\\\",\\\"link\\\":\\\"https://mp.weixin.qq.com/s/first\\\",\\\"digest\\\":\\\"First digest\\\",\\\"author_name\\\":\\\"Alice\\\",\\\"cover\\\":\\\"https://example.test/first.jpg\\\",\\\"create_time\\\":1700000010,\\\"update_time\\\":1700000020,\\\"item_show_type\\\":0,\\\"is_deleted\\\":false,\\\"copyright_type\\\":0}]}\"},{\"publish_info\":\"{\\\"appmsgex\\\":[{\\\"aid\\\":\\\"aid-2\\\",\\\"appmsgid\\\":102,\\\"itemidx\\\":1,\\\"title\\\":\\\"Second Article\\\",\\\"link\\\":\\\"https://mp.weixin.qq.com/s/second\\\",\\\"digest\\\":\\\"Second digest\\\",\\\"author_name\\\":\\\"Bob\\\",\\\"cover\\\":\\\"https://example.test/second.jpg\\\",\\\"create_time\\\":1700000110,\\\"update_time\\\":1700000120,\\\"item_show_type\\\":0,\\\"is_deleted\\\":false,\\\"copyright_type\\\":0}]}\"}]}"
    }
    "#;

    let page = parse_appmsgpublish_response("fakeid-1", raw).expect("parse article list response");

    assert_eq!(page.total_count, Some(2));
    assert_eq!(page.articles.len(), 2);
    assert_eq!(page.articles[0].article_id, "aid-1");
    assert_eq!(page.articles[0].target_account_id, "fakeid-1");
    assert_eq!(page.articles[0].title, "First Article");
    assert_eq!(
        page.articles[1].source_url,
        "https://mp.weixin.qq.com/s/second"
    );
}

#[test]
fn sync_uses_stored_login_secret_persists_limited_articles_and_records_completion() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let backend = MemorySecretBackend::default();
    save_login_secret(&backend);
    let transport = FixtureArticleListSyncTransport::new().with_page(ArticleListSyncPage {
        total_count: Some(2),
        articles: vec![
            target_article("fakeid-1", "aid-1", "First Article", 1_700_000_010),
            target_article("fakeid-1", "aid-2", "Second Article", 1_700_000_020),
        ],
    });
    let client = ArticleListSyncClient::new(transport.clone(), SecretStore::new(backend));

    let outcome = client
        .sync(&store, "fakeid-1", 1, 5)
        .expect("sync target article list");

    assert_eq!(outcome.status, ArticleListSyncStatus::Completed);
    assert_eq!(outcome.requested_limit, 1);
    assert_eq!(outcome.fetched_count, 1);
    assert_eq!(outcome.total_count, Some(2));
    let stored_articles = store
        .list_target_articles("fakeid-1")
        .expect("list synced articles");
    assert_eq!(stored_articles.len(), 1);
    assert_eq!(stored_articles[0].article_id, "aid-1");
    let latest = store
        .latest_article_list_sync("fakeid-1")
        .expect("load latest sync")
        .expect("sync exists");
    assert_eq!(latest.status, ArticleListSyncStatus::Completed);

    let calls = transport.calls.lock().expect("read transport calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].fakeid, "fakeid-1");
    assert_eq!(calls[0].begin, 0);
    assert_eq!(calls[0].count, 1);
    assert_eq!(calls[0].token, "token-123");
    assert_eq!(calls[0].cookie_header, "wxuin=1; rand_info=secret");
}

#[test]
fn sync_records_persistent_collection_task_status() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let backend = MemorySecretBackend::default();
    save_login_secret(&backend);
    let transport = FixtureArticleListSyncTransport::new().with_page(ArticleListSyncPage {
        total_count: Some(1),
        articles: vec![target_article(
            "fakeid-1",
            "aid-1",
            "First Article",
            1_700_000_010,
        )],
    });
    let client = ArticleListSyncClient::new(transport, SecretStore::new(backend));

    client
        .sync(&store, "fakeid-1", 20, 5)
        .expect("sync target article list");

    let tasks = store
        .list_collection_tasks()
        .expect("list collection tasks");
    let task = tasks
        .iter()
        .find(|task| task.task_type == CollectionTaskType::AccountArticleSync)
        .expect("article sync task exists");

    assert_eq!(task.target_account_id.as_deref(), Some("fakeid-1"));
    assert_eq!(task.status, CollectionTaskStatus::Succeeded);
    assert_eq!(task.total_items, 1);
    assert_eq!(task.succeeded_items, 1);
}

#[test]
fn sync_failure_records_retryable_error_information() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let backend = MemorySecretBackend::default();
    save_login_secret(&backend);
    let client = ArticleListSyncClient::new(
        FixtureArticleListSyncTransport::new().with_error("upstream timeout"),
        SecretStore::new(backend),
    );

    let error = client
        .sync(&store, "fakeid-1", 20, 5)
        .expect_err("sync should fail");

    assert!(error.to_string().contains("upstream timeout"));
    let latest = store
        .latest_article_list_sync("fakeid-1")
        .expect("load latest sync")
        .expect("sync exists");
    assert_eq!(latest.status, ArticleListSyncStatus::Failed);
    assert_eq!(latest.error_message.as_deref(), Some("upstream timeout"));
    assert_eq!(latest.requested_limit, 20);
    let tasks = store
        .list_collection_tasks()
        .expect("list collection tasks");
    let task = tasks
        .iter()
        .find(|task| task.task_type == CollectionTaskType::AccountArticleSync)
        .expect("article sync task exists");
    assert_eq!(task.status, CollectionTaskStatus::Failed);
    assert_eq!(task.failed_items, 1);
    assert_eq!(task.error_message.as_deref(), Some("upstream timeout"));
}

fn target_article(
    target_account_id: &str,
    article_id: &str,
    title: &str,
    create_time: i64,
) -> TargetArticleInput {
    TargetArticleInput {
        article_id: article_id.to_string(),
        target_account_id: target_account_id.to_string(),
        title: title.to_string(),
        source_url: format!("https://mp.weixin.qq.com/s/{article_id}"),
        digest: format!("{title} digest"),
        author_name: "Fixture Author".to_string(),
        cover: format!("https://example.test/{article_id}.jpg"),
        appmsgid: 100,
        itemidx: 1,
        item_show_type: 0,
        create_time,
        update_time: create_time + 1,
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
struct FixtureArticleListSyncTransport {
    calls: Arc<Mutex<Vec<ArticleListSyncRequest>>>,
    pages: Arc<Mutex<Vec<ArticleListSyncPage>>>,
    error: Arc<Mutex<Option<String>>>,
}

impl FixtureArticleListSyncTransport {
    fn new() -> Self {
        Self::default()
    }

    fn with_page(self, page: ArticleListSyncPage) -> Self {
        self.pages.lock().expect("lock pages").push(page);
        self
    }

    fn with_error(self, message: &str) -> Self {
        *self.error.lock().expect("lock error") = Some(message.to_string());
        self
    }
}

impl ArticleListSyncTransport for FixtureArticleListSyncTransport {
    fn fetch_page(&self, request: ArticleListSyncRequest) -> Result<ArticleListSyncPage, String> {
        self.calls.lock().expect("lock calls").push(request);
        if let Some(error) = self.error.lock().expect("lock error").clone() {
            return Err(error);
        }

        let mut pages = self.pages.lock().expect("lock pages");
        if pages.is_empty() {
            Ok(ArticleListSyncPage {
                total_count: Some(0),
                articles: Vec::new(),
            })
        } else {
            Ok(pages.remove(0))
        }
    }
}
