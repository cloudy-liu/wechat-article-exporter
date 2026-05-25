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

for (const command of [
  'load_article_reading_credential_status',
  'save_article_reading_credential',
  'mark_article_reading_credential_expired',
  'delete_article_reading_credential',
]) {
  assert.match(settingsView, new RegExp(command));
  assert.match(lib, new RegExp(command));
}

assert.match(lib, /enrich_selected_articles_with_reading_credential/);
assert.doesNotMatch(articlesView, /load_article_reading_credential_status/);
assert.doesNotMatch(articlesView, /enrich_selected_articles_with_reading_credential/);
assert.doesNotMatch(articlesView, /reading-enrichment-panel/);
assert.doesNotMatch(articlesView, /富集选中文章/);

for (const marker of [
  '阅读凭证',
  '不是公众号平台登录',
  '保存阅读凭证',
  '标记过期',
  '删除阅读凭证',
  '没有阅读凭证也可以下载 HTML 和导出 Markdown/HTML',
]) {
  assert.match(settingsView, new RegExp(marker));
}

assert.match(settingsView, /公众号平台登录/);
assert.match(settingsView, /阅读凭证/);
assert.notEqual(settingsView.indexOf('公众号平台登录'), settingsView.indexOf('阅读凭证'));
assert.doesNotMatch(articlesView, /Public Proxy|Cloudflare|Worker|AG Grid Enterprise/i);
assert.doesNotMatch(settingsView, /Public Proxy|Cloudflare|Worker|AG Grid Enterprise/i);
