import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

const router = read('apps/desktop/src/router.ts');
const lib = read('apps/desktop/src-tauri/src/lib.rs');
const albumsViewPath = path.join(root, 'apps/desktop/src/views/AlbumsView.vue');

assert.ok(fs.existsSync(albumsViewPath), 'AlbumsView.vue should exist for the desktop album workflow');

const albumsView = fs.readFileSync(albumsViewPath, 'utf8');

assert.match(router, /AlbumsView/);
const albumsRoute = router.match(/path: '\/albums'[\s\S]*?meta:/)?.[0] || '';
assert.match(albumsRoute, /component: AlbumsView/);
assert.doesNotMatch(albumsRoute, /component: PlaceholderView/);

for (const command of [
  'load_desktop_settings',
  'list_target_accounts',
  'list_target_account_albums',
  'fetch_album_page',
  'fetch_all_album_articles',
  'download_album_articles',
  'export_album_articles',
  'list_collection_tasks',
]) {
  assert.match(albumsView, new RegExp(command));
  assert.match(lib, new RegExp(command));
}

for (const marker of [
  'album-account-selector',
  'selectedAlbumId',
  'albumArticles',
  '抓取全部文章链接',
  '下载合集 HTML',
  '导出 Markdown',
  '导出 HTML',
  '任务进度',
]) {
  assert.match(albumsView, new RegExp(marker));
}

assert.doesNotMatch(albumsView, /ag-grid|AgGrid|agGrid|AG Grid|server\/api|['"`]\/api\//);

const refreshCollectionTasksBody = albumsView.match(/async function refreshCollectionTasks\(\) \{[\s\S]*?\n\}/)?.[0] || '';
assert.match(refreshCollectionTasksBody, /try \{/);
assert.match(refreshCollectionTasksBody, /catch \(error\)/);
assert.match(refreshCollectionTasksBody, /errorMessage\.value = formatError\(error\)/);
