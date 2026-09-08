'use strict';
const { app, BrowserWindow, dialog, ipcMain, utilityProcess } = require('electron');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const fs = require('node:fs/promises');
const { buildReport, writeReport } = require('./report.cjs');
const { reportFor, commitCandidate } = require('./conversion.cjs');

const smoke = !app.isPackaged && process.env.CK3_SMOKE === '1';
if (smoke) {
  const profile = path.join(__dirname, '../test-output/electron-profile');
  require('node:fs').mkdirSync(profile, { recursive: true });
  app.setPath('userData', profile);
}
app.setAppUserModelId('org.ck3continuum.prototype');
const devUrl = !app.isPackaged && process.env.CONTINUUM_DEV === '1' ? 'http://127.0.0.1:5173/' : null;
const pageUrl = devUrl || pathToFileURL(path.join(__dirname, '../dist/index.html')).href;
let window;
let busy = false;
let job = null;
const saves = { source: null, reference: null };
const sourcePaths = { source: null, reference: null };

function trusted(event) {
  return window && event.sender === window.webContents &&
    event.senderFrame === window.webContents.mainFrame && event.senderFrame.url === pageUrl;
}
function handle(channel, fn) {
  ipcMain.handle(channel, async (event, ...args) => {
    if (!trusted(event)) return { ok: false, error: 'This request did not come from the application window.' };
    try { return { ok: true, value: await fn(...args) }; }
    catch (error) { return { ok: false, error: error.message || String(error), cancelled: error.code === 'CANCELLED' }; }
  });
}
function runNative(request) {
  return new Promise((resolve, reject) => {
    const nativePath = app.isPackaged ? path.join(process.resourcesPath, 'native/index.cjs') : path.join(__dirname, '../native/index.cjs');
    const child = utilityProcess.fork(path.join(__dirname, 'inspect-worker.cjs'), [], {
      serviceName: request.operation === 'convert' ? 'CK3 test conversion' : 'CK3 save inspector', stdio: 'pipe'
    });
    let settled = false;
    const finish = (error, result) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      job = null;
      child.kill();
      if (error) reject(error); else resolve(result);
    };
    const timeout = setTimeout(() => finish(new Error('The save operation timed out after five minutes.')), 300_000);
    job = { cancel: () => finish(Object.assign(new Error('Operation cancelled.'), { code: 'CANCELLED' })) };
    child.on('spawn', () => child.postMessage({ ...request, nativePath }));
    child.on('message', (data) => data.ok ? finish(null, data.value) : finish(new Error(data.error)));
    child.on('error', (error) => finish(error));
    child.on('exit', (code) => finish(new Error('The save process exited unexpectedly (' + code + ').')));
    child.stdout?.on('data', () => {});
    child.stderr?.on('data', () => {});
  });
}
handle('save:open', async (slot) => {
  if (slot !== 'source' && slot !== 'reference') throw new Error('Invalid save slot.');
  if (busy) throw new Error('Wait for the current operation to finish.');
  busy = true;
  try {
    const selected = await dialog.showOpenDialog(window, {
      title: slot === 'source' ? 'Open your campaign save' : 'Open a reference save',
      properties: ['openFile'], filters: [{ name: 'Crusader Kings III saves', extensions: ['ck3'] }]
    });
    if (selected.canceled || !selected.filePaths.length) return null;
    const filePath = selected.filePaths[0];
    if (path.extname(filePath).toLowerCase() !== '.ck3') throw new Error('Choose a .ck3 save file.');
    const info = await fs.stat(filePath);
    if (!info.isFile() || info.size > 2 ** 31) throw new Error('Choose a regular save file no larger than 2 GiB.');
    const inspection = await runNative({ operation: 'inspect', filePath });
    const result = { name: path.basename(filePath), inspectedAt: new Date().toISOString(), inspection };
    saves[slot] = result;
    sourcePaths[slot] = filePath;
    return result;
  } finally { busy = false; }
});
handle('save:cancel', () => { if (!job) return false; job.cancel(); return true; });
handle('save:convert', async (mode) => {
  if (busy) throw new Error('Wait for the current operation to finish.');
  if (!saves.source || !sourcePaths.source) throw new Error('Inspect a campaign first.');
  if (!['roundtrip', 'experimental-1.16.1-to-1.19.0.6', 'experimental-random-regions-1.16.1-to-1.19.0.6'].includes(mode)) throw new Error('Unknown conversion profile.');
  if (mode !== 'roundtrip' && saves.source.inspection.metadata.find(f => f.key === 'version')?.value !== '1.16.1') {
    throw new Error('The experimental migration profile requires version 1.16.1. Use writer control for other supported text saves.');
  }
  if (mode === 'experimental-random-regions-1.16.1-to-1.19.0.6' && (!sourcePaths.reference || saves.reference?.inspection.metadata.find(f => f.key === 'version')?.value !== '1.19.0.6')) {
    throw new Error('Open a 1.19.0.6 reference save to initialize new regions.');
  }
  busy = true;
  let staging;
  let parent;
  try {
    const stem = path.basename(saves.source.name, '.ck3');
    const selected = await dialog.showSaveDialog(window, {
      title: mode === 'roundtrip' ? 'Create writer-control save' : 'Create experimental CK3 1.19 test save',
      defaultPath: stem + (mode === 'roundtrip' ? '_writer_control_v' + app.getVersion() + '.ck3' : '_1.19_test_v' + app.getVersion() + '.ck3'),
      filters: [{ name: 'Crusader Kings III test save', extensions: ['ck3'] }]
    });
    if (selected.canceled || !selected.filePath) return null;
    const destination = path.resolve(selected.filePath);
    if (path.extname(destination).toLowerCase() !== '.ck3') throw new Error('Choose a .ck3 output filename.');
    for (const candidate of [destination, destination + '.conversion.json']) {
      try { await fs.lstat(candidate); throw new Error('The save or conversion report already exists. Choose a new filename.'); }
      catch (error) { if (error.code !== 'ENOENT') throw error; }
    }
    parent = path.dirname(destination);
    staging = await fs.mkdtemp(path.join(parent, '.continuum-convert-'));
    const outputPath = path.join(staging, 'candidate.ck3');
    const result = await runNative({
      operation: 'convert', filePath: sourcePaths.source, outputPath,
      expected: saves.source.inspection.gamestateSha256, mode, referencePath: sourcePaths.reference
    });
    const report = reportFor(result, saves.source.name, path.basename(destination));
    return await commitCandidate(outputPath, destination, report);
  } finally {
    if (staging && path.dirname(path.resolve(staging)) === parent && path.basename(staging).startsWith('.continuum-convert-')) {
      await fs.rm(staging, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 }).catch(() => {});
    }
    busy = false;
  }
});
handle('report:export', async () => {
  if (busy) throw new Error('Wait for the current operation to finish.');
  if (!saves.source) throw new Error('Inspect a campaign save first.');
  busy = true;
  try {
    const result = await dialog.showSaveDialog(window, {
      title: 'Export inspection report', defaultPath: 'continuum-report-' + Date.now() + '.json',
      filters: [{ name: 'JSON inspection report', extensions: ['json'] }]
    });
    if (result.canceled || !result.filePath) return null;
    await writeReport(result.filePath, buildReport(saves, app.getVersion()));
    return path.basename(result.filePath);
  } finally { busy = false; }
});
app.whenReady().then(async () => {
  window = new BrowserWindow({
    width: 1360, height: 940, minWidth: 900, minHeight: 680,
    title: 'CK3 Continuum', backgroundColor: '#101818', show: !smoke,
    icon: app.isPackaged ? path.join(process.resourcesPath, 'images/ck3.ico') : path.join(__dirname, '../images/ck3.ico'),
    autoHideMenuBar: true,
    webPreferences: { preload: path.join(__dirname, 'preload.cjs'), contextIsolation: true,
      nodeIntegration: false, sandbox: true, webSecurity: true }
  });
  window.webContents.setWindowOpenHandler(() => ({ action: 'deny' }));
  window.webContents.on('will-navigate', (event) => event.preventDefault());
  window.webContents.session.setPermissionRequestHandler((_contents, _permission, callback) => callback(false));
  window.webContents.session.setPermissionCheckHandler(() => false);
  await window.loadURL(pageUrl);
}).catch((error) => { console.error(error.message); app.exit(1); });
app.on('before-quit', () => job?.cancel());
app.on('window-all-closed', () => app.quit());
