use std::fs;
use std::path::Path;

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    initialize_archive_store_from_app_data_dir, ArchiveArticleInput, ArchiveStore,
    ArchiveStoreConfig, ArchiveStoreSettings,
};

#[test]
fn initializes_sqlite_schema_and_archive_directories() {
    let temp = tempdir().expect("temp dir");
    let db_path = temp.path().join("archive.sqlite");
    let archive_dir = temp.path().join("archive");

    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: db_path.clone(),
        archive_dir: archive_dir.clone(),
    })
    .expect("open archive store");

    assert!(db_path.exists());
    assert!(archive_dir.is_dir());
    assert!(archive_dir.join("articles").is_dir());
    assert!(archive_dir.join("assets").is_dir());
    assert!(archive_dir.join("exports").is_dir());
    assert_eq!(store.schema_version().expect("schema version"), 5);
}

#[test]
fn initializes_from_app_data_directory_with_default_paths() {
    let temp = tempdir().expect("temp dir");

    let snapshot =
        initialize_archive_store_from_app_data_dir(temp.path()).expect("initialize archive store");

    assert_eq!(snapshot.database_path, temp.path().join("archive.sqlite"));
    assert_eq!(snapshot.archive_dir, temp.path().join("archive"));
    assert_eq!(snapshot.schema_version, 5);
    assert!(snapshot.database_path.exists());
    assert!(snapshot.archive_dir.join("articles").is_dir());
    assert!(snapshot.archive_dir.join("assets").is_dir());
    assert!(snapshot.archive_dir.join("exports").is_dir());
}

#[test]
fn persists_settings_after_reinitialization() {
    let temp = tempdir().expect("temp dir");
    let config = ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    };

    let store = ArchiveStore::open(config.clone()).expect("open archive store");
    store
        .save_settings(&ArchiveStoreSettings {
            archive_dir: config.archive_dir.clone(),
        })
        .expect("save settings");
    drop(store);

    let reopened = ArchiveStore::open(config.clone()).expect("reopen archive store");
    let settings = reopened.load_settings().expect("load settings");

    assert_eq!(settings.archive_dir, config.archive_dir);
}

#[test]
fn stores_article_records_with_relative_file_references() {
    let temp = tempdir().expect("temp dir");
    let config = ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    };
    let store = ArchiveStore::open(config.clone()).expect("open archive store");

    let html_path = Path::new("articles").join("2026").join("demo.html");
    let markdown_path = Path::new("articles").join("2026").join("demo.md");
    fs::create_dir_all(config.archive_dir.join("articles").join("2026"))
        .expect("create article directory");
    fs::write(
        config.archive_dir.join(&html_path),
        "<article>demo</article>",
    )
    .expect("write html");
    fs::write(config.archive_dir.join(&markdown_path), "# demo").expect("write markdown");

    store
        .upsert_article(&ArchiveArticleInput {
            article_id: "article-1".to_string(),
            target_account_id: "target-1".to_string(),
            title: "Demo article".to_string(),
            source_url: "https://mp.weixin.qq.com/s/example".to_string(),
            html_file: Some(html_path.clone()),
            markdown_file: Some(markdown_path.clone()),
        })
        .expect("upsert article");

    let article = store
        .get_article("article-1")
        .expect("load article")
        .expect("article exists");

    assert_eq!(article.html_file.as_deref(), Some(html_path.as_path()));
    assert_eq!(
        article.markdown_file.as_deref(),
        Some(markdown_path.as_path())
    );
    assert!(!article.html_file.expect("html file").is_absolute());
    assert!(!article.markdown_file.expect("markdown file").is_absolute());
}
