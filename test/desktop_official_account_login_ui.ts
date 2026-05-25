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

assert.equal(exists('apps/desktop/src/components/OfficialAccountLoginPanel.vue'), true);
assert.equal(exists('apps/desktop/src/views/TargetAccountsView.vue'), true);

const loginPanel = read('apps/desktop/src/components/OfficialAccountLoginPanel.vue');
for (const command of [
  'start_official_account_login',
  'poll_official_account_login',
  'finalize_official_account_login',
  'logout_official_account_login',
]) {
  assert.match(loginPanel, new RegExp(command));
}
assert.match(loginPanel, /@tauri-apps\/api\/core/);
assert.match(loginPanel, /仅支持公众号平台登录/);
assert.match(loginPanel, /不支持个人微信号登录/);
assert.match(loginPanel, /qrCodeDataUrl/);
assert.match(loginPanel, /sessionId/);

const app = read('apps/desktop/src/App.vue');
const targetAccountsView = read('apps/desktop/src/views/TargetAccountsView.vue');
assert.match(app, /OfficialAccountLoginPanel/);
assert.match(app, /legacy-sidebar/);
assert.doesNotMatch(
  targetAccountsView,
  /OfficialAccountLoginPanel/,
  'official account login should live in the legacy sidebar footer, not inside the account management page body',
);

const router = read('apps/desktop/src/router.ts');
assert.match(router, /TargetAccountsView/);
