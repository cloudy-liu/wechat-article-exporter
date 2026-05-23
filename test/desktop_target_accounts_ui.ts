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

assert.equal(exists('apps/desktop/src/components/TargetAccountManager.vue'), true);

const manager = read('apps/desktop/src/components/TargetAccountManager.vue');
for (const command of [
  'search_target_accounts',
  'add_target_account',
  'list_target_accounts',
  'delete_target_account',
  'export_target_accounts',
  'import_target_accounts',
]) {
  assert.match(manager, new RegExp(command));
}
assert.match(manager, /@tauri-apps\/api\/core/);
assert.match(manager, /Target Official Accounts/);
assert.match(manager, /Search/);
assert.match(manager, /Import/);
assert.match(manager, /Export/);
assert.match(manager, /Delete/);
assert.doesNotMatch(manager, /Public API/i);
assert.doesNotMatch(manager, /web deployment/i);

const targetAccountsView = read('apps/desktop/src/views/TargetAccountsView.vue');
assert.match(targetAccountsView, /TargetAccountManager/);
