import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const guiDir = path.resolve(scriptDir, '..');
const packageJson = JSON.parse(fs.readFileSync(path.join(guiDir, 'package.json'), 'utf8'));

const pkgName = 'mtklogo-gui';
const pkgVersion = `${packageJson.version}-1`;
const pkgArch = 'x86_64';
const pkgDir = path.join(guiDir, 'src-tauri', 'target', 'release', 'bundle', 'arch');
const bundleDir = path.join(guiDir, 'src-tauri', 'target', 'release', 'bundle', 'deb');

const runtimeDependencies = [
  'cairo',
  'desktop-file-utils',
  'gdk-pixbuf2',
  'glib2',
  'gtk3',
  'hicolor-icon-theme',
  'libsoup',
  'pango',
  'webkit2gtk-4.1',
];

const iconSizes = [16, 32, 48, 64, 128, 256, 1024];

function run(command, args, options = {}) {
  execFileSync(command, args, { stdio: 'inherit', ...options });
}

function findLatestDeb() {
  const debFiles = fs
    .readdirSync(bundleDir)
    .filter((entry) => entry.endsWith('.deb'))
    .map((entry) => ({
      path: path.join(bundleDir, entry),
      name: entry,
      mtimeMs: fs.statSync(path.join(bundleDir, entry)).mtimeMs,
    }))
    .sort((left, right) => right.mtimeMs - left.mtimeMs);

  if (debFiles.length === 0) {
    throw new Error(`No Debian bundle found in ${bundleDir}`);
  }

  const versionMatch = debFiles.find((entry) => entry.name.includes(`_${packageJson.version}_`));
  return versionMatch ? versionMatch.path : debFiles[0].path;
}

function walkFiles(root) {
  const entries = [];

  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const fullPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      entries.push(...walkFiles(fullPath));
      continue;
    }

    if (entry.isFile()) {
      entries.push(fullPath);
    }
  }

  return entries;
}

function installedSize(root) {
  return walkFiles(root).reduce((total, filePath) => total + fs.statSync(filePath).size, 0);
}

function writePkgInfo(root, size) {
  const lines = [
    `pkgname = ${pkgName}`,
    `pkgbase = ${pkgName}`,
    `pkgver = ${pkgVersion}`,
    'pkgdesc = Tauri GUI for MTK logo images.',
    'url = https://github.com/arlept/mtkimgrs',
    `builddate = ${Math.floor(Date.now() / 1000)}`,
    'packager = GitHub Actions',
    `size = ${size}`,
    `arch = ${pkgArch}`,
    'license = MIT',
    'license = Apache-2.0',
    ...runtimeDependencies.map((depend) => `depend = ${depend}`),
    '',
  ];

  fs.writeFileSync(path.join(root, '.PKGINFO'), `${lines.join('\n')}`);
}

function rewriteDesktopEntry(root) {
  const desktopDir = path.join(root, 'usr', 'share', 'applications');
  const sourcePath = path.join(desktopDir, 'mtklogo GUI.desktop');
  const targetPath = path.join(desktopDir, 'mtklogo-gui.desktop');

  if (!fs.existsSync(sourcePath)) {
    throw new Error(`Missing desktop entry: ${sourcePath}`);
  }

  const desktopContents = fs
    .readFileSync(sourcePath, 'utf8')
    .replace(/^Categories=.*$/m, 'Categories=Utility;');

  fs.writeFileSync(targetPath, desktopContents);
  fs.rmSync(sourcePath);
}

function installIconSet(root) {
  const sourceIcon = path.join(guiDir, 'src-tauri', 'icons', 'icon.png');
  const iconBaseDir = path.join(root, 'usr', 'share', 'icons', 'hicolor');
  const pixmapDir = path.join(root, 'usr', 'share', 'pixmaps');

  if (!fs.existsSync(sourceIcon)) {
    throw new Error(`Missing source icon: ${sourceIcon}`);
  }

  fs.mkdirSync(pixmapDir, { recursive: true });
  execFileSync('magick', [sourceIcon, '-resize', '256x256', path.join(pixmapDir, 'mtklogo-gui.png')], {
    stdio: 'inherit',
  });

  for (const size of iconSizes) {
    const sizeDir = path.join(iconBaseDir, `${size}x${size}`, 'apps');
    fs.mkdirSync(sizeDir, { recursive: true });
    execFileSync('magick', [sourceIcon, '-resize', `${size}x${size}`, path.join(sizeDir, 'mtklogo-gui.png')], {
      stdio: 'inherit',
    });
  }
}

function createMtree(root) {
  const mtree = execFileSync(
    'bsdtar',
    ['--format=mtree', '-cf', '-', '-C', root, '.'],
    { encoding: 'utf8' },
  );

  fs.writeFileSync(path.join(root, '.MTREE'), mtree);
}

function createPackage(root, outputPath) {
  const archive = execFileSync('bsdtar', ['-cf', '-', '-C', root, '.PKGINFO', '.MTREE', 'usr'], {
    maxBuffer: 50 * 1024 * 1024,
  });
  execFileSync('zstd', ['-q', '-19', '-T0', '-f', '-o', outputPath], {
    input: archive,
  });
}

function main() {
  const debPath = findLatestDeb();
  const outputDir = pkgDir;
  const outputPath = path.join(outputDir, `${pkgName}-${pkgVersion}-${pkgArch}.pkg.tar.zst`);
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'mtklogo-arch-'));
  const extractRoot = path.join(tempRoot, 'payload');

  try {
    fs.mkdirSync(extractRoot, { recursive: true });
    fs.mkdirSync(outputDir, { recursive: true });

    run('ar', ['x', debPath], { cwd: tempRoot });
    run('bsdtar', ['-xzf', 'data.tar.gz', '-C', extractRoot], { cwd: tempRoot });

    rewriteDesktopEntry(extractRoot);
    installIconSet(extractRoot);

    const size = installedSize(extractRoot);
    writePkgInfo(extractRoot, size);
    createMtree(extractRoot);
    createPackage(extractRoot, outputPath);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
}

main();
