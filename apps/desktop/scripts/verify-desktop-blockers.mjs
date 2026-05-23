import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const workspaceRoot = path.resolve(import.meta.dirname, '..');

const dependencyBlockers = [
  'ag-grid-enterprise',
  'ag-grid-vue3',
  '@ag-grid-community',
  'nuxt',
  'nitro',
  '@sentry',
  'umami',
  'cloudflare',
  'wrangler',
];

const sourceBlockers = [
  ...dependencyBlockers,
  'web analytics',
  'Public API',
  'Public Proxy',
  'Sponsorship',
  'support pages',
  'Developer',
  'developer/debug',
  'server/api',
  "'/api/",
  '"/api/',
  '`/api/',
];

const textFileExtensions = new Set([
  '.css',
  '.html',
  '.json',
  '.js',
  '.mjs',
  '.rs',
  '.toml',
  '.ts',
  '.vue',
]);

const sourceRoots = [
  'index.html',
  'package.json',
  'vite.config.ts',
  'src',
  path.join('src-tauri', 'Cargo.toml'),
  path.join('src-tauri', 'tauri.conf.json'),
  path.join('src-tauri', 'src'),
];

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(workspaceRoot, relativePath), 'utf8'));
}

function collectTextFiles(relativePath) {
  const absolutePath = path.join(workspaceRoot, relativePath);
  const stats = fs.statSync(absolutePath);

  if (stats.isFile()) {
    return textFileExtensions.has(path.extname(absolutePath)) ? [absolutePath] : [];
  }

  const files = [];
  for (const entry of fs.readdirSync(absolutePath, { withFileTypes: true })) {
    if (entry.name === 'node_modules' || entry.name === 'dist' || entry.name === 'target') {
      continue;
    }
    files.push(...collectTextFiles(path.join(relativePath, entry.name)));
  }
  return files;
}

function assertNoBlockers(label, value, blockers) {
  const normalized = value.toLowerCase();
  const hits = blockers.filter(blocker => normalized.includes(blocker.toLowerCase()));
  assert.deepEqual(hits, [], `${label} includes desktop blocker(s): ${hits.join(', ')}`);
}

const desktopPackage = readJson('package.json');
for (const section of ['dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies']) {
  const dependencyNames = Object.keys(desktopPackage[section] || {});
  assertNoBlockers(`package.json ${section}`, dependencyNames.join('\n'), dependencyBlockers);
}

for (const [scriptName, scriptCommand] of Object.entries(desktopPackage.scripts || {})) {
  assertNoBlockers(`package.json script ${scriptName}`, scriptCommand, dependencyBlockers);
}

for (const relativePath of sourceRoots) {
  for (const absolutePath of collectTextFiles(relativePath)) {
    const projectPath = path.relative(workspaceRoot, absolutePath);
    const content = fs.readFileSync(absolutePath, 'utf8');
    assertNoBlockers(projectPath, content, sourceBlockers);
  }
}

console.log('desktop blocker verification passed');
