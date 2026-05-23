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
  '保存文章',
  '下载 HTML',
  '预览',
  '导出 Markdown',
  '导出 HTML',
  '归档预览',
  '任务进度',
]) {
  assert.match(singleArticleView, new RegExp(marker));
}

assert.doesNotMatch(singleArticleView, /ag-grid|AgGrid|agGrid|AG Grid|server\/api|['"`]\/api\//);
