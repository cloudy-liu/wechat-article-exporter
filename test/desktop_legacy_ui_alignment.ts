import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const router = read('apps/desktop/src/router.ts');
const app = read('apps/desktop/src/App.vue');
const styles = read('apps/desktop/src/styles.css');
const targetAccountsView = read('apps/desktop/src/views/TargetAccountsView.vue');
const targetAccountManager = read('apps/desktop/src/components/TargetAccountManager.vue');
const articlesView = read('apps/desktop/src/views/ArticlesView.vue');
const singleArticleView = read('apps/desktop/src/views/SingleArticleView.vue');
const albumsView = read('apps/desktop/src/views/AlbumsView.vue');

for (const legacyNavLabel of ['公众号管理', '文章下载', '单篇文章下载', '合集下载', '设置']) {
  assert.match(router, new RegExp(legacyNavLabel), `router should keep the legacy navigation label: ${legacyNavLabel}`);
}

assert.doesNotMatch(router, /文章库/, 'desktop navigation should not rename the legacy article workflow');
assert.match(app, /legacy-dashboard-shell/);
assert.match(app, /legacy-sidebar/);
assert.match(app, /legacy-topbar/);

for (const legacyShellMarker of [
  '.legacy-dashboard-shell',
  '.legacy-sidebar',
  '.legacy-topbar',
  '.desktop-data-page',
  '.desktop-page-toolbar',
  '.desktop-table-shell',
  '.desktop-grid',
  '.desktop-bulk-toolbar',
]) {
  assert.match(styles, new RegExp(legacyShellMarker.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
}

assert.match(targetAccountsView, /desktop-data-page/);
assert.match(targetAccountManager, /desktop-page-toolbar/);
assert.match(targetAccountManager, /desktop-table-shell/);
assert.match(targetAccountManager, /desktop-grid/);

for (const source of [articlesView, singleArticleView, albumsView]) {
  assert.match(source, /desktop-data-page/);
  assert.match(source, /desktop-page-toolbar/);
  assert.match(source, /desktop-table-shell/);
}

assert.match(articlesView, /list_target_accounts/);
assert.match(articlesView, /<select/);
assert.match(articlesView, /抓取/);
assert.match(articlesView, /导出 Markdown/);
assert.match(singleArticleView, /请输入公众号文章链接/);
assert.match(singleArticleView, /<div class="workflow-table/);
assert.match(albumsView, /album-preview-shell/);
assert.match(albumsView, /批量下载/);
