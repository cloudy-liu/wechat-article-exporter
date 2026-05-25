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
const styles = read('apps/desktop/src/styles.css');
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

assert.doesNotMatch(settingsView, /桌面端设置/);
assert.doesNotMatch(settingsView, /本地采集配置/);
assert.match(settingsView, /class="settings-workbench desktop-data-page"/);
assert.doesNotMatch(
  settingsView,
  /class="manager-toolbar"/,
  'settings page should not render a migration-era page title toolbar below the legacy topbar',
);
assert.match(
  styles,
  /\.settings-workbench[\s\S]*padding:\s*40px 16px/,
  'settings content should start like the legacy settings page instead of with a compact desktop toolbar',
);
assert.match(
  styles,
  /\.settings-panel[\s\S]*background:\s*#ffffff/,
  'settings cards should use the legacy white card surface rather than the warm migration card surface',
);
assert.match(
  styles,
  /\.settings-panel[\s\S]*border-color:\s*#e2e8f0/,
  'settings cards should use the same slate border as the legacy dashboard cards',
);

for (const excluded of ['Public Proxy', 'Public API', 'Cloudflare', 'Sponsorship', 'Developer']) {
  assert.doesNotMatch(settingsView, new RegExp(excluded, 'i'));
}
