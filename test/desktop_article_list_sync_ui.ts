import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const manager = read('apps/desktop/src/components/TargetAccountManager.vue');

for (const command of [
  'load_desktop_settings',
  'sync_target_account_articles',
  'list_target_articles',
  'latest_article_list_sync',
]) {
  assert.match(manager, new RegExp(command));
}

assert.match(manager, /maxItems/);
assert.match(manager, /pageSize/);
assert.match(manager, /syncDownload/);
assert.match(manager, /syncArticles/);
assert.match(manager, /articleList/);
assert.match(manager, /syncStatus/);
assert.match(manager, /同步文章/);
assert.match(manager, /最近同步/);
assert.match(manager, /已同步文章/);
assert.match(manager, /重试/);
