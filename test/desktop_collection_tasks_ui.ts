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
  assert.match(manager, new RegExp(command));
  assert.match(lib, new RegExp(command));
}

assert.match(manager, /Collection tasks/);
assert.match(manager, /Retry failed/);
assert.match(manager, /accountArticleSync/);
assert.match(manager, /succeeded_items/);
assert.match(manager, /failed_items/);
assert.match(manager, /waiting_items/);
assert.match(manager, /running_items/);
