const { execFileSync } = require('node:child_process');
const { copyFileSync, readFileSync } = require('node:fs');
const { createHash } = require('node:crypto');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
// Keep artifact selection explicit. Add and test other targets before supporting them.
if (process.platform !== 'win32' || process.arch !== 'x64') {
  throw new Error('This development build currently supports Windows x64 MSVC only.');
}
const jominiRoot = path.resolve(root, '../vendor/jomini');
const manifest = JSON.parse(readFileSync(path.join(root, 'jomini-source.json'), 'utf8'));
for (const [relative, expected] of Object.entries(manifest.files)) {
  const actual = createHash('sha256').update(readFileSync(path.join(jominiRoot, relative))).digest('hex');
  if (actual !== expected) throw new Error(`Jomini source changed: ${relative}. Review before refreshing the fingerprint.`);
}
execFileSync('cargo', ['build', '--release', '--locked', '--target', 'x86_64-pc-windows-msvc'], {
  cwd: root, stdio: 'inherit'
});
copyFileSync(
  path.join(root, 'target/x86_64-pc-windows-msvc/release/ck3_save_native.dll'),
  path.join(root, 'ck3_save_native.win32-x64-msvc.node')
);
console.log('Built ck3_save_native.win32-x64-msvc.node');
