import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import electron from 'electron';
const root = fileURLToPath(new URL('..', import.meta.url));
if (!existsSync(new URL('../native/ck3_save_native.win32-x64-msvc.node', import.meta.url))) {
  throw new Error('Run npm run native:build before starting the desktop app.');
}
const server = await createServer({ root });
await server.listen();
const env = { ...process.env, CONTINUUM_DEV: '1' };
delete env.ELECTRON_RUN_AS_NODE;
const child = spawn(electron, [root], { cwd: root, env, stdio: 'inherit', windowsHide: true });
child.on('error', async error => { console.error(error.message); await server.close(); process.exitCode = 1; });
child.on('exit', async code => { await server.close(); process.exitCode = code ?? 1; });
process.on('SIGINT', () => child.kill());
process.on('SIGTERM', () => child.kill());
