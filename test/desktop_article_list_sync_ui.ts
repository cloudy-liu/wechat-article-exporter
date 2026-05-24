import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const manager = read('apps/desktop/src/components/TargetAccountManager.vue');
const articlesView = read('apps/desktop/src/views/ArticlesView.vue');
const lib = read('apps/desktop/src-tauri/src/lib.rs');

for (const command of [
  'load_desktop_settings',
  'sync_target_account_articles',
]) {
  assert.match(manager, new RegExp(command));
  assert.match(lib, new RegExp(command));
}

for (const command of [
  'list_target_articles',
  'latest_article_list_sync',
]) {
  assert.match(lib, new RegExp(command));
}

assert.match(manager, /maxItems/);
assert.match(manager, /pageSize/);
assert.match(manager, /syncDownload/);
assert.match(manager, /syncArticles/);
assert.match(manager, /syncStatus/);
assert.match(manager, /同步/);

assert.match(articlesView, /list_target_articles/);
assert.match(articlesView, /refreshArticles/);
assert.match(articlesView, /已同步文章/);
assert.match(articlesView, /legacy-article-table-shell/);
assert.doesNotMatch(
  manager,
  /articleList|最近同步|已同步文章/,
  'account management should stay focused on target accounts; synced article browsing belongs on the article download page',
);
