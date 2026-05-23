import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const manager = read('apps/desktop/src/components/TargetAccountManager.vue');
const lib = read('apps/desktop/src-tauri/src/lib.rs');

assert.match(manager, /download_article_html/);
assert.match(lib, /download_article_html/);
assert.match(manager, /downloadArticleHtml/);
assert.match(manager, /Download HTML/);
assert.match(manager, /Network proxy/);
assert.match(manager, /proxyAuthorization/);
assert.match(manager, /articleHtmlDownload/);
assert.match(manager, /refreshCollectionTasks/);
