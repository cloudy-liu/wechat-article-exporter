use std::fs;
use std::path::Path;

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveArticleInput, ArchiveStore, ArchiveStoreConfig, CollectionTaskStatus, CollectionTaskType,
};
use wechat_article_exporter_desktop_lib::article_export::{
    ArticleExportFormat, ArticleExportRequest, ArticleExportService,
};

#[test]
fn exports_downloaded_article_as_clean_markdown_and_html_with_local_assets() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let html_file = Path::new("articles")
        .join("fakeid-1")
        .join("aid-danger.html");
    let asset_file = Path::new("assets")
        .join("fakeid-1")
        .join("aid-danger")
        .join("000-cover.png");
    fs::create_dir_all(
        store
            .archive_dir()
            .join(html_file.parent().expect("html parent")),
    )
    .expect("create html parent");
    fs::create_dir_all(
        store
            .archive_dir()
            .join(asset_file.parent().expect("asset parent")),
    )
    .expect("create asset parent");
    fs::write(store.archive_dir().join(&asset_file), b"cover").expect("write asset");
    fs::write(
        store.archive_dir().join(&html_file),
        representative_article_html(),
    )
    .expect("write downloaded html");
    store
        .upsert_article(&ArchiveArticleInput {
            article_id: "aid-danger".to_string(),
            target_account_id: "fakeid-1".to_string(),
            title: "A/B: Clean * Export?".to_string(),
            source_url: "https://mp.weixin.qq.com/s/aid-danger".to_string(),
            html_file: Some(html_file.clone()),
            markdown_file: None,
        })
        .expect("store article archive");

    let outcome = ArticleExportService::new()
        .export_article(
            &store,
            ArticleExportRequest {
                article_id: "aid-danger".to_string(),
                formats: vec![ArticleExportFormat::Markdown, ArticleExportFormat::Html],
            },
        )
        .expect("export downloaded article");

    assert_eq!(outcome.article_id, "aid-danger");
    let markdown_file = outcome.markdown_file.as_ref().expect("markdown export");
    let export_html_file = outcome.html_file.as_ref().expect("html export");
    assert!(markdown_file.ends_with("A_B_Clean_Export-aid-danger.md"));
    assert!(export_html_file.ends_with("A_B_Clean_Export-aid-danger.html"));
    assert!(store.archive_dir().join(markdown_file).is_file());
    assert!(store.archive_dir().join(export_html_file).is_file());
    assert!(!markdown_file.is_absolute());
    assert!(!export_html_file.is_absolute());

    let markdown = fs::read_to_string(store.archive_dir().join(markdown_file)).expect("read md");
    assert!(markdown.contains("# A/B: Clean * Export?"));
    assert!(markdown.contains("Useful paragraph with bold text."));
    assert!(markdown.contains("![Cover](../assets/fakeid-1/aid-danger/000-cover.png)"));
    assert!(!markdown.contains("runtime noise"));
    assert!(!markdown.contains("js_top_ad_area"));
    assert!(!markdown.contains("<script"));

    let html = fs::read_to_string(store.archive_dir().join(export_html_file)).expect("read html");
    assert!(html.contains("<article id=\"js_article\""));
    assert!(html.contains("src=\"../assets/fakeid-1/aid-danger/000-cover.png\""));
    assert!(!html.contains("runtime noise"));
    assert!(!html.contains("js_top_ad_area"));
    assert!(!html.contains("<script"));

    let archived = store
        .get_article("aid-danger")
        .expect("load article")
        .expect("article exists");
    assert_eq!(archived.html_file.as_deref(), Some(html_file.as_path()));
    assert_eq!(
        archived.markdown_file.as_deref(),
        Some(markdown_file.as_path())
    );

    let task = store
        .list_collection_tasks()
        .expect("list tasks")
        .into_iter()
        .find(|task| task.task_type == CollectionTaskType::Export)
        .expect("export task exists");
    assert_eq!(task.status, CollectionTaskStatus::Succeeded);
    assert_eq!(task.succeeded_items, 1);
    assert_eq!(task.items[0].item_type, "article-export");
}

#[test]
fn export_fails_with_persistent_task_state_when_downloaded_html_is_missing() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    store
        .upsert_article(&ArchiveArticleInput {
            article_id: "aid-missing".to_string(),
            target_account_id: "fakeid-1".to_string(),
            title: "Missing HTML".to_string(),
            source_url: "https://mp.weixin.qq.com/s/aid-missing".to_string(),
            html_file: Some("articles/fakeid-1/aid-missing.html".into()),
            markdown_file: None,
        })
        .expect("store archive record");

    let error = ArticleExportService::new()
        .export_article(
            &store,
            ArticleExportRequest {
                article_id: "aid-missing".to_string(),
                formats: vec![ArticleExportFormat::Markdown],
            },
        )
        .expect_err("missing downloaded html should fail");

    assert!(error
        .to_string()
        .contains("downloaded article HTML is missing"));
    let task = store
        .list_collection_tasks()
        .expect("list tasks")
        .into_iter()
        .find(|task| task.task_type == CollectionTaskType::Export)
        .expect("export task exists");
    assert_eq!(task.status, CollectionTaskStatus::Failed);
    assert_eq!(task.failed_items, 1);
    assert!(task
        .error_message
        .as_deref()
        .expect("task error")
        .contains("downloaded article HTML is missing"));
}

fn representative_article_html() -> &'static str {
    r#"<!doctype html>
<html>
  <head><title>runtime noise</title></head>
  <body class="rich_media_empty_extra">
    <div id="outside">runtime noise</div>
    <article id="js_article">
      <h1 id="activity-name">A/B: Clean * Export?</h1>
      <div id="js_top_ad_area">runtime noise</div>
      <div id="js_content" style="visibility:hidden">
        <p>Useful paragraph with <strong>bold text</strong>.</p>
        <img data-src="https://mmbiz.qpic.cn/mmbiz_png/cover.png" />
        <script>runtime noise</script>
      </div>
      <div id="content_bottom_area">runtime noise</div>
    </article>
    <script>runtime noise</script>
  </body>
</html>"#
}
