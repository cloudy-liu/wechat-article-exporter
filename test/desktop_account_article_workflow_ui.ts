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
  'list_collection_tasks',
]) {
  assert.match(articlesView, new RegExp(command));
  assert.match(lib, new RegExp(command));
}

for (const marker of [
  'article-search-input',
  'selectedArticleIds',
  'filteredArticles',
  'toggleArticleSelection',
  'Download selected',
  'Preview',
  'Export selected Markdown',
  'Export selected HTML',
  'Archive preview',
  'Task progress',
]) {
  assert.match(articlesView, new RegExp(marker));
}

assert.doesNotMatch(articlesView, /ag-grid|AgGrid|agGrid|AG Grid|server\/api|['"`]\/api\//);

const refreshCollectionTasksBody = articlesView.match(/async function refreshCollectionTasks\(\) \{[\s\S]*?\n\}/)?.[0] || '';
assert.match(refreshCollectionTasksBody, /try \{/);
assert.match(refreshCollectionTasksBody, /catch \(error\)/);
assert.match(refreshCollectionTasksBody, /errorMessage\.value = formatError\(error\)/);
