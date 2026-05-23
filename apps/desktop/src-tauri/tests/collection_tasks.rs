use tempfile::tempdir;
use wechat_article_exporter_desktop_lib::archive_store::{
    ArchiveStore, ArchiveStoreConfig, CollectionTaskItemInput, CollectionTaskItemStatus,
    CollectionTaskStatus, CollectionTaskType,
};

#[test]
fn creates_persistent_task_with_item_level_waiting_state() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");

    let task = store
        .create_collection_task(
            CollectionTaskType::AccountArticleSync,
            Some("fakeid-1"),
            vec![
                CollectionTaskItemInput {
                    item_id: "page-0".to_string(),
                    item_type: "article-list-page".to_string(),
                    payload_json: r#"{"begin":0,"count":5}"#.to_string(),
                },
                CollectionTaskItemInput {
                    item_id: "page-5".to_string(),
                    item_type: "article-list-page".to_string(),
                    payload_json: r#"{"begin":5,"count":5}"#.to_string(),
                },
            ],
        )
        .expect("create collection task");

    assert_eq!(task.task_type, CollectionTaskType::AccountArticleSync);
    assert_eq!(task.status, CollectionTaskStatus::Waiting);
    assert_eq!(task.total_items, 2);
    assert_eq!(task.waiting_items, 2);
    assert_eq!(task.running_items, 0);
    assert_eq!(task.succeeded_items, 0);
    assert_eq!(task.failed_items, 0);
    assert_eq!(task.cancelled_items, 0);

    let loaded = store
        .get_collection_task(&task.task_id)
        .expect("load collection task")
        .expect("task exists");

    assert_eq!(loaded.items.len(), 2);
    assert_eq!(loaded.items[0].status, CollectionTaskItemStatus::Waiting);
    assert_eq!(loaded.items[0].attempts, 0);
    assert_eq!(loaded.items[0].payload_json, r#"{"begin":0,"count":5}"#);
}

#[test]
fn updates_item_state_and_rolls_up_task_outcome() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let task = create_two_item_task(&store);

    store
        .update_collection_task_item_status(
            &task.task_id,
            "page-0",
            CollectionTaskItemStatus::Running,
            None,
        )
        .expect("mark first item running");

    let running = store
        .get_collection_task(&task.task_id)
        .expect("load running task")
        .expect("task exists");
    assert_eq!(running.status, CollectionTaskStatus::Running);
    assert_eq!(running.running_items, 1);
    assert_eq!(running.waiting_items, 1);

    store
        .update_collection_task_item_status(
            &task.task_id,
            "page-0",
            CollectionTaskItemStatus::Succeeded,
            None,
        )
        .expect("mark first item succeeded");
    store
        .update_collection_task_item_status(
            &task.task_id,
            "page-5",
            CollectionTaskItemStatus::Failed,
            Some("upstream timeout".to_string()),
        )
        .expect("mark second item failed");

    let failed = store
        .get_collection_task(&task.task_id)
        .expect("load failed task")
        .expect("task exists");
    assert_eq!(failed.status, CollectionTaskStatus::Failed);
    assert_eq!(failed.succeeded_items, 1);
    assert_eq!(failed.failed_items, 1);
    assert_eq!(failed.error_message.as_deref(), Some("upstream timeout"));
}

#[test]
fn task_and_item_state_survive_store_reopen() {
    let temp = tempdir().expect("temp dir");
    let config = ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    };
    let store = ArchiveStore::open(config.clone()).expect("open archive store");
    let task = create_two_item_task(&store);
    store
        .update_collection_task_item_status(
            &task.task_id,
            "page-0",
            CollectionTaskItemStatus::Succeeded,
            None,
        )
        .expect("mark first item succeeded");
    drop(store);

    let reopened = ArchiveStore::open(config).expect("reopen archive store");
    let loaded = reopened
        .get_collection_task(&task.task_id)
        .expect("load reopened task")
        .expect("task exists after reopen");

    assert_eq!(loaded.status, CollectionTaskStatus::Running);
    assert_eq!(loaded.succeeded_items, 1);
    assert_eq!(loaded.waiting_items, 1);
    assert_eq!(loaded.items[0].status, CollectionTaskItemStatus::Succeeded);
    assert_eq!(loaded.items[1].status, CollectionTaskItemStatus::Waiting);
}

#[test]
fn retry_failed_items_preserves_successful_items() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let task = create_two_item_task(&store);
    store
        .update_collection_task_item_status(
            &task.task_id,
            "page-0",
            CollectionTaskItemStatus::Succeeded,
            None,
        )
        .expect("mark first item succeeded");
    store
        .update_collection_task_item_status(
            &task.task_id,
            "page-5",
            CollectionTaskItemStatus::Failed,
            Some("temporary failure".to_string()),
        )
        .expect("mark second item failed");

    let retry = store
        .retry_failed_collection_task_items(&task.task_id)
        .expect("retry failed task items");

    assert_eq!(retry.status, CollectionTaskStatus::Running);
    assert_eq!(retry.succeeded_items, 1);
    assert_eq!(retry.failed_items, 0);
    assert_eq!(retry.waiting_items, 1);
    assert_eq!(retry.items[0].status, CollectionTaskItemStatus::Succeeded);
    assert_eq!(retry.items[0].attempts, 1);
    assert_eq!(retry.items[1].status, CollectionTaskItemStatus::Waiting);
    assert_eq!(retry.items[1].attempts, 1);
    assert_eq!(retry.items[1].error_message, None);
}

#[test]
fn paused_and_cancelled_task_states_are_durable() {
    let temp = tempdir().expect("temp dir");
    let store = ArchiveStore::open(ArchiveStoreConfig {
        database_path: temp.path().join("archive.sqlite"),
        archive_dir: temp.path().join("archive"),
    })
    .expect("open archive store");
    let task = create_two_item_task(&store);

    let paused = store
        .set_collection_task_status(&task.task_id, CollectionTaskStatus::Paused, None)
        .expect("pause task");
    assert_eq!(paused.status, CollectionTaskStatus::Paused);

    let cancelled = store
        .set_collection_task_status(&task.task_id, CollectionTaskStatus::Cancelled, None)
        .expect("cancel task");
    assert_eq!(cancelled.status, CollectionTaskStatus::Cancelled);
    assert_eq!(cancelled.cancelled_items, 2);
    assert_eq!(
        cancelled.items[0].status,
        CollectionTaskItemStatus::Cancelled
    );
}

fn create_two_item_task(
    store: &ArchiveStore,
) -> wechat_article_exporter_desktop_lib::archive_store::CollectionTask {
    store
        .create_collection_task(
            CollectionTaskType::AccountArticleSync,
            Some("fakeid-1"),
            vec![
                CollectionTaskItemInput {
                    item_id: "page-0".to_string(),
                    item_type: "article-list-page".to_string(),
                    payload_json: r#"{"begin":0,"count":5}"#.to_string(),
                },
                CollectionTaskItemInput {
                    item_id: "page-5".to_string(),
                    item_type: "article-list-page".to_string(),
                    payload_json: r#"{"begin":5,"count":5}"#.to_string(),
                },
            ],
        )
        .expect("create collection task")
}
