import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const manager = read('apps/desktop/src/components/TargetAccountManager.vue');
const lib = read('apps/desktop/src-tauri/src/lib.rs');

assert.match(manager, /export_article_archive/);
assert.match(lib, /export_article_archive/);
assert.match(manager, /exportArticleArchive/);
assert.match(manager, /导出 Markdown/);
assert.match(manager, /导出 HTML/);
assert.match(manager, /ArticleExportOutcome/);
assert.match(manager, /markdownFile/);
assert.match(manager, /htmlFile/);
assert.match(manager, /refreshCollectionTasks/);
