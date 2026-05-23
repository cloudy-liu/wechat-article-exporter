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

assert.equal(
  exists('apps/desktop/scripts/verify-desktop-blockers.mjs'),
  true,
  'desktop workspace must include an executable blocker verification script',
);

const desktopPackage = JSON.parse(read('apps/desktop/package.json'));
assert.equal(
  desktopPackage.scripts['check:workspace'],
  'node scripts/verify-desktop-blockers.mjs',
  'desktop check must verify blocker isolation instead of only printing readiness',
);

const verifier = read('apps/desktop/scripts/verify-desktop-blockers.mjs');
for (const requiredCheck of [
  'ag-grid-enterprise',
  'ag-grid-vue3',
  '@ag-grid-community',
  'nuxt',
  'nitro',
  '@sentry',
  'umami',
  'cloudflare',
  'wrangler',
  'web analytics',
  'Public API',
  'Public Proxy',
  'Sponsorship',
  'support pages',
  'Developer',
  'developer/debug',
]) {
  assert.match(verifier, new RegExp(requiredCheck.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i'));
}
