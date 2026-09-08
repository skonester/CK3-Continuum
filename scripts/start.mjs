import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import electron from 'electron';
const root = fileURLToPath(new URL('..', import.meta.url));
for (const file of ['dist/index.html', 'native/ck3_save_native.win32-x64-msvc.node']) {
  if (!existsSync(new URL('../' + file, import.meta.url))) throw new Error('Run npm run native:build and npm run build first. Missing: ' + file);
}
const env = { ...process.env };
delete env.ELECTRON_RUN_AS_NODE;
const child = spawn(electron, [root], { cwd: root, env, stdio: 'inherit', windowsHide: true });
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
