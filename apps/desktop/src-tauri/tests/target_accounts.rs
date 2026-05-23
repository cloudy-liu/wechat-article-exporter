use std::sync::{Arc, Mutex};

use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveStore, ArchiveStoreConfig, TargetAccountExport, TargetAccountInput,
};
use wechat_article_exporter_desktop_lib::official_account_login::{
    OfficialAccountLoginProfile, OfficialAccountLoginSecret,
};
use wechat_article_exporter_desktop_lib::secret_store::{
    MemorySecretBackend, SecretSlot, SecretStore,
};
use wechat_article_exporter_desktop_lib::target_accounts::{
    TargetAccountSearchClient, TargetAccountSearchRequest, TargetAccountSearchResponse,
    TargetAccountSearchTransport,
};

#[test]
fn target_accounts_can_be_added_listed_after_reopen_exported_imported_and_deleted() {
    let temp = tempdir().expect("temp dir");
    let config = ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    };
    let store = ArchiveStore::open(config.clone()).expect("open archive store");

    store
        .upsert_target_account(&target_account("fakeid-1", "Alpha Official"))
        .expect("add first target account");
    store
        .upsert_target_account(&target_account("fakeid-2", "Beta Official"))
        .expect("add second target account");

    assert_eq!(
        store.list_target_accounts().expect("list accounts").len(),
        2
    );
    drop(store);

    let reopened = ArchiveStore::open(config.clone()).expect("reopen archive store");
    let accounts = reopened
        .list_target_accounts()
        .expect("list accounts after reopen");
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].fakeid, "fakeid-1");
    assert_eq!(accounts[1].fakeid, "fakeid-2");

    let exported = reopened
        .export_target_accounts()
        .expect("export target accounts");
    assert_eq!(
        exported.format,
        "wechat-article-exporter.target-accounts.v1"
    );
    assert_eq!(exported.accounts.len(), 2);

    reopened
        .delete_target_account("fakeid-1")
        .expect("delete target account");
    let remaining = reopened
        .list_target_accounts()
        .expect("list accounts after delete");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].fakeid, "fakeid-2");

    let import_temp = tempdir().expect("import temp dir");
    let import_store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: import_temp.path().join("archive.sqlite"),
        archive_dir: import_temp.path().join("archive"),
    })
    .expect("open import archive store");
    import_store
        .import_target_accounts(&exported)
        .expect("import target accounts");
    let imported = import_store
        .list_target_accounts()
        .expect("list imported target accounts");
    assert_eq!(imported.len(), 2);
    assert_eq!(imported[0].nickname, "Alpha Official");
    assert_eq!(imported[1].nickname, "Beta Official");
}

#[test]
fn target_account_import_rejects_unknown_export_format() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");

    let error = store
        .import_target_accounts(&TargetAccountExport {
            format: "unknown-format".to_string(),
            accounts: vec![target_account("fakeid-1", "Alpha Official")],
        })
        .expect_err("unknown export format should fail");

    assert!(error
        .to_string()
        .contains("unsupported target account export format"));
}

#[test]
fn target_account_search_uses_stored_official_account_login_secret() {
    let backend = MemorySecretBackend::default();
    let secret_store = SecretStore::new(backend.clone());
    let login_secret = OfficialAccountLoginSecret {
        token: "token-123".to_string(),
        cookie_header: "wxuin=1; rand_info=secret".to_string(),
        profile: OfficialAccountLoginProfile {
            nickname: "Operator Official Account".to_string(),
            avatar_url: "https://example.test/operator.png".to_string(),
        },
        expires_at: "2026-05-27T00:00:00Z".to_string(),
    };
    secret_store
        .save(
            SecretSlot::OfficialAccountLogin,
            &serde_json::to_string(&login_secret).expect("serialize login secret"),
        )
        .expect("save login secret");
    let transport =
        FixtureTargetAccountSearchTransport::new().with_response(TargetAccountSearchResponse {
            total: 1,
            list: vec![target_account("fakeid-1", "Alpha Official")],
        });
    let client = TargetAccountSearchClient::new(transport.clone(), SecretStore::new(backend));

    let result = client
        .search("alpha", 0, 5)
        .expect("search target accounts");

    assert_eq!(result.total, 1);
    assert_eq!(result.list[0].fakeid, "fakeid-1");
    let calls = transport.calls.lock().expect("read fixture calls");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].keyword, "alpha");
    assert_eq!(calls[0].begin, 0);
    assert_eq!(calls[0].count, 5);
    assert_eq!(calls[0].token, "token-123");
    assert_eq!(calls[0].cookie_header, "wxuin=1; rand_info=secret");
}

#[test]
fn target_account_search_fails_without_official_account_login() {
    let client = TargetAccountSearchClient::new(
        FixtureTargetAccountSearchTransport::new(),
        SecretStore::new(MemorySecretBackend::default()),
    );

    let error = client
        .search("alpha", 0, 5)
        .expect_err("missing login should fail");

    assert!(error
        .to_string()
        .contains("Official Account Login is required"));
}

fn target_account(fakeid: &str, nickname: &str) -> TargetAccountInput {
    TargetAccountInput {
        fakeid: fakeid.to_string(),
        nickname: nickname.to_string(),
        alias: format!("{fakeid}-alias"),
        round_head_img: format!("https://example.test/{fakeid}.png"),
        service_type: 1,
        signature: format!("{nickname} signature"),
    }
}

#[derive(Clone, Default)]
struct FixtureTargetAccountSearchTransport {
    calls: Arc<Mutex<Vec<TargetAccountSearchRequest>>>,
    response: Arc<Mutex<Option<TargetAccountSearchResponse>>>,
}

impl FixtureTargetAccountSearchTransport {
    fn new() -> Self {
        Self::default()
    }

    fn with_response(self, response: TargetAccountSearchResponse) -> Self {
        *self.response.lock().expect("lock response") = Some(response);
        self
    }
}

impl TargetAccountSearchTransport for FixtureTargetAccountSearchTransport {
    fn search(
        &self,
        request: TargetAccountSearchRequest,
    ) -> Result<TargetAccountSearchResponse, String> {
        self.calls.lock().expect("lock calls").push(request);
        self.response
            .lock()
            .expect("lock response")
            .clone()
            .ok_or_else(|| "fixture response is missing".to_string())
    }
}
