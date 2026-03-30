#!/usr/bin/env node

const { spawnSync } = require('node:child_process');
const path = require('node:path');

function detectLibc() {
  if (process.platform !== 'linux') return null;
  try {
    const report = process.report && process.report.getReport ? process.report.getReport() : null;
    const glibc = report && report.header ? report.header.glibcVersionRuntime : null;
    return glibc ? 'gnu' : 'musl';
  } catch {
    return 'musl';
  }
}

function resolvePackageName() {
  const key = `${process.platform}-${process.arch}`;
  if (key === 'linux-x64') return '@memohjs/af-linux-x64-musl';
  if (key === 'linux-arm64') return '@memohjs/af-linux-arm64-musl';
  if (key === 'darwin-x64') return '@memohjs/af-darwin-x64';
  if (key === 'darwin-arm64') return '@memohjs/af-darwin-arm64';
  if (key === 'win32-x64') return '@memohjs/af-win32-x64-msvc';
  const libc = detectLibc();
  throw new Error(`Unsupported platform: ${process.platform}-${process.arch}${libc ? `-${libc}` : ''}`);
}

function resolveBinaryPath(pkgName) {
  const pkgJson = require.resolve(`${pkgName}/package.json`);
  const pkgRoot = path.dirname(pkgJson);
  const binName = process.platform === 'win32' ? 'af.exe' : 'af';
  return path.join(pkgRoot, 'bin', binName);
}

function main() {
  try {
    const pkgName = resolvePackageName();
    const binPath = resolveBinaryPath(pkgName);
    const result = spawnSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
    if (result.error) {
      throw result.error;
    }
    process.exit(result.status === null ? 1 : result.status);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[af npm] ${msg}`);
    console.error('[af npm] Try downloading binary from GitHub Releases: https://github.com/memohai/Autofish/releases');
    process.exit(1);
  }
}

main();
