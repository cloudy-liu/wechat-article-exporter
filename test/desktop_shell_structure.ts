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

assert.equal(exists('apps/desktop/index.html'), true);
assert.equal(exists('apps/desktop/vite.config.ts'), true);
assert.equal(exists('apps/desktop/tsconfig.json'), true);
assert.equal(exists('apps/desktop/src/main.ts'), true);
assert.equal(exists('apps/desktop/src/App.vue'), true);
assert.equal(exists('apps/desktop/src/router.ts'), true);
assert.equal(exists('apps/desktop/src-tauri/Cargo.toml'), true);
assert.equal(exists('apps/desktop/src-tauri/tauri.conf.json'), true);
assert.equal(exists('apps/desktop/src-tauri/src/main.rs'), true);
assert.equal(exists('apps/desktop/src-tauri/icons/icon.ico'), true);

const desktopPackage = JSON.parse(read('apps/desktop/package.json'));
assert.equal(desktopPackage.scripts.dev, 'vite --host 127.0.0.1');
assert.equal(desktopPackage.scripts.build, 'vue-tsc --noEmit && vite build');
assert.equal(desktopPackage.scripts.tauri, 'tauri');
assert.equal(desktopPackage.scripts['tauri:dev'], 'tauri dev');
assert.equal(desktopPackage.scripts['tauri:build'], 'tauri build');
assert.equal(desktopPackage.dependencies.vue.startsWith('^3.'), true);
assert.equal(desktopPackage.dependencies['vue-router'].startsWith('^4.'), true);
assert.equal(desktopPackage.devDependencies.vite.startsWith('^'), true);
assert.equal(desktopPackage.devDependencies['@vitejs/plugin-vue'].startsWith('^'), true);
assert.equal(desktopPackage.devDependencies.typescript.startsWith('^'), true);

const main = read('apps/desktop/src/main.ts');
assert.match(main, /createApp\(App\)/);
assert.match(main, /\.use\(router\)/);

const router = read('apps/desktop/src/router.ts');
for (const route of ['Target Accounts', 'Articles', 'Single Article', 'Albums', 'Settings']) {
  assert.match(router, new RegExp(route));
}

const app = read('apps/desktop/src/App.vue');
assert.match(app, /RouterLink/);
assert.match(app, /navItems/);
assert.match(app, /Desktop core feature set/);
for (const excluded of ['API', 'Public Proxy', 'Sponsorship', 'Developer']) {
  assert.doesNotMatch(app, new RegExp(excluded, 'i'));
}

const tauriConfig = JSON.parse(read('apps/desktop/src-tauri/tauri.conf.json'));
assert.equal(tauriConfig.productName, 'WeChat Article Exporter');
assert.equal(tauriConfig.build.devUrl, 'http://127.0.0.1:1420');
assert.equal(tauriConfig.build.frontendDist, '../dist');
assert.equal(tauriConfig.app.windows[0].title, 'WeChat Article Exporter');

const cargo = read('apps/desktop/src-tauri/Cargo.toml');
assert.match(cargo, /tauri = \{ version = "=2\./);
assert.match(cargo, /tauri-build = \{ version = "=2\./);

const rustTauriVersion = cargo.match(/tauri = \{ version = "=([^"]+)"/)?.[1];
assert.equal(rustTauriVersion, '2.8.0');
assert.equal(desktopPackage.dependencies['@tauri-apps/api'], rustTauriVersion);
assert.equal(desktopPackage.devDependencies['@tauri-apps/cli'], rustTauriVersion);
