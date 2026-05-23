import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath: string): string {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

function exists(relativePath: string): boolean {
  return fs.existsSync(path.join(root, relativePath));
}

assert.equal(exists('apps/desktop/src/views/SettingsView.vue'), true);

const router = read('apps/desktop/src/router.ts');
const settingsView = read('apps/desktop/src/views/SettingsView.vue');
const lib = read('apps/desktop/src-tauri/src/lib.rs');

assert.match(router, /SettingsView/);
const settingsRoute = router.match(/path: '\/settings'[\s\S]*?meta:/)?.[0] || '';
assert.match(settingsRoute, /component: SettingsView/);
assert.doesNotMatch(settingsRoute, /component: PlaceholderView/);

for (const command of [
  'load_desktop_settings',
  'save_desktop_settings',
  'clear_credentials',
  'logout_official_account_login',
]) {
  assert.match(settingsView, new RegExp(command));
  assert.match(lib, new RegExp(command));
}

for (const marker of [
  '本地归档',
  '导出偏好',
  '同步与下载',
  '网络代理',
  '清理凭证',
  '保存设置',
  '清除全部本地凭证',
  '退出公众号登录',
]) {
  assert.match(settingsView, new RegExp(marker));
}

for (const excluded of ['Public Proxy', 'Public API', 'Cloudflare', 'Sponsorship', 'Developer']) {
  assert.doesNotMatch(settingsView, new RegExp(excluded, 'i'));
}
