pub mod archive_store;
pub mod article_export;
pub mod article_html_download;
pub mod article_list_sync;
pub mod official_account_login;
pub mod secret_store;
pub mod target_accounts;

use archive_store::{
    ArchiveStore, ArchiveStoreConfig, ArchiveStoreSnapshot, ArticleListSyncRecord, CollectionTask,
    CollectionTaskStatus, TargetAccountExport, TargetAccountInput, TargetArticleInput,
};
use article_export::{
    ArticleArchivePreview, ArticleArchivePreviewRequest, ArticleArchivePreviewService,
    ArticleExportOutcome, ArticleExportRequest, ArticleExportService,
};
use article_html_download::{
    ArticleHtmlDownloadOutcome, ArticleHtmlDownloadRequest, ArticleHtmlDownloadState,
};
use article_list_sync::ArticleListSyncState;
use official_account_login::{
    LoginScanStatus, OfficialAccountLoginAccount, OfficialAccountLoginSession,
    OfficialAccountLoginState,
};
use secret_store::SecretSlot;
use target_accounts::{TargetAccountSearchResponse, TargetAccountSearchState};
use tauri::Manager;

#[tauri::command]
fn initialize_archive_store(app: tauri::AppHandle) -> Result<ArchiveStoreSnapshot, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;

    archive_store::initialize_archive_store_from_app_data_dir(app_data_dir)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn search_target_accounts(
    state: tauri::State<TargetAccountSearchState>,
    keyword: String,
    begin: u32,
    count: u32,
) -> Result<TargetAccountSearchResponse, String> {
    let client = target_accounts::TargetAccountSearchClient::new(
        state.transport.clone(),
        secret_store::production_secret_store(),
    );

    client
        .search(&keyword, begin, count)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn add_target_account(app: tauri::AppHandle, account: TargetAccountInput) -> Result<(), String> {
    open_archive_store(&app)?
        .upsert_target_account(&account)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_target_accounts(app: tauri::AppHandle) -> Result<Vec<TargetAccountInput>, String> {
    open_archive_store(&app)?
        .list_target_accounts()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_target_account(app: tauri::AppHandle, fakeid: String) -> Result<(), String> {
    open_archive_store(&app)?
        .delete_target_account(&fakeid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn export_target_accounts(app: tauri::AppHandle) -> Result<TargetAccountExport, String> {
    open_archive_store(&app)?
        .export_target_accounts()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn import_target_accounts(
    app: tauri::AppHandle,
    export: TargetAccountExport,
) -> Result<(), String> {
    open_archive_store(&app)?
        .import_target_accounts(&export)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_target_account_articles(
    app: tauri::AppHandle,
    state: tauri::State<ArticleListSyncState>,
    fakeid: String,
    max_items: u32,
    page_size: u32,
) -> Result<ArticleListSyncRecord, String> {
    let archive_store = open_archive_store(&app)?;
    let client = article_list_sync::ArticleListSyncClient::new(
        state.transport.clone(),
        secret_store::production_secret_store(),
    );

    client
        .sync(&archive_store, &fakeid, max_items, page_size)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_target_articles(
    app: tauri::AppHandle,
    fakeid: String,
) -> Result<Vec<TargetArticleInput>, String> {
    open_archive_store(&app)?
        .list_target_articles(&fakeid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn latest_article_list_sync(
    app: tauri::AppHandle,
    fakeid: String,
) -> Result<Option<ArticleListSyncRecord>, String> {
    open_archive_store(&app)?
        .latest_article_list_sync(&fakeid)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn download_article_html(
    app: tauri::AppHandle,
    state: tauri::State<ArticleHtmlDownloadState>,
    request: ArticleHtmlDownloadRequest,
) -> Result<ArticleHtmlDownloadOutcome, String> {
    let archive_store = open_archive_store(&app)?;
    let client = article_html_download::ArticleHtmlDownloadClient::new(
        state.transport.clone(),
        secret_store::production_secret_store(),
    );

    client
        .download_article(
            &archive_store,
            &request.fakeid,
            &request.article_id,
            request.proxy,
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn export_article_archive(
    app: tauri::AppHandle,
    request: ArticleExportRequest,
) -> Result<ArticleExportOutcome, String> {
    ArticleExportService::new()
        .export_article(&open_archive_store(&app)?, request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preview_article_archive(
    app: tauri::AppHandle,
    request: ArticleArchivePreviewRequest,
) -> Result<ArticleArchivePreview, String> {
    ArticleArchivePreviewService::new()
        .preview_article(&open_archive_store(&app)?, request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn list_collection_tasks(app: tauri::AppHandle) -> Result<Vec<CollectionTask>, String> {
    open_archive_store(&app)?
        .list_collection_tasks()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn retry_failed_collection_task_items(
    app: tauri::AppHandle,
    task_id: String,
) -> Result<CollectionTask, String> {
    open_archive_store(&app)?
        .retry_failed_collection_task_items(&task_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn pause_collection_task(app: tauri::AppHandle, task_id: String) -> Result<CollectionTask, String> {
    open_archive_store(&app)?
        .set_collection_task_status(&task_id, CollectionTaskStatus::Paused, None)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn cancel_collection_task(
    app: tauri::AppHandle,
    task_id: String,
) -> Result<CollectionTask, String> {
    open_archive_store(&app)?
        .set_collection_task_status(&task_id, CollectionTaskStatus::Cancelled, None)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_secret(slot: SecretSlot, value: String) -> Result<(), String> {
    secret_store::production_secret_store()
        .save(slot, &value)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn read_secret(slot: SecretSlot) -> Result<Option<String>, String> {
    secret_store::production_secret_store()
        .read(slot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_secret(slot: SecretSlot) -> Result<(), String> {
    secret_store::production_secret_store()
        .delete(slot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn clear_credentials() -> Result<(), String> {
    secret_store::production_secret_store()
        .clear_credentials()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn start_official_account_login(
    state: tauri::State<OfficialAccountLoginState>,
) -> Result<OfficialAccountLoginSession, String> {
    let client = official_account_login::OfficialAccountLoginClient::new(
        state.transport.clone(),
        state.session_store.clone(),
        secret_store::production_secret_store(),
    );

    client.start_login().map_err(|error| error.to_string())
}

#[tauri::command]
fn poll_official_account_login(
    state: tauri::State<OfficialAccountLoginState>,
    session_id: String,
) -> Result<LoginScanStatus, String> {
    let client = official_account_login::OfficialAccountLoginClient::new(
        state.transport.clone(),
        state.session_store.clone(),
        secret_store::production_secret_store(),
    );

    client
        .poll_scan_status(&session_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn finalize_official_account_login(
    state: tauri::State<OfficialAccountLoginState>,
    session_id: String,
) -> Result<OfficialAccountLoginAccount, String> {
    let client = official_account_login::OfficialAccountLoginClient::new(
        state.transport.clone(),
        state.session_store.clone(),
        secret_store::production_secret_store(),
    );

    client
        .finalize_login(&session_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn logout_official_account_login() -> Result<(), String> {
    secret_store::production_secret_store()
        .delete(SecretSlot::OfficialAccountLogin)
        .map_err(|error| error.to_string())
}

fn open_archive_store(app: &tauri::AppHandle) -> Result<ArchiveStore, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;

    ArchiveStore::open(ArchiveStoreConfig::from_app_data_dir(app_data_dir))
        .map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ArticleHtmlDownloadState::default())
        .manage(ArticleListSyncState::default())
        .manage(OfficialAccountLoginState::default())
        .manage(TargetAccountSearchState::default())
        .invoke_handler(tauri::generate_handler![
            initialize_archive_store,
            search_target_accounts,
            add_target_account,
            list_target_accounts,
            delete_target_account,
            export_target_accounts,
            import_target_accounts,
            sync_target_account_articles,
            list_target_articles,
            latest_article_list_sync,
            download_article_html,
            preview_article_archive,
            export_article_archive,
            list_collection_tasks,
            retry_failed_collection_task_items,
            pause_collection_task,
            cancel_collection_task,
            save_secret,
            read_secret,
            delete_secret,
            clear_credentials,
            start_official_account_login,
            poll_official_account_login,
            finalize_official_account_login,
            logout_official_account_login
        ])
        .run(tauri::generate_context!())
        .expect("error while running WeChat Article Exporter desktop app");
}
