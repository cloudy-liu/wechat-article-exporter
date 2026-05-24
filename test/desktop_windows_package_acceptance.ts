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
  exists('apps/desktop/scripts/verify-windows-mvp-artifact.mjs'),
  true,
  'desktop workspace must include a Windows MVP artifact verifier',
);
assert.equal(
  exists('apps/desktop/WINDOWS_MVP_ACCEPTANCE.md'),
  true,
  'desktop workspace must document Windows MVP acceptance',
);

const desktopPackage = JSON.parse(read('apps/desktop/package.json'));
assert.equal(
  desktopPackage.scripts['build:windows-mvp'],
  'tauri build --no-bundle',
);
assert.equal(
  desktopPackage.scripts['verify:windows-mvp'],
  'node scripts/verify-windows-mvp-artifact.mjs',
);
assert.equal(
  desktopPackage.scripts['bundle:windows-installer'],
  'tauri bundle --bundles nsis,msi',
);
assert.equal(
  desktopPackage.scripts['verify:windows-installer'],
  'node scripts/verify-windows-mvp-artifact.mjs --require-installer',
);

const tauriConfig = JSON.parse(read('apps/desktop/src-tauri/tauri.conf.json'));
assert.equal(tauriConfig.bundle.useLocalToolsDir, true);

const mainRs = read('apps/desktop/src-tauri/src/main.rs');
assert.match(mainRs, /windows_subsystem\s*=\s*"windows"/);
assert.match(mainRs, /not\(debug_assertions\)/);

const verifier = read('apps/desktop/scripts/verify-windows-mvp-artifact.mjs');
for (const marker of [
  'src-tauri/target/release',
  '--launch-smoke',
  '--require-installer',
  '.exe',
  'assertWindowsGuiSubsystem',
  'Windows subsystem: GUI',
  'expected Windows GUI subsystem',
  'optional installable Windows bundle artifact',
  'Windows bundler local tools directory',
  'installer bundle verification enabled',
  'installer bundle output directory',
  'installer bundle output artifacts',
  'Windows bundler local tools directory exists',
  'Windows bundler local tools present',
  'HTTP_PROXY',
  'HTTPS_PROXY',
  'TAURI_BUNDLER_TOOLS_GITHUB_MIRROR',
  'TAURI_BUNDLER_TOOLS_GITHUB_MIRROR_TEMPLATE',
  'win32',
  'x64',
]) {
  assert.match(verifier, new RegExp(marker.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i'));
}

const acceptance = read('apps/desktop/WINDOWS_MVP_ACCEPTANCE.md');
for (const marker of [
  'Windows x64 desktop artifact',
  'runnable Windows artifact',
  'tauri build --no-bundle',
  'packaged app launches on Windows',
  'Official Account Login',
  'Target Official Account search',
  'article list synchronization',
  'article HTML download',
  'Markdown export',
  'HTML export',
  'Restart behavior',
  'local archive data',
  'task visibility',
  'Logout',
  'credential clearing',
  'Known first-release limitations',
  'bundle:windows-installer',
  'verify:windows-installer',
  'useLocalToolsDir',
  'target/.tauri',
  'TAURI_BUNDLER_TOOLS_GITHUB_MIRROR',
  'TAURI_BUNDLER_TOOLS_GITHUB_MIRROR_TEMPLATE',
  'HTTP_PROXY',
  'HTTPS_PROXY',
  'installer artifact count',
  'local Tauri tool-cache state',
  'denied access',
  'network timed out',
  'installer bundle is optional',
  'final local acceptance by the project owner',
]) {
  assert.match(acceptance, new RegExp(marker.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i'));
}
