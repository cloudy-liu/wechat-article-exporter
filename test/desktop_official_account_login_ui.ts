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
assert.match(loginPanel, /Official Account platform login only/);
assert.match(loginPanel, /Personal WeChat login is not supported/);
assert.match(loginPanel, /qrCodeDataUrl/);
assert.match(loginPanel, /sessionId/);

const targetAccountsView = read('apps/desktop/src/views/TargetAccountsView.vue');
assert.match(targetAccountsView, /OfficialAccountLoginPanel/);

const router = read('apps/desktop/src/router.ts');
assert.match(router, /TargetAccountsView/);
