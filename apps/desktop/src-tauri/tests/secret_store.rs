use std::fs;

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveArticleInput, ArchiveStore, ArchiveStoreConfig, ArchiveStoreSettings,
};
use wechat_article_exporter_desktop_lib::secret_store::{
    MemorySecretBackend, SecretSlot, SecretStore,
};

#[test]
fn stores_login_and_reading_credentials_under_distinct_slots() {
    let store = SecretStore::new(MemorySecretBackend::default());

    store
        .save(SecretSlot::OfficialAccountLogin, "official-cookie-token")
        .expect("save official account login secret");
    store
        .save(SecretSlot::ArticleReadingCredential, "reading-side-cookie")
        .expect("save article reading credential");

    assert_eq!(
        store
            .read(SecretSlot::OfficialAccountLogin)
            .expect("read official account login secret")
            .as_deref(),
        Some("official-cookie-token")
    );
    assert_eq!(
        store
            .read(SecretSlot::ArticleReadingCredential)
            .expect("read article reading credential")
            .as_deref(),
        Some("reading-side-cookie")
    );
}

#[test]
fn deletes_individual_secret_without_touching_other_credentials() {
    let store = SecretStore::new(MemorySecretBackend::default());

    store
        .save(SecretSlot::OfficialAccountLogin, "official-cookie-token")
        .expect("save official account login secret");
    store
        .save(SecretSlot::ArticleReadingCredential, "reading-side-cookie")
        .expect("save article reading credential");

    store
        .delete(SecretSlot::OfficialAccountLogin)
        .expect("delete official account login secret");

    assert_eq!(
        store
            .read(SecretSlot::OfficialAccountLogin)
            .expect("read deleted official account login secret"),
        None
    );
    assert_eq!(
        store
            .read(SecretSlot::ArticleReadingCredential)
            .expect("read article reading credential")
            .as_deref(),
        Some("reading-side-cookie")
    );
}

#[test]
fn clears_all_known_credentials_for_logout_cleanup() {
    let store = SecretStore::new(MemorySecretBackend::default());

    store
        .save(SecretSlot::OfficialAccountLogin, "official-cookie-token")
        .expect("save official account login secret");
    store
        .save(SecretSlot::ArticleReadingCredential, "reading-side-cookie")
        .expect("save article reading credential");

    store.clear_credentials().expect("clear credentials");

    assert_eq!(
        store
            .read(SecretSlot::OfficialAccountLogin)
            .expect("read cleared official account login secret"),
        None
    );
    assert_eq!(
        store
            .read(SecretSlot::ArticleReadingCredential)
            .expect("read cleared article reading credential"),
        None
    );
}

#[test]
fn does_not_write_plaintext_credentials_to_archive_store_paths() {
    let temp = tempdir().expect("temp dir");
    let archive_config = ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    };
    let archive_store = ArchiveStore::open(archive_config.clone()).expect("open archive store");
    let secret_store = SecretStore::new(MemorySecretBackend::default());
    let login_secret = "wx-login-cookie-token-secret-value";
    let reading_secret = "wx-reading-cookie-token-secret-value";

    archive_store
        .save_settings(&ArchiveStoreSettings::default_for_archive_dir(
            archive_config.archive_dir.clone(),
        ))
        .expect("save archive settings");
    archive_store
        .upsert_article(&ArchiveArticleInput {
            article_id: "article-1".to_string(),
            target_account_id: "target-1".to_string(),
            title: "Article without credentials".to_string(),
            source_url: "https://mp.weixin.qq.com/s/example".to_string(),
            html_file: Some("articles/example.html".into()),
            markdown_file: Some("articles/example.md".into()),
        })
        .expect("save article");
    secret_store
        .save(SecretSlot::OfficialAccountLogin, login_secret)
        .expect("save login secret");
    secret_store
        .save(SecretSlot::ArticleReadingCredential, reading_secret)
        .expect("save reading secret");

    let mut scanned_text = String::new();
    scanned_text.push_str(
        &fs::read_to_string(&archive_config.database_path).unwrap_or_else(|_| String::new()),
    );
    collect_file_text(&archive_config.archive_dir, &mut scanned_text);

    assert!(!scanned_text.contains(login_secret));
    assert!(!scanned_text.contains(reading_secret));
}

fn collect_file_text(dir: &std::path::Path, out: &mut String) {
    if !dir.exists() {
        return;
    }

    for entry in fs::read_dir(dir).expect("read directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_file_text(&path, out);
        } else if let Ok(text) = fs::read_to_string(path) {
            out.push_str(&text);
        }
    }
}
