import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const router = read('apps/desktop/src/router.ts');
const articlesView = read('apps/desktop/src/views/ArticlesView.vue');
const lib = read('apps/desktop/src-tauri/src/lib.rs');

assert.match(router, /ArticlesView/);
const articlesRoute = router.match(/path: '\/articles'[\s\S]*?meta:/)?.[0] || '';
assert.match(articlesRoute, /component: ArticlesView/);
assert.doesNotMatch(articlesRoute, /component: PlaceholderView/);

for (const command of [
  'list_target_accounts',
  'list_target_articles',
  'download_article_html',
  'preview_article_archive',
  'export_article_archive',
]) {
  assert.match(articlesView, new RegExp(command));
  assert.match(lib, new RegExp(command));
}

for (const marker of [
  'article-search-input',
  'selectedArticleIds',
  'filteredArticles',
  'toggleArticleSelection',
  '下载选中文章',
  '预览',
  '导出选中 Markdown',
  '导出选中 HTML',
  'legacy-article-table-shell',
  'article-preview-dialog',
  '归档预览',
]) {
  assert.match(articlesView, new RegExp(marker));
}

assert.doesNotMatch(articlesView, /ag-grid|AgGrid|agGrid|AG Grid|server\/api|['"`]\/api\//);
assert.doesNotMatch(
  articlesView,
  /<section class="article-preview-panel"/,
  'article download should not keep a desktop-only persistent preview panel in the legacy table page',
);
assert.doesNotMatch(
  articlesView,
  /<section class="article-sync-panel"/,
  'article download should not keep a desktop-only persistent task panel in the legacy table page',
);
assert.doesNotMatch(
  articlesView,
  /list_collection_tasks|refreshCollectionTasks|文章工作流任务|任务进度/,
  'article download should not depend on collection task panels in the primary legacy table workflow',
);
