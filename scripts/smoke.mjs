import { _electron as electron, expect } from '@playwright/test';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(fileURLToPath(new URL('..', import.meta.url)));
const out = path.join(root, 'test-output');
await mkdir(out, { recursive: true });
function fixture(version, entries) {
  const meta = 'meta_data={ version="' + version + '" meta_player_name="Bárbara" meta_title_name="A test realm" meta_date=1243.5.10 save_game_version=15 portraits_version=5 }\n';
  const header = 'SAV010000000000' + Buffer.byteLength(meta).toString(16).padStart(8, '0') + '\n';
  return Buffer.from(header + meta + 'provinces={ ' + entries + ' }\ntriggered_event={}\ntriggered_event={}\nliving={ 10={} }\nreligion={ religions={ 0={ template=christianity_religion } } faiths={ 11={ template=orthodox } } }\nlanded_titles={ landed_titles={ 1184={ key=e_byzantium name="Test realm" holder=10 } } }\n');
}
const fixtureSource = path.join(out, 'campaign.ck3');
const fixtureReference = path.join(out, 'reference.ck3');
await writeFile(fixtureSource, fixture('1.16.1', '1={} 2={}'));
await writeFile(fixtureReference, fixture('1.19.0.6', '1={} 2={} 3={}'));
const [source = fixtureSource, reference = fixtureReference] = process.argv.slice(2);
const hash = async file => createHash('sha256').update(await readFile(file)).digest('hex');
const before = await Promise.all([hash(source), hash(reference)]);
const env = { ...process.env, CK3_SMOKE: '1' };
delete env.ELECTRON_RUN_AS_NODE;
delete env.CONTINUUM_DEV;
const desktop = await electron.launch({
  ...(process.env.CONTINUUM_EXE ? { executablePath: process.env.CONTINUUM_EXE, args: [] } : { args: [root] }),
  env, timeout: 30_000
});
const errors = [];
try {
  const page = await desktop.firstWindow();
  page.on('pageerror', error => errors.push(error.message));
  await expect(page.getByRole('heading', { name: 'Your legacy, continued.' })).toBeVisible();
  await desktop.evaluate(({ BrowserWindow }) => {
    const win = BrowserWindow.getAllWindows()[0];
    win.webContents.setBackgroundThrottling(false);
    win.hide();
  });
  if (!process.env.CONTINUUM_EXE) await page.screenshot({ path: path.join(out, '01-overview.png'), fullPage: true });
  const isolated = await page.evaluate(() => ({ node: typeof window.require, process: typeof window.process, bridge: typeof window.continuum?.openSave }));
  expect(isolated).toEqual({ node: 'undefined', process: 'undefined', bridge: 'function' });
  const invalid = await page.evaluate(() => window.continuum.openSave('invalid'));
  expect(invalid.ok).toBe(false);
  const choose = async file => desktop.evaluate(({ dialog }, selected) => {
    dialog.showOpenDialog = async () => ({ canceled: false, filePaths: [selected] });
  }, file);
  await choose(source);
  await page.getByTestId('open-source').click();
  await expect(page.getByTestId('source-version')).toHaveText('1.16.1', { timeout: 30_000 });
  await expect(page.getByRole('button', { name: 'Export report' })).toBeEnabled();
  // Hold one real utility process at its request boundary to make cancellation
  // deterministic, independent of whether a tiny fixture parses in 2 ms.
  const waitingWorker = path.join(out, 'waiting-worker.cjs');
  await writeFile(waitingWorker, 'process.parentPort.on("message", () => { setInterval(() => {}, 1000); });');
  await desktop.evaluate(({ utilityProcess }, workerPath) => {
    const original = utilityProcess.fork;
    utilityProcess.fork = function (_module, args, options) {
      utilityProcess.fork = original;
      return original.call(this, workerPath, args, options);
    };
  }, waitingWorker);
  await choose(source);
  await page.getByTestId('open-source').click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('.notice')).toContainText('Inspection cancelled');
  await expect(page.getByTestId('source-version')).toHaveText('1.16.1');
  await choose(reference);
  await page.getByTestId('open-reference').click();
  await expect(page.getByText(path.basename(reference), { exact: true })).toBeVisible({ timeout: 30_000 });
  if (!process.env.CONTINUUM_EXE) await page.screenshot({ path: path.join(out, '02-inspected.png'), fullPage: true });
  await page.getByRole('button', { name: /Save structure/ }).click();
  await page.getByRole('textbox', { name: 'Filter sections' }).fill('provinces');
  await expect(page.locator('tbody tr')).toHaveCount(1);
  await expect(page.locator('tbody tr code')).toHaveText('provinces');
  if (!process.env.CONTINUUM_EXE) await page.screenshot({ path: path.join(out, '03-structure.png'), fullPage: true });
  const destination = path.join(out, 'desktop-report-' + Date.now() + '.json');
  await desktop.evaluate(({ dialog }, filePath) => {
    dialog.showSaveDialog = async () => ({ canceled: false, filePath });
  }, destination);
  await page.getByRole('button', { name: 'Export report' }).click();
  await expect(page.locator('.notice')).toContainText('Report saved as');
  const report = JSON.parse(await readFile(destination, 'utf8'));
  expect(report.conversionPerformed).toBe(false);
  expect(report.saves.source.name).toBe(path.basename(source));
  expect(report.saves.reference.name).toBe(path.basename(reference));
  await page.getByRole('button', { name: 'Export report' }).click();
  await expect(page.getByRole('alert')).toContainText('already exists');
  await page.getByRole('button', { name: 'Dismiss error' }).click();
  await page.getByRole('button', { name: 'Migration plan', exact: true }).click();
  await expect(page.getByTestId('convert-save')).toBeEnabled();
  const converted = path.join(out, 'ui-converted-' + Date.now() + '.ck3');
  await desktop.evaluate(({ dialog }, filePath) => {
    dialog.showSaveDialog = async () => ({ canceled: false, filePath });
  }, converted);
  await expect(page.locator('#conversion-mode')).toHaveValue('experimental-random-regions-1.16.1-to-1.19.0.6');
  await page.locator('#conversion-mode').selectOption(process.env.CONTINUUM_WORLD_SMOKE === '1' ? 'experimental-random-regions-1.16.1-to-1.19.0.6' : 'experimental-1.16.1-to-1.19.0.6');
  await page.getByTestId('convert-save').click();
  await expect(page.getByRole('region', { name: 'Conversion result' })).toBeVisible({ timeout: 90_000 });
  const conversionReport = JSON.parse(await readFile(converted + '.conversion.json', 'utf8'));
  expect(conversionReport.engineTested).toBe(false);
  expect(conversionReport.outputVerified).toBe(true);
  expect(conversionReport.unchangedSpansVerified).toBe(true);
  expect(conversionReport.counts['religion-faith-key']).toBeGreaterThan(0);
  await page.getByTestId('convert-save').click();
  await expect(page.getByRole('alert')).toContainText('already exists');
  await page.getByRole('button', { name: 'Dismiss error' }).click();

  // Cancellation before writing must publish neither a save nor its report.
  const cancelledPath = path.join(out, 'ui-cancelled-' + Date.now() + '.ck3');
  await desktop.evaluate(({ utilityProcess, dialog }, data) => {
    dialog.showSaveDialog = async () => ({ canceled: false, filePath: data.destination });
    const original = utilityProcess.fork;
    utilityProcess.fork = function (_module, args, options) {
      utilityProcess.fork = original;
      return original.call(this, data.waitingWorker, args, options);
    };
  }, { destination: cancelledPath, waitingWorker });
  await page.getByTestId('convert-save').click();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('.notice')).toContainText('Conversion cancelled');
  await expect(readFile(cancelledPath)).rejects.toThrow();
  await expect(readFile(cancelledPath + '.conversion.json')).rejects.toThrow();
  if (!process.env.CONTINUUM_EXE) await page.screenshot({ path: path.join(out, '04-conversion.png'), fullPage: true });

  const malformed = path.join(out, 'malformed.ck3');
  await writeFile(malformed, 'not a save');
  await page.getByRole('button', { name: 'Overview', exact: true }).click();
  await choose(malformed);
  await page.getByTestId('open-source').click();
  await expect(page.getByRole('alert')).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId('source-version')).toHaveText('1.16.1');
  await page.getByRole('button', { name: 'Dismiss error' }).click();
  await choose(source);
  await page.getByTestId('open-source').click();
  await expect(page.getByRole('button', { name: 'Export report' })).toBeEnabled({ timeout: 30_000 });
  expect(await Promise.all([hash(source), hash(reference)])).toEqual(before);
  expect(errors).toEqual([]);
  console.log('Desktop smoke passed: isolated renderer, dialogs, native inspection, comparison, cancellation, export, collision protection, error recovery, source hashes.');
  await writeFile(path.join(out, 'desktop-verification.json'), JSON.stringify({
    passed: true, source: path.basename(source), reference: path.basename(reference),
    originalHashesUnchanged: true, rendererErrors: errors, packaged: Boolean(process.env.CONTINUUM_EXE), conversion: true, conversionCancellation: true
  }, null, 2));
} finally { await desktop.close(); }
