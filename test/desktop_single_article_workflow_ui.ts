import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const router = read('apps/desktop/src/router.ts');
const singleArticleView = read('apps/desktop/src/views/SingleArticleView.vue');
const lib = read('apps/desktop/src-tauri/src/lib.rs');

assert.match(router, /SingleArticleView/);
const singleArticleRoute = router.match(/path: '\/single-article'[\s\S]*?meta:/)?.[0] || '';
assert.match(singleArticleRoute, /component: SingleArticleView/);
assert.doesNotMatch(singleArticleRoute, /component: PlaceholderView/);

for (const command of [
  'save_single_article',
  'list_single_articles',
  'download_single_article_html',
  'preview_article_archive',
  'export_article_archive',
  'list_collection_tasks',
]) {
  assert.match(singleArticleView, new RegExp(command));
  assert.match(lib, new RegExp(command));
}

for (const marker of [
  'single-article-url-input',
  'singleArticleUrl',
  'validateSingleArticleUrl',
  'singleArticles',
  'Save article',
  'Download HTML',
  'Preview',
  'Export Markdown',
  'Export HTML',
  'Archive preview',
  'Task progress',
]) {
  assert.match(singleArticleView, new RegExp(marker));
}

assert.doesNotMatch(singleArticleView, /ag-grid|AgGrid|agGrid|AG Grid|server\/api|['"`]\/api\//);
