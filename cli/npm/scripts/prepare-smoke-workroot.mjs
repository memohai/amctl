#!/usr/bin/env node

import fs from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

function usage() {
  console.error(
    'Usage: node prepare-smoke-workroot.mjs --workroot <dir> --version <version> --platform-package <name> --binary <path>',
  );
}

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (!arg.startsWith('--')) {
      throw new Error(`unexpected argument: ${arg}`);
    }
    const key = arg.slice(2);
    const value = argv[i + 1];
    if (!value || value.startsWith('--')) {
      throw new Error(`missing value for --${key}`);
    }
    args[key] = value;
    i += 1;
  }
  return args;
}

async function rewritePackageVersions(rootDir, version) {
  const entries = await fs.readdir(rootDir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(rootDir, entry.name);
    if (entry.isDirectory()) {
      await rewritePackageVersions(fullPath, version);
      continue;
    }
    if (entry.isFile() && entry.name === 'package.json') {
      const raw = await fs.readFile(fullPath, 'utf8');
      await fs.writeFile(fullPath, raw.replaceAll('__VERSION__', version));
    }
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const workroot = args.workroot;
  const version = args.version;
  const platformPackage = args['platform-package'];
  const binaryPath = args.binary;

  if (!workroot || !version || !platformPackage || !binaryPath) {
    usage();
    process.exit(1);
  }

  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const npmRoot = path.resolve(scriptDir, '..');
  const metaTemplateDir = path.join(npmRoot, 'meta');
  const platformsTemplateDir = path.join(npmRoot, 'platforms');
  const selectedPlatformDir = path.join(platformsTemplateDir, platformPackage);

  await fs.access(metaTemplateDir);
  await fs.access(selectedPlatformDir);
  await fs.access(binaryPath);

  await fs.rm(workroot, { recursive: true, force: true });
  await fs.mkdir(workroot, { recursive: true });

  const metaDir = path.join(workroot, 'meta');
  const platformsDir = path.join(workroot, 'platforms');
  await fs.cp(metaTemplateDir, metaDir, { recursive: true });
  await fs.mkdir(platformsDir, { recursive: true });
  await fs.cp(selectedPlatformDir, path.join(platformsDir, platformPackage), {
    recursive: true,
  });

  await rewritePackageVersions(workroot, version);

  const platformPkgPath = path.join(platformsDir, platformPackage, 'package.json');
  const platformPkg = JSON.parse(await fs.readFile(platformPkgPath, 'utf8'));

  const metaPkgPath = path.join(metaDir, 'package.json');
  const metaPkg = JSON.parse(await fs.readFile(metaPkgPath, 'utf8'));
  const tarballName = `${platformPkg.name.replace(/^@/, '').replace(/\//g, '-')}-${version}.tgz`;
  const files = new Set(Array.isArray(metaPkg.files) ? metaPkg.files : []);
  files.add('vendor');
  metaPkg.files = [...files];
  metaPkg.optionalDependencies = {
    [platformPkg.name]: `file:./vendor/${tarballName}`,
  };
  await fs.writeFile(`${metaPkgPath}`, `${JSON.stringify(metaPkg, null, 2)}\n`);
  await fs.mkdir(path.join(metaDir, 'vendor'), { recursive: true });

  const binName = path.basename(binaryPath);
  const platformBinDir = path.join(platformsDir, platformPackage, 'bin');
  const platformBinPath = path.join(platformBinDir, binName);
  await fs.mkdir(platformBinDir, { recursive: true });
  await fs.copyFile(binaryPath, platformBinPath);
  if (!binName.endsWith('.exe')) {
    await fs.chmod(platformBinPath, 0o755);
  }

  console.log(
    JSON.stringify({
      workroot,
      metaDir,
      platformDir: path.join(platformsDir, platformPackage),
      binary: platformBinPath,
      platformTarball: tarballName,
    }),
  );
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
