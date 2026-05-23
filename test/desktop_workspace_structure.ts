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

assert.equal(exists('CONTEXT.md'), true, 'CONTEXT.md must remain available from the repository root');
assert.equal(exists('docs/adr/0015-create-desktop-workspace-and-freeze-legacy-web.md'), true);
assert.equal(exists('apps/desktop/README.md'), true, 'desktop workspace README must exist');
assert.equal(exists('apps/desktop/package.json'), true, 'desktop workspace package manifest must exist');
assert.equal(exists('legacy-web/README.md'), true, 'legacy web reference README must exist');

const rootReadme = read('README.md');
assert.match(rootReadme, /active product is the desktop app/i);
assert.match(rootReadme, /apps\/desktop/i);
assert.match(rootReadme, /legacy web app/i);

const rootPackage = JSON.parse(read('package.json'));
assert.equal(rootPackage.scripts.dev, 'yarn desktop:check');
assert.equal(rootPackage.scripts['desktop:check'], 'yarn --cwd apps/desktop check:workspace');
assert.equal(rootPackage.scripts['legacy:dev'], 'nuxt dev');
assert.equal(rootPackage.scripts['legacy:build'], 'nuxt build');
assert.equal(rootPackage.scripts.build, undefined);

const desktopReadme = read('apps/desktop/README.md');
assert.match(desktopReadme, /Desktop Workspace/);
assert.match(desktopReadme, /Tauri 2/);
assert.match(desktopReadme, /Vite/);
assert.match(desktopReadme, /Vue 3/);

const legacyReadme = read('legacy-web/README.md');
assert.match(legacyReadme, /Legacy Web App/);
assert.match(legacyReadme, /migration reference/i);

const gitignore = read('.gitignore');
assert.match(gitignore, /!docs\/adr\//);
assert.match(gitignore, /!docs\/adr\/\*\*/);
