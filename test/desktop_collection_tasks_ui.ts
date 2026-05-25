import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const manager = read('apps/desktop/src/components/TargetAccountManager.vue');
const lib = read('apps/desktop/src-tauri/src/lib.rs');

for (const command of [
  'list_collection_tasks',
  'retry_failed_collection_task_items',
  'pause_collection_task',
  'cancel_collection_task',
]) {
  assert.match(lib, new RegExp(command));
}

for (const marker of [
  'list_collection_tasks',
  'retry_failed_collection_task_items',
  'pause_collection_task',
  'cancel_collection_task',
  '采集任务',
  '重试失败项',
  'accountArticleSync',
  'succeeded_items',
  'failed_items',
  'waiting_items',
  'running_items',
]) {
  assert.doesNotMatch(
    manager,
    new RegExp(marker),
    'account management should keep the legacy primary table layout without embedding collection task panels',
  );
}
