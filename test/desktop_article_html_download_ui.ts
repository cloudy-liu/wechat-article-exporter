import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const articlesView = read('apps/desktop/src/views/ArticlesView.vue');
const settingsView = read('apps/desktop/src/views/SettingsView.vue');
const lib = read('apps/desktop/src-tauri/src/lib.rs');

assert.match(articlesView, /download_article_html/);
assert.match(lib, /download_article_html/);
assert.match(articlesView, /downloadArticle/);
assert.match(articlesView, /下载选中文章/);
assert.match(articlesView, /抓取/);
assert.match(articlesView, /proxy:\s*null/);
assert.doesNotMatch(
  articlesView,
  /refreshCollectionTasks|文章工作流任务|任务进度/,
  'article HTML download UI should stay in the legacy article table workflow without a persistent task panel',
);

assert.match(settingsView, /网络代理/);
assert.match(settingsView, /proxyAuthorization/);
