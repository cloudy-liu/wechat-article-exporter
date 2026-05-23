import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const workspaceRoot = path.resolve(import.meta.dirname, '..');
const releaseDir = path.join(workspaceRoot, 'src-tauri/target/release');
const bundleDir = path.join(workspaceRoot, 'src-tauri/target/release/bundle');
const executableName = 'wechat-article-exporter-desktop.exe';
const executablePath = path.join(releaseDir, executableName);
const launchSmoke = process.argv.includes('--launch-smoke');
const requireInstaller = process.argv.includes('--require-installer');

if (process.platform !== 'win32') {
  throw new Error(`Windows MVP artifact verification must run on win32, got ${process.platform}`);
}

if (process.arch !== 'x64') {
  throw new Error(`Windows MVP artifact verification must run on x64, got ${process.arch}`);
}

assertFile(executablePath, 'release executable');
console.log(`Windows x64 release executable: ${path.relative(workspaceRoot, executablePath)}`);

const bundleArtifacts = fs.existsSync(bundleDir)
  ? collectFiles(bundleDir).filter(filePath => {
      const extension = path.extname(filePath).toLowerCase();
      return extension === '.msi' || extension === '.exe';
    })
  : [];

if (requireInstaller) {
  assert.ok(
    bundleArtifacts.length > 0,
    `expected at least one optional installable Windows bundle artifact under ${bundleDir}`,
  );
}

for (const artifact of bundleArtifacts) {
  console.log(`optional installable Windows bundle artifact: ${path.relative(workspaceRoot, artifact)}`);
}

if (launchSmoke) {
  await verifyLaunchSmoke(executablePath);
}

function assertFile(filePath, label) {
  assert.ok(fs.existsSync(filePath), `missing ${label}: ${filePath}`);
  assert.ok(fs.statSync(filePath).isFile(), `${label} is not a file: ${filePath}`);
}

function collectFiles(directoryPath) {
  const files = [];
  for (const entry of fs.readdirSync(directoryPath, { withFileTypes: true })) {
    const entryPath = path.join(directoryPath, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectFiles(entryPath));
    } else {
      files.push(entryPath);
    }
  }
  return files;
}

async function verifyLaunchSmoke(filePath) {
  const child = spawn(filePath, [], {
    detached: true,
    stdio: 'ignore',
    windowsHide: true,
  });

  child.unref();
  await delay(3000);

  const stillRunning = isProcessRunning(child.pid);
  if (!stillRunning) {
    throw new Error(`packaged app launch smoke failed; process ${child.pid} exited too early`);
  }

  process.kill(child.pid);
  console.log(`Windows packaged app launch smoke passed with pid ${child.pid}`);
}

function isProcessRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function delay(milliseconds) {
  return new Promise(resolve => {
    setTimeout(resolve, milliseconds);
  });
}
