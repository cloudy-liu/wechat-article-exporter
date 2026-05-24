use std::collections::{HashMap, VecDeque};
use std::fs;
use std::sync::{Arc, Mutex};

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::album_workflow::{
    parse_appmsgalbum_response, AlbumArticleDownloadTransport, AlbumDownloadClient,
    AlbumPageClient, AlbumPageRequest, AlbumPageTransport, AlbumWorkflowService,
};
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveStore, ArchiveStoreConfig, CollectionTaskStatus, CollectionTaskType,
    TargetArticleAlbumInfo, TargetArticleInput,
};
use wechat_article_exporter_desktop_lib::article_export::ArticleExportFormat;
use wechat_article_exporter_desktop_lib::article_html_download::{
    ArticleHtmlDownloadResourceRequest, ArticleHtmlDownloadResourceResponse,
};
use wechat_article_exporter_desktop_lib::official_account_login::{
    OfficialAccountLoginProfile, OfficialAccountLoginSecret,
};
use wechat_article_exporter_desktop_lib::secret_store::{
    MemorySecretBackend, SecretSlot, SecretStore,
};

#[test]
fn stores_and_lists_unique_album_choices_from_synced_articles() {
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
            101,
            1,
            vec![album_info("album-1", "Growth Notes")],
        ))
        .expect("store first album article");
    store
        .upsert_target_article(&target_article(
            "fakeid-1",
            "aid-2",
            "Second Article",
            102,
            1,
            vec![
                album_info("album-1", "Growth Notes"),
                album_info("album-2", "Operator Cases"),
            ],
        ))
        .expect("store second album article");

    let albums = store
        .list_target_account_albums("fakeid-1")
        .expect("list target account albums");

    assert_eq!(albums.len(), 2);
    assert_eq!(albums[0].id, "album-1");
    assert_eq!(albums[0].title, "Growth Notes");
    assert_eq!(albums[1].id, "album-2");
    assert_eq!(albums[1].title, "Operator Cases");
}

#[test]
fn parses_representative_album_page_response_with_single_or_array_articles() {
    let raw = r#"
    {
      "base_resp": {"ret": 0, "err_msg": "ok"},
      "getalbum_resp": {
        "base_info": {
          "article_count": "2",
          "brand_icon": "https://example.test/brand.png",
          "cover": "https://example.test/cover.png",
          "description": "Curated operator examples",
          "nickname": "Research Account",
          "title": "Growth Notes",
          "username": "gh_fixture"
        },
        "article_list": [
          {
            "cover_img_1_1": "https://example.test/first.jpg",
            "create_time": "1700000010",
            "item_show_type": "0",
            "itemidx": "1",
            "msgid": "101",
            "title": "First Album Article",
            "url": "https://mp.weixin.qq.com/s/first"
          },
          {
            "cover_img_1_1": "https://example.test/second.jpg",
            "create_time": "1700000020",
            "item_show_type": "0",
            "itemidx": "2",
            "msgid": "102",
            "title": "Second Album Article",
            "url": "https://mp.weixin.qq.com/s/second"
          }
        ],
        "continue_flag": "1",
        "is_pay_subscribe": "0",
        "reverse_continue_flag": "0"
      }
    }
    "#;

    let page = parse_appmsgalbum_response("fakeid-1", "album-1", "Growth Notes", raw)
        .expect("parse album page");

    assert_eq!(page.base_info.title, "Growth Notes");
    assert_eq!(page.base_info.nickname, "Research Account");
    assert!(page.has_more);
    assert_eq!(page.articles.len(), 2);
    assert_eq!(page.articles[0].article_id, "101_1");
    assert_eq!(page.articles[0].appmsgid, 101);
    assert_eq!(page.articles[0].itemidx, 1);
    assert_eq!(
        page.articles[0].source_url,
        "https://mp.weixin.qq.com/s/first"
    );
    assert_eq!(page.articles[0].album_infos[0].id, "album-1");
}

#[test]
fn fetch_all_album_articles_paginates_with_last_msgid_and_itemidx_and_persists_links() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let backend = MemorySecretBackend::default();
    save_login_secret(&backend);
    let transport = FixtureAlbumPageTransport::new()
        .with_page(album_page_json("1", "101", "1", "First Album Article"))
        .with_page(album_page_json("0", "102", "1", "Second Album Article"));
    let client = AlbumPageClient::new(transport.clone(), SecretStore::new(backend));

    let outcome = client
        .fetch_all_articles(&store, "fakeid-1", "album-1", "Growth Notes", 1, false)
        .expect("fetch all album links");

    assert_eq!(outcome.album_id, "album-1");
    assert_eq!(outcome.articles.len(), 2);
    let stored = store
        .list_target_articles_by_album("fakeid-1", "album-1")
        .expect("list stored album articles");
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].article_id, "102_1");
    assert_eq!(stored[1].article_id, "101_1");

    let calls = transport.calls.lock().expect("read transport calls");
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].begin_msgid, None);
    assert_eq!(calls[0].begin_itemidx, None);
    assert_eq!(calls[0].count, 1);
    assert_eq!(calls[1].begin_msgid.as_deref(), Some("101"));
    assert_eq!(calls[1].begin_itemidx.as_deref(), Some("1"));
    assert_eq!(calls[1].cookie_header, "wxuin=1; rand_info=secret");

    let task = store
        .list_collection_tasks()
        .expect("list collection tasks")
        .into_iter()
        .find(|task| task.task_type == CollectionTaskType::AlbumDownload)
        .expect("album link task exists");
    assert_eq!(task.status, CollectionTaskStatus::Succeeded);
    assert_eq!(task.succeeded_items, 2);
}

#[test]
fn batch_download_and_export_album_articles_use_local_archive_and_persistent_task_state() {
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
            "101_1",
            "First Album Article",
            101,
            1,
            vec![album_info("album-1", "Growth Notes")],
        ))
        .expect("store first album article");
    store
        .upsert_target_article(&target_article(
            "fakeid-1",
            "102_1",
            "Second Album Article",
            102,
            1,
            vec![album_info("album-1", "Growth Notes")],
        ))
        .expect("store second album article");
    let transport = FixtureAlbumDownloadTransport::default()
        .with_html(
            "https://mp.weixin.qq.com/s/101_1",
            article_html("First Album Article"),
        )
        .with_html(
            "https://mp.weixin.qq.com/s/102_1",
            article_html("Second Album Article"),
        );
    let client = AlbumDownloadClient::new(transport, SecretStore::new(backend));

    let download_outcomes = client
        .download_album_articles(&store, "fakeid-1", "album-1", None)
        .expect("download album articles");

    assert_eq!(download_outcomes.len(), 2);
    for outcome in &download_outcomes {
        assert!(store.archive_dir().join(&outcome.html_file).is_file());
    }

    let export_outcomes = AlbumWorkflowService::new()
        .export_album_articles(
            &store,
            "fakeid-1",
            "album-1",
            vec![ArticleExportFormat::Markdown, ArticleExportFormat::Html],
            None,
        )
        .expect("export album articles");

    assert_eq!(export_outcomes.len(), 2);
    for outcome in &export_outcomes {
        assert!(store
            .archive_dir()
            .join(outcome.markdown_file.as_ref().expect("markdown export"))
            .is_file());
        assert!(store
            .archive_dir()
            .join(outcome.html_file.as_ref().expect("html export"))
            .is_file());
    }
    let markdown = fs::read_to_string(
        store.archive_dir().join(
            export_outcomes[0]
                .markdown_file
                .as_ref()
                .expect("markdown export"),
        ),
    )
    .expect("read exported markdown");
    assert!(markdown.contains("# Second Album Article"));

    let tasks = store
        .list_collection_tasks()
        .expect("list collection tasks");
    let download_task = tasks
        .iter()
        .find(|task| {
            task.task_type == CollectionTaskType::AlbumDownload
                && task
                    .items
                    .iter()
                    .any(|item| item.item_type == "album-article-html")
        })
        .expect("album download task exists");
    assert_eq!(download_task.status, CollectionTaskStatus::Succeeded);
    assert_eq!(download_task.succeeded_items, 2);

    let export_task = tasks
        .iter()
        .find(|task| {
            task.task_type == CollectionTaskType::Export
                && task
                    .items
                    .iter()
                    .any(|item| item.item_type == "album-article-export")
        })
        .expect("album export task exists");
    assert_eq!(export_task.status, CollectionTaskStatus::Succeeded);
    assert_eq!(export_task.succeeded_items, 2);
}

fn target_article(
    target_account_id: &str,
    article_id: &str,
    title: &str,
    appmsgid: i64,
    itemidx: i64,
    album_infos: Vec<TargetArticleAlbumInfo>,
) -> TargetArticleInput {
    TargetArticleInput {
        article_id: article_id.to_string(),
        target_account_id: target_account_id.to_string(),
        title: title.to_string(),
        source_url: format!("https://mp.weixin.qq.com/s/{article_id}"),
        digest: format!("{title} digest"),
        author_name: "Fixture Author".to_string(),
        cover: format!("https://example.test/{article_id}.jpg"),
        appmsgid,
        itemidx,
        item_show_type: 0,
        create_time: 1_700_000_000 + appmsgid,
        update_time: 1_700_000_100 + appmsgid,
        is_deleted: false,
        copyright_type: 0,
        album_infos,
    }
}

fn album_info(id: &str, title: &str) -> TargetArticleAlbumInfo {
    TargetArticleAlbumInfo {
        album_id: id.parse::<i64>().unwrap_or_default(),
        id: id.to_string(),
        tag_source: 0,
        title: title.to_string(),
    }
}

fn album_page_json(continue_flag: &str, msgid: &str, itemidx: &str, title: &str) -> String {
    format!(
        r#"{{
          "base_resp": {{"ret": 0, "err_msg": "ok"}},
          "getalbum_resp": {{
            "base_info": {{
              "article_count": "2",
              "brand_icon": "https://example.test/brand.png",
              "cover": "https://example.test/cover.png",
              "description": "Curated operator examples",
              "nickname": "Research Account",
              "title": "Growth Notes",
              "username": "gh_fixture"
            }},
            "article_list": {{
              "cover_img_1_1": "https://example.test/{msgid}.jpg",
              "create_time": "{create_time}",
              "item_show_type": "0",
              "itemidx": "{itemidx}",
              "msgid": "{msgid}",
              "title": "{title}",
              "url": "https://mp.weixin.qq.com/s/{msgid}_{itemidx}"
            }},
            "continue_flag": "{continue_flag}",
            "is_pay_subscribe": "0",
            "reverse_continue_flag": "0"
          }}
        }}"#,
        create_time = 1_700_000_000_i64 + msgid.parse::<i64>().expect("msgid")
    )
}

fn article_html(title: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
  <body>
    <article id="js_article">
      <h1>{title}</h1>
      <div id="js_content"><p>Useful album paragraph.</p></div>
    </article>
  </body>
</html>"#
    )
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
struct FixtureAlbumPageTransport {
    calls: Arc<Mutex<Vec<AlbumPageRequest>>>,
    pages: Arc<Mutex<VecDeque<String>>>,
}

impl FixtureAlbumPageTransport {
    fn new() -> Self {
        Self::default()
    }

    fn with_page(self, page: String) -> Self {
        self.pages.lock().expect("lock pages").push_back(page);
        self
    }
}

impl AlbumPageTransport for FixtureAlbumPageTransport {
    fn fetch_page(&self, request: AlbumPageRequest) -> Result<String, String> {
        self.calls.lock().expect("lock calls").push(request);
        self.pages
            .lock()
            .expect("lock pages")
            .pop_front()
            .ok_or_else(|| "missing fixture album page".to_string())
    }
}

#[derive(Clone, Default)]
struct FixtureAlbumDownloadTransport {
    requests: Arc<Mutex<Vec<ArticleHtmlDownloadResourceRequest>>>,
    html_by_url: Arc<Mutex<HashMap<String, String>>>,
}

impl FixtureAlbumDownloadTransport {
    fn with_html(self, url: &str, html: String) -> Self {
        self.html_by_url
            .lock()
            .expect("lock html map")
            .insert(url.to_string(), html);
        self
    }
}

impl AlbumArticleDownloadTransport for FixtureAlbumDownloadTransport {
    fn fetch(
        &self,
        request: ArticleHtmlDownloadResourceRequest,
    ) -> Result<ArticleHtmlDownloadResourceResponse, String> {
        self.requests
            .lock()
            .expect("lock requests")
            .push(request.clone());
        let html = self
            .html_by_url
            .lock()
            .expect("lock html map")
            .get(&request.url)
            .cloned()
            .ok_or_else(|| format!("missing fixture html: {}", request.url))?;

        Ok(ArticleHtmlDownloadResourceResponse {
            bytes: html.into_bytes(),
            content_type: Some("text/html; charset=utf-8".to_string()),
        })
    }
}
