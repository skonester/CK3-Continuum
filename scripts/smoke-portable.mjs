import { chromium, expect } from '@playwright/test';
import { copyFile, mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const pkg = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
const artifactName = pkg.build.artifactName.replace('${version}', pkg.version).replace('${ext}', 'exe');
const artifact = process.env.CONTINUUM_EXE || path.join(root, pkg.build.directories.output, artifactName);
const out = path.join(root, 'test-output');
await mkdir(out, { recursive: true });
const isolated = await mkdtemp(path.join(out, 'portable-'));
const executable = path.join(isolated, artifactName);
// Only the distributable is copied: no win-unpacked directory or adjacent resources.
await copyFile(artifact, executable);
const server = createServer();
await new Promise((resolve, reject) => {
  server.once('error', reject);
  server.listen(0, '127.0.0.1', resolve);
});
const port = server.address().port;
await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
const endpoint = `http://127.0.0.1:${port}`;
const env = { ...process.env };
delete env.ELECTRON_RUN_AS_NODE;
delete env.CONTINUUM_DEV;
delete env.NODE_OPTIONS;
const child = spawn(executable, [
  `--remote-debugging-port=${port}`,
  '--remote-debugging-address=127.0.0.1',
  `--user-data-dir=${path.join(isolated, 'profile')}`
], { cwd: isolated, env, windowsHide: true, stdio: 'ignore' });
let exited = false;
let launchError;
child.once('error', error => { launchError = error; });
child.once('exit', () => { exited = true; });
let browser;
try {
  // NSIS does not forward Electron's debugger stdout; discover Chromium over HTTP.
  const deadline = Date.now() + 90_000;
  let ready = false;
  while (Date.now() < deadline) {
    if (launchError) throw launchError;
    if (exited) throw new Error('Portable launcher exited before the app became ready.');
    try {
      const response = await fetch(`${endpoint}/json/version`, { signal: AbortSignal.timeout(1_000) });
      if (response.ok) { ready = true; break; }
    } catch { /* Wait for extraction and Chromium startup. */ }
    await delay(250);
  }
  if (!ready) throw new Error('Portable app did not start within 90 seconds.');
  browser = await chromium.connectOverCDP(endpoint, { timeout: 15_000 });
  const context = browser.contexts()[0];
  const page = context.pages()[0] || await context.waitForEvent('page', { timeout: 15_000 });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await expect(page.getByRole('heading', { name: 'Your legacy, continued.' })).toBeVisible();
  await expect(page.getByTestId('open-source')).toBeEnabled();
  await expect(page.getByTestId('open-reference')).toBeEnabled();
  expect(page.url()).toContain('app.asar');
  const isolation = await page.evaluate(() => ({
    node: typeof window.require, process: typeof window.process,
    bridge: typeof window.continuum?.openSave
  }));
  expect(isolation).toEqual({ node: 'undefined', process: 'undefined', bridge: 'function' });
  const reply = await page.evaluate(() => window.continuum.openSave('invalid'));
  expect(reply.ok).toBe(false);
  expect(reply.error).toContain('Invalid save slot');
  expect(errors).toEqual([]);
  await writeFile(path.join(out, 'portable-verification.json'), JSON.stringify({
    passed: true, artifact: artifactName, isolatedExecutable: true,
    rendererIsolation: true, mainProcessBridge: true, rendererErrors: errors
  }, null, 2));
  console.log('Portable smoke passed: single-file extraction, app startup, UI, renderer isolation, and main-process bridge.');
} finally {
  if (browser) {
    try {
      const session = await browser.newBrowserCDPSession();
      await Promise.race([session.send('Browser.close').catch(() => {}), delay(1_000)]);
    } catch { /* The debugging connection can close before the reply arrives. */ }
    await browser.close();
  }
  const deadline = Date.now() + 10_000;
  while (!exited && !launchError && Date.now() < deadline) await delay(100);
  if (!exited && child.pid) spawnSync('taskkill', ['/pid', String(child.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' });
}
