import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const lib = read('apps/desktop/src-tauri/src/lib.rs');
const articlesView = read('apps/desktop/src/views/ArticlesView.vue');
const singleArticleView = read('apps/desktop/src/views/SingleArticleView.vue');
const albumsView = read('apps/desktop/src/views/AlbumsView.vue');
const exportDialog = read('apps/desktop/src/exportDialog.ts');
const desktopCapability = read('apps/desktop/src-tauri/capabilities/default.json');
const desktopPackage = JSON.parse(read('apps/desktop/package.json'));
const cargoToml = read('apps/desktop/src-tauri/Cargo.toml');
const tauriLib = read('apps/desktop/src-tauri/src/lib.rs');

assert.match(articlesView, /export_article_archive/);
assert.match(lib, /export_article_archive/);
assert.match(articlesView, /exportArticleArchive/);
assert.match(articlesView, /导出 Markdown/);
assert.match(articlesView, /导出 HTML/);
assert.match(articlesView, /ArticleExportOutcome/);
assert.match(articlesView, /markdownFile/);
assert.match(articlesView, /htmlFile/);
assert.doesNotMatch(
  articlesView,
  /refreshCollectionTasks|文章工作流任务|任务进度/,
  'article export UI should keep the legacy article table as the primary workflow, without a persistent task panel',
);

assert.equal(desktopPackage.dependencies['@tauri-apps/plugin-dialog'], '2.3.0');
assert.match(cargoToml, /tauri-plugin-dialog = "=2\.3\.0"/);
assert.match(tauriLib, /\.plugin\(tauri_plugin_dialog::init\(\)\)/);
assert.match(desktopCapability, /"dialog:default"/);
assert.match(desktopCapability, /"core:default"/);

for (const source of [exportDialog, articlesView, singleArticleView, albumsView]) {
  assert.match(source, /@tauri-apps\/plugin-dialog|chooseArticleExportFile|chooseArticleExportDirectory/);
}

for (const source of [articlesView, singleArticleView]) {
  assert.match(source, /outputFile/);
  assert.match(source, /savedMarkdownFile/);
  assert.match(source, /savedHtmlFile/);
}

for (const source of [articlesView, albumsView]) {
  assert.match(source, /outputDir/);
}
