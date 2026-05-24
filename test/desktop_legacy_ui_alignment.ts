import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

function readRule(source: string, selector: string): string {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const matches = [...source.matchAll(new RegExp(`${escapedSelector}\\s*\\{([^}]*)\\}`, 'gm'))];
  assert.ok(matches.length > 0, `styles should include ${selector}`);
  return matches.map(match => match[1]).join('\n');
}

const router = read('apps/desktop/src/router.ts');
const app = read('apps/desktop/src/App.vue');
const styles = read('apps/desktop/src/styles.css');
const targetAccountsView = read('apps/desktop/src/views/TargetAccountsView.vue');
const targetAccountManager = read('apps/desktop/src/components/TargetAccountManager.vue');
const officialAccountLoginPanel = read('apps/desktop/src/components/OfficialAccountLoginPanel.vue');
const articlesView = read('apps/desktop/src/views/ArticlesView.vue');
const singleArticleView = read('apps/desktop/src/views/SingleArticleView.vue');
const albumsView = read('apps/desktop/src/views/AlbumsView.vue');
const settingsView = read('apps/desktop/src/views/SettingsView.vue');

for (const legacyNavLabel of ['公众号管理', '文章下载', '单篇文章下载', '合集下载', '设置']) {
  assert.match(router, new RegExp(legacyNavLabel), `router should keep the legacy navigation label: ${legacyNavLabel}`);
}

assert.doesNotMatch(router, /文章库/, 'desktop navigation should not rename the legacy article workflow');
assert.match(app, /legacy-dashboard-shell/);
assert.match(app, /legacy-sidebar/);
assert.match(app, /legacy-topbar/);
assert.doesNotMatch(
  readRule(styles, '.legacy-sidebar .navigation'),
  /flex:\s*1\b/,
  'legacy sidebar navigation must stay compact instead of stretching items down the full sidebar',
);

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
assert.doesNotMatch(targetAccountsView, /workflow-strip/, 'account page must not show the redesigned three-step guide');
assert.doesNotMatch(targetAccountsView, /目标公众号工作流/, 'account page must not show migration-era workflow guidance cards');

assert.match(officialAccountLoginPanel, /legacy-login-strip/);
assert.doesNotMatch(
  officialAccountLoginPanel,
  /login-panel__copy/,
  'official account login must be a compact legacy-like strip, not a large introduction card',
);

assert.match(targetAccountManager, /desktop-page-toolbar/);
assert.match(targetAccountManager, /desktop-table-shell/);
assert.match(targetAccountManager, /account-data-table/);
assert.doesNotMatch(
  targetAccountManager,
  /class="workflow-table desktop-grid account-table"/,
  'account management must use a compact account-specific table instead of the generic grid workflow table',
);
assert.doesNotMatch(
  targetAccountManager,
  /role="columnheader">标识<|role="columnheader">fakeid<|account-identifier/,
  'account management should not expose the internal fakeid as a visible legacy table column',
);
assert.match(
  targetAccountManager,
  /normalizeWechatImageUrl/,
  'account avatars should normalize WeChat image URLs before rendering',
);
assert.doesNotMatch(
  targetAccountManager,
  /<div class="article-workflow-summary">/,
  'account management should match the legacy AG Grid page by putting the table directly below the toolbar',
);
assert.doesNotMatch(
  targetAccountManager,
  /<section class="article-sync-panel"/,
  'account management should not embed article sync and task cards below the primary legacy account table',
);

for (const accountTableMarker of [
  '.account-data-table',
  '.account-data-table__head',
  '.account-data-table__row',
  '.account-description',
  '.account-row-actions',
]) {
  assert.match(styles, new RegExp(accountTableMarker.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
}

assert.match(
  readRule(styles, '.account-data-table__row'),
  /height:\s*58px/,
  'account table rows should keep a compact fixed height like the legacy AG Grid table',
);
assert.match(
  readRule(styles, '.account-description'),
  /-webkit-line-clamp:\s*2/,
  'long account descriptions should be clamped instead of stretching rows or overlapping actions',
);

assert.doesNotMatch(
  articlesView,
  /<section class="article-preview-panel"/,
  'article download should not render the desktop-only persistent preview panel in the primary legacy table view',
);
assert.doesNotMatch(
  articlesView,
  /<section class="article-sync-panel"/,
  'article download should not render the desktop-only persistent task panel in the primary legacy table view',
);
assert.match(
  articlesView,
  /legacy-article-table-shell/,
  'article download should use a full-height legacy table shell under the toolbar',
);

for (const source of [articlesView, singleArticleView, albumsView]) {
  assert.match(source, /desktop-data-page/);
  assert.match(source, /desktop-page-toolbar/);
  assert.match(source, /desktop-table-shell/);
}

for (const source of [targetAccountManager, articlesView, singleArticleView, albumsView, settingsView]) {
  assert.doesNotMatch(source, /workflow-strip/);
  assert.doesNotMatch(source, /reading-enrichment-panel/);
}

assert.match(articlesView, /list_target_accounts/);
assert.match(articlesView, /<select/);
assert.match(articlesView, /抓取/);
assert.match(articlesView, /导出 Markdown/);
assert.match(singleArticleView, /请输入公众号文章链接/);
assert.match(singleArticleView, /<div class="workflow-table/);
assert.match(albumsView, /album-preview-shell/);
assert.match(albumsView, /批量下载/);
