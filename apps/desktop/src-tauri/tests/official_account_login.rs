use std::sync::{Arc, Mutex};

use wechat_article_exporter_desktop_lib::official_account_login::{
    InMemoryLoginSessionStore, LoginFinalizePayload, LoginPollPayload, LoginScanStatus,
    OfficialAccountLoginClient, OfficialAccountLoginProfile, OfficialAccountLoginSecret,
    OfficialAccountLoginTransport, StartLoginPayload,
};
use wechat_article_exporter_desktop_lib::secret_store::{
    MemorySecretBackend, SecretSlot, SecretStore,
};

#[test]
fn starts_login_session_with_qr_code_from_transport() {
    let transport = FixtureLoginTransport::new()
        .with_started_session("fixture-session", "data:image/png;base64,fixture-qr");
    let secret_store = SecretStore::new(MemorySecretBackend::default());
    let client = OfficialAccountLoginClient::new(
        transport,
        InMemoryLoginSessionStore::default(),
        secret_store,
    );

    let session = client.start_login().expect("start login");

    assert_eq!(session.session_id, "fixture-session");
    assert_eq!(session.qr_code_data_url, "data:image/png;base64,fixture-qr");
    assert_eq!(
        session.message,
        "Scan this QR code with a WeChat Official Account operator account."
    );
}

#[test]
fn polls_waiting_confirmed_expired_and_error_states() {
    let transport = FixtureLoginTransport::new()
        .with_started_session("fixture-session", "data:image/png;base64,fixture-qr")
        .with_poll_status(LoginScanStatus::Waiting)
        .with_poll_status(LoginScanStatus::Confirmed { account_count: 1 })
        .with_poll_status(LoginScanStatus::Expired)
        .with_poll_status(LoginScanStatus::Error {
            message: "fixture transport failed".to_string(),
        });
    let secret_store = SecretStore::new(MemorySecretBackend::default());
    let client = OfficialAccountLoginClient::new(
        transport,
        InMemoryLoginSessionStore::default(),
        secret_store,
    );
    let session = client.start_login().expect("start login");

    assert_eq!(
        client
            .poll_scan_status(&session.session_id)
            .expect("poll waiting"),
        LoginScanStatus::Waiting
    );
    assert_eq!(
        client
            .poll_scan_status(&session.session_id)
            .expect("poll confirmed"),
        LoginScanStatus::Confirmed { account_count: 1 }
    );
    assert_eq!(
        client
            .poll_scan_status(&session.session_id)
            .expect("poll expired"),
        LoginScanStatus::Expired
    );
    assert_eq!(
        client
            .poll_scan_status(&session.session_id)
            .expect("poll error"),
        LoginScanStatus::Error {
            message: "fixture transport failed".to_string(),
        }
    );
}

#[test]
fn finalizes_login_and_saves_official_account_secret() {
    let backend = MemorySecretBackend::default();
    let secret_store = SecretStore::new(backend.clone());
    let transport = FixtureLoginTransport::new()
        .with_started_session("fixture-session", "data:image/png;base64,fixture-qr")
        .with_finalized_login(OfficialAccountLoginSecret {
            token: "token-123".to_string(),
            cookie_header: "wxuin=1; rand_info=secret".to_string(),
            profile: OfficialAccountLoginProfile {
                nickname: "Fixture Official Account".to_string(),
                avatar_url: "https://example.test/avatar.png".to_string(),
            },
            expires_at: "2026-05-27T00:00:00Z".to_string(),
        });
    let client = OfficialAccountLoginClient::new(
        transport,
        InMemoryLoginSessionStore::default(),
        secret_store,
    );
    let session = client.start_login().expect("start login");

    let account = client
        .finalize_login(&session.session_id)
        .expect("finalize login");

    assert_eq!(account.nickname, "Fixture Official Account");
    assert_eq!(account.avatar_url, "https://example.test/avatar.png");
    let stored_secret = SecretStore::new(backend)
        .read(SecretSlot::OfficialAccountLogin)
        .expect("read login secret")
        .expect("login secret exists");
    assert!(stored_secret.contains("\"token\":\"token-123\""));
    assert!(stored_secret.contains("\"cookieHeader\":\"wxuin=1; rand_info=secret\""));
    assert!(stored_secret.contains("\"nickname\":\"Fixture Official Account\""));
}

#[test]
fn logout_clears_official_account_login_without_touching_reading_credentials() {
    let backend = MemorySecretBackend::default();
    let secret_store = SecretStore::new(backend.clone());
    secret_store
        .save(SecretSlot::ArticleReadingCredential, "reading-cookie")
        .expect("save reading credential");
    let transport = FixtureLoginTransport::new()
        .with_started_session("fixture-session", "data:image/png;base64,fixture-qr")
        .with_finalized_login(OfficialAccountLoginSecret {
            token: "token-123".to_string(),
            cookie_header: "wxuin=1; rand_info=secret".to_string(),
            profile: OfficialAccountLoginProfile {
                nickname: "Fixture Official Account".to_string(),
                avatar_url: "https://example.test/avatar.png".to_string(),
            },
            expires_at: "2026-05-27T00:00:00Z".to_string(),
        });
    let client = OfficialAccountLoginClient::new(
        transport,
        InMemoryLoginSessionStore::default(),
        secret_store,
    );
    let session = client.start_login().expect("start login");
    client
        .finalize_login(&session.session_id)
        .expect("finalize login");

    client.logout().expect("logout");

    let store = SecretStore::new(backend);
    assert_eq!(
        store
            .read(SecretSlot::OfficialAccountLogin)
            .expect("read login secret"),
        None
    );
    assert_eq!(
        store
            .read(SecretSlot::ArticleReadingCredential)
            .expect("read reading credential")
            .as_deref(),
        Some("reading-cookie")
    );
}

#[test]
fn rejects_unknown_or_expired_sessions_before_polling_transport() {
    let transport = FixtureLoginTransport::new();
    let secret_store = SecretStore::new(MemorySecretBackend::default());
    let client = OfficialAccountLoginClient::new(
        transport,
        InMemoryLoginSessionStore::default(),
        secret_store,
    );

    let error = client
        .poll_scan_status("missing-session")
        .expect_err("missing session should fail");

    assert!(error.to_string().contains("login session was not found"));
}

#[derive(Clone, Default)]
struct FixtureLoginTransport {
    state: Arc<Mutex<FixtureLoginTransportState>>,
}

#[derive(Default)]
struct FixtureLoginTransportState {
    start: Option<StartLoginPayload>,
    polls: Vec<LoginScanStatus>,
    finalized: Option<LoginFinalizePayload>,
}

impl FixtureLoginTransport {
    fn new() -> Self {
        Self::default()
    }

    fn with_started_session(self, session_id: &str, qr_code_data_url: &str) -> Self {
        self.state.lock().expect("lock fixture").start = Some(StartLoginPayload {
            session_id: session_id.to_string(),
            qr_code_data_url: qr_code_data_url.to_string(),
        });
        self
    }

    fn with_poll_status(self, status: LoginScanStatus) -> Self {
        self.state.lock().expect("lock fixture").polls.push(status);
        self
    }

    fn with_finalized_login(self, secret: OfficialAccountLoginSecret) -> Self {
        self.state.lock().expect("lock fixture").finalized = Some(LoginFinalizePayload { secret });
        self
    }
}

impl OfficialAccountLoginTransport for FixtureLoginTransport {
    fn start_login(&self) -> Result<StartLoginPayload, String> {
        self.state
            .lock()
            .expect("lock fixture")
            .start
            .clone()
            .ok_or_else(|| "fixture start response is missing".to_string())
    }

    fn poll_scan_status(&self, _session_id: &str) -> Result<LoginPollPayload, String> {
        let status = self.state.lock().expect("lock fixture").polls.remove(0);

        Ok(LoginPollPayload { status })
    }

    fn finalize_login(&self, _session_id: &str) -> Result<LoginFinalizePayload, String> {
        self.state
            .lock()
            .expect("lock fixture")
            .finalized
            .clone()
            .ok_or_else(|| "fixture finalized response is missing".to_string())
    }
}
