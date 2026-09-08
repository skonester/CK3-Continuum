const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { buildReport, writeReport } = require('../electron/report.cjs');
test('report retains duplicate metadata and separates inspection from compatibility', () => {
  const source = { name: 'example.ck3', inspection: { metadata: [{ key: 'a', value: '1' }, { key: 'a', value: '2' }] } };
  const report = buildReport({ source }, '0.1.0');
  assert.equal(report.conversionPerformed, false);
  assert.equal(report.compatibility, 'not-validated-in-game');
  assert.deepEqual(report.saves.source.inspection.metadata, source.inspection.metadata);
  assert.equal(report.saves.reference, null);
});
test('report export commits complete JSON and never overwrites a file or save', async () => {
  const root = path.join(__dirname, '../test-output');
  await fs.mkdir(root, { recursive: true });
  const folder = await fs.mkdtemp(path.join(root, 'report-test-'));
  const output = path.join(folder, 'report.json');
  const report = buildReport({}, '0.1.0');
  await writeReport(output, report);
  assert.deepEqual(JSON.parse(await fs.readFile(output, 'utf8')), report);
  await assert.rejects(writeReport(output, { overwritten: true }), /already exists/);
  assert.deepEqual(JSON.parse(await fs.readFile(output, 'utf8')), report);
  const save = path.join(folder, 'original.ck3');
  await fs.writeFile(save, 'untouched');
  await assert.rejects(writeReport(save, report), /\.json/);
  assert.equal(await fs.readFile(save, 'utf8'), 'untouched');
  assert.deepEqual((await fs.readdir(folder)).sort(), ['original.ck3', 'report.json']);
});
