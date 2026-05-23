use std::fs;
use std::path::Path;

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveArticleInput, ArchiveStore, ArchiveStoreConfig,
};
use wechat_article_exporter_desktop_lib::article_export::{
    ArticleArchivePreviewRequest, ArticleArchivePreviewService,
};

#[test]
fn previews_downloaded_article_with_clean_local_html() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let html_file = Path::new("articles").join("fakeid-1").join("aid-1.html");
    fs::create_dir_all(
        store
            .archive_dir()
            .join(html_file.parent().expect("html parent")),
    )
    .expect("create html parent");
    fs::write(
        store.archive_dir().join(&html_file),
        r#"<!doctype html>
<html>
  <body>
    <div id="outside">runtime noise</div>
    <article id="js_article">
      <h1>Preview title</h1>
      <div id="js_top_ad_area">ad noise</div>
      <div id="js_content" style="visibility:hidden"><p>Readable body</p></div>
      <script>runtime noise</script>
    </article>
  </body>
</html>"#,
    )
    .expect("write html");
    store
        .upsert_article(&ArchiveArticleInput {
            article_id: "aid-1".to_string(),
            target_account_id: "fakeid-1".to_string(),
            title: "Preview title".to_string(),
            source_url: "https://mp.weixin.qq.com/s/aid-1".to_string(),
            html_file: Some(html_file.clone()),
            markdown_file: None,
        })
        .expect("store article archive");

    let preview = ArticleArchivePreviewService::new()
        .preview_article(
            &store,
            ArticleArchivePreviewRequest {
                article_id: "aid-1".to_string(),
            },
        )
        .expect("preview article");

    assert_eq!(preview.article_id, "aid-1");
    assert_eq!(preview.title, "Preview title");
    assert_eq!(preview.source_html_file, html_file);
    assert!(preview.html.contains("<article id=\"js_article\""));
    assert!(preview.html.contains("Readable body"));
    assert!(!preview.html.contains("runtime noise"));
    assert!(!preview.html.contains("js_top_ad_area"));
    assert!(!preview.html.contains("style=\"visibility:hidden\""));
}
