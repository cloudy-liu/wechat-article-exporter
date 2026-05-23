use std::sync::{Arc, Mutex};

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveStore, ArchiveStoreConfig, ArticleReadingCommentInput, TargetArticleInput,
};
use wechat_article_exporter_desktop_lib::article_reading_enrichment::{
    ArticleReadingCredential, ArticleReadingCredentialService, ArticleReadingEnrichmentClient,
    ArticleReadingEnrichmentError, ArticleReadingEnrichmentFetchRequest,
    ArticleReadingEnrichmentFetchResponse, ArticleReadingEnrichmentRequest,
    ArticleReadingEnrichmentTransport,
};
use wechat_article_exporter_desktop_lib::secret_store::{
    MemorySecretBackend, SecretSlot, SecretStore,
};

const NOW_UNIX: i64 = 1_800_000_000;

#[test]
fn reading_credential_status_tracks_valid_and_expired_credentials() {
    let backend = MemorySecretBackend::default();
    let service = ArticleReadingCredentialService::new(SecretStore::new(backend.clone()), NOW_UNIX);

    let empty = service.status().expect("load empty status");
    assert!(!empty.configured);
    assert!(!empty.valid);
    assert!(!empty.expired);
    assert_eq!(empty.expires_at_unix, None);

    let saved = service
        .save(ArticleReadingCredential {
            biz: "MzA123".to_string(),
            uin: "12345".to_string(),
            key: "reading-key".to_string(),
            pass_ticket: "pass-ticket".to_string(),
            appmsg_token: "appmsg-token".to_string(),
            cookie: Some("wap_sid2=secret".to_string()),
            expires_at_unix: NOW_UNIX + 3600,
        })
        .expect("save reading credential");

    assert!(saved.configured);
    assert!(saved.valid);
    assert!(!saved.expired);
    assert_eq!(saved.expires_at_unix, Some(NOW_UNIX + 3600));

    let expired = ArticleReadingCredentialService::new(SecretStore::new(backend), NOW_UNIX + 7200)
        .status()
        .expect("load expired status");
    assert!(expired.configured);
    assert!(!expired.valid);
    assert!(expired.expired);
    assert_eq!(expired.expires_at_unix, Some(NOW_UNIX + 3600));
}

#[test]
fn mark_expired_updates_the_secret_store_without_touching_official_login() {
    let backend = MemorySecretBackend::default();
    SecretStore::new(backend.clone())
        .save(SecretSlot::OfficialAccountLogin, "official-login-secret")
        .expect("save official login secret");
    let service = ArticleReadingCredentialService::new(SecretStore::new(backend.clone()), NOW_UNIX);
    service
        .save(ArticleReadingCredential {
            biz: "MzA123".to_string(),
            uin: "12345".to_string(),
            key: "reading-key".to_string(),
            pass_ticket: "pass-ticket".to_string(),
            appmsg_token: "appmsg-token".to_string(),
            cookie: None,
            expires_at_unix: NOW_UNIX + 3600,
        })
        .expect("save reading credential");

    let status = service.mark_expired().expect("mark expired");

    assert!(status.configured);
    assert!(!status.valid);
    assert!(status.expired);
    assert_eq!(
        SecretStore::new(backend)
            .read(SecretSlot::OfficialAccountLogin)
            .expect("read official login secret")
            .as_deref(),
        Some("official-login-secret")
    );
}

#[test]
fn enriches_selected_articles_with_valid_reading_credentials() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    store
        .upsert_target_article(&target_article(
            "MzA123",
            "aid-1",
            "Reading fixture",
            "https://mp.weixin.qq.com/s/reading-fixture",
        ))
        .expect("store target article");
    let backend = MemorySecretBackend::default();
    ArticleReadingCredentialService::new(SecretStore::new(backend.clone()), NOW_UNIX)
        .save(ArticleReadingCredential {
            biz: "MzA123".to_string(),
            uin: "12345".to_string(),
            key: "reading-key".to_string(),
            pass_ticket: "pass-ticket".to_string(),
            appmsg_token: "appmsg-token".to_string(),
            cookie: Some("wap_sid2=secret".to_string()),
            expires_at_unix: NOW_UNIX + 3600,
        })
        .expect("save reading credential");
    let transport = FixtureReadingTransport::new(ArticleReadingEnrichmentFetchResponse {
        read_count: Some(1024),
        like_count: Some(81),
        share_count: Some(13),
        comment_count: Some(2),
        paid_content: Some("paid excerpt".to_string()),
        comments: vec![ArticleReadingCommentInput {
            comment_id: "comment-1".to_string(),
            author_name: "Reader A".to_string(),
            content: "很有启发".to_string(),
            like_count: 7,
            created_at_unix: 1_700_000_000,
            raw_json: Some(r#"{"id":"comment-1"}"#.to_string()),
        }],
    });
    let client =
        ArticleReadingEnrichmentClient::new(transport.clone(), SecretStore::new(backend), NOW_UNIX);

    let outcome = client
        .enrich_selected_articles(
            &store,
            ArticleReadingEnrichmentRequest {
                fakeid: "MzA123".to_string(),
                article_ids: vec!["aid-1".to_string()],
            },
        )
        .expect("enrich selected articles");

    assert_eq!(outcome.enriched_count, 1);
    assert_eq!(outcome.skipped_count, 0);
    assert_eq!(outcome.articles[0].article_id, "aid-1");

    let archived = store
        .get_article("aid-1")
        .expect("load archived article")
        .expect("article archive exists after enrichment");
    let stats = archived
        .reading_enrichment
        .as_ref()
        .expect("reading enrichment");
    assert_eq!(stats.read_count, Some(1024));
    assert_eq!(stats.like_count, Some(81));
    assert_eq!(stats.share_count, Some(13));
    assert_eq!(stats.comment_count, Some(2));
    assert_eq!(stats.paid_content.as_deref(), Some("paid excerpt"));
    assert_eq!(archived.reading_comments.len(), 1);
    assert_eq!(archived.reading_comments[0].content, "很有启发");

    let requests = transport.requests.lock().expect("read requests");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].article_id, "aid-1");
    assert_eq!(requests[0].credential.biz, "MzA123");
    assert_eq!(
        requests[0].source_url,
        "https://mp.weixin.qq.com/s/reading-fixture"
    );
}

#[test]
fn expired_reading_credentials_block_only_enrichment() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    store
        .upsert_target_article(&target_article(
            "MzA123",
            "aid-1",
            "Reading fixture",
            "https://mp.weixin.qq.com/s/reading-fixture",
        ))
        .expect("store target article");
    let backend = MemorySecretBackend::default();
    ArticleReadingCredentialService::new(SecretStore::new(backend.clone()), NOW_UNIX)
        .save(ArticleReadingCredential {
            biz: "MzA123".to_string(),
            uin: "12345".to_string(),
            key: "reading-key".to_string(),
            pass_ticket: "pass-ticket".to_string(),
            appmsg_token: "appmsg-token".to_string(),
            cookie: None,
            expires_at_unix: NOW_UNIX - 60,
        })
        .expect("save expired reading credential");
    let transport = FixtureReadingTransport::new(ArticleReadingEnrichmentFetchResponse {
        read_count: Some(1),
        like_count: None,
        share_count: None,
        comment_count: None,
        paid_content: None,
        comments: vec![],
    });
    let client =
        ArticleReadingEnrichmentClient::new(transport.clone(), SecretStore::new(backend), NOW_UNIX);

    let error = client
        .enrich_selected_articles(
            &store,
            ArticleReadingEnrichmentRequest {
                fakeid: "MzA123".to_string(),
                article_ids: vec!["aid-1".to_string()],
            },
        )
        .expect_err("expired credential should block enrichment");

    assert!(matches!(
        error,
        ArticleReadingEnrichmentError::MissingValidReadingCredential
    ));
    assert!(transport.requests.lock().expect("read requests").is_empty());
    assert!(store
        .list_target_articles("MzA123")
        .expect("list target articles")
        .iter()
        .any(|article| article.article_id == "aid-1"));
}

#[derive(Clone)]
struct FixtureReadingTransport {
    response: ArticleReadingEnrichmentFetchResponse,
    requests: Arc<Mutex<Vec<ArticleReadingEnrichmentFetchRequest>>>,
}

impl FixtureReadingTransport {
    fn new(response: ArticleReadingEnrichmentFetchResponse) -> Self {
        Self {
            response,
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ArticleReadingEnrichmentTransport for FixtureReadingTransport {
    fn fetch(
        &self,
        request: ArticleReadingEnrichmentFetchRequest,
    ) -> Result<ArticleReadingEnrichmentFetchResponse, String> {
        self.requests.lock().expect("record request").push(request);

        Ok(self.response.clone())
    }
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
        album_infos: vec![],
    }
}
