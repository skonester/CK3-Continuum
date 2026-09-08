'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { deflateRawSync } = require('node:zlib');
const { createHash } = require('node:crypto');
const { inspectSave } = require('../index.cjs');
const out = path.resolve(__dirname, '../test-output');
fs.mkdirSync(out, { recursive: true });
const meta = 'meta_data={ version="1.19.0.6" save_game_version=15 portraits_version=5 }\n';
function header(kind, metadata = meta) {
  return Buffer.from(`SAV01${kind}00000000${Buffer.byteLength(metadata).toString(16).padStart(8, '0')}\n`);
}
function fixture(name, content) {
  const file = path.join(out, name);
  fs.writeFileSync(file, content);
  return file;
}
function textFixture(name, rest, kind = '00') {
  return fixture(name, Buffer.concat([header(kind), Buffer.from(meta + rest)]));
}
function crc32(buf) {
  let crc = 0xffffffff;
  for (const b of buf) {
    crc ^= b;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
function zipFixture(name, brokenCrc = false) {
  const raw = Buffer.from(meta + 'provinces={ 1={} 2={} }\n');
  const deflated = deflateRawSync(raw);
  const prefix = Buffer.concat([header('02'), Buffer.from(meta)]);
  const filename = Buffer.from('gamestate');
  const crc = (crc32(raw) ^ (brokenCrc ? 1 : 0)) >>> 0;
  const local = Buffer.alloc(30);
  local.writeUInt32LE(0x04034b50, 0); local.writeUInt16LE(20, 4);
  local.writeUInt16LE(8, 8); local.writeUInt32LE(crc, 14);
  local.writeUInt32LE(deflated.length, 18); local.writeUInt32LE(raw.length, 22);
  local.writeUInt16LE(filename.length, 26);
  const central = Buffer.alloc(46);
  central.writeUInt32LE(0x02014b50, 0); central.writeUInt16LE(20, 4);
  central.writeUInt16LE(20, 6); central.writeUInt16LE(8, 10);
  central.writeUInt32LE(crc, 16); central.writeUInt32LE(deflated.length, 20);
  central.writeUInt32LE(raw.length, 24); central.writeUInt16LE(filename.length, 28);
  central.writeUInt32LE(prefix.length, 42);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0); end.writeUInt16LE(1, 8); end.writeUInt16LE(1, 10);
  end.writeUInt32LE(central.length + filename.length, 12);
  end.writeUInt32LE(prefix.length + local.length + filename.length + deflated.length, 16);
  return { file: fixture(name, Buffer.concat([prefix, local, filename, deflated, central, filename, end])), raw };
}
test('CJS and ESM expose the same asynchronous native inspector', async () => {
  const esm = await import('../index.mjs');
  assert.equal(esm.inspectSave, inspectSave);
});
test('duplicate roots, mixed values, comments, escapes, and UTF-8 survive inventory', async () => {
  const rest = 'triggered_event={ event="first" }\ntriggered_event={ event="second" }\n' +
    'provinces={ 1={} 2=none }\nlevels={ 11 0=2 1=2 }\n' +
    'names={ "Latin { # }" "Bárbara" "quoted \\"name\\"" }\n# ignored }\n';
  const file = textFixture('syntax.ck3', rest);
  const report = await inspectSave(file);
  const sections = Object.fromEntries(report.sections.map(s => [s.key, s]));
  assert.equal(sections.triggered_event.occurrences, 2);
  assert.equal(sections.triggered_event.childEntries, 2);
  assert.equal(sections.provinces.childEntries, 2);
  assert.equal(sections.levels.childEntries, 3);
  assert.equal(sections.names.childEntries, 3);
  assert.equal(report.crcVerified, false);
  assert.equal(report.gamestateSha256, createHash('sha256').update(meta + rest).digest('hex'));
});
test('compressed text is decompressed and CRC/size verified', async () => {
  const { file, raw } = zipFixture('valid-zip.ck3');
  const result = await inspectSave(file);
  assert.equal(result.crcVerified, true);
  assert.equal(result.gamestateBytes, raw.length);
  assert.equal(result.gamestateSha256, createHash('sha256').update(raw).digest('hex'));
});
test('bad ZIP CRC rejects even when its text is valid', async () => {
  const { file } = zipFixture('bad-crc.ck3', true);
  await assert.rejects(inspectSave(file), /checksum|crc/i);
});
test('malformed/truncated syntax is rejected', async () => {
  for (const [name, rest] of Object.entries({ open: 'x={', close: '}', value: 'x=', key: '=1', quote: 'x="unfinished' })) {
    await assert.rejects(inspectSave(textFixture(`bad-${name}.ck3`, rest)));
  }
});
test('binary input and invalid file headers are explicit errors', async () => {
  await assert.rejects(inspectSave(textFixture('binary.ck3', 'x=1', '01')), /binary saves/);
  await assert.rejects(inspectSave(fixture('bad-header.ck3', Buffer.from('not a save'))));
});
test('limits and failed-job recovery work', async () => {
  const file = textFixture('limits.ck3', 'x={ 1 2 3 }');
  await assert.rejects(inspectSave(file, Buffer.byteLength(meta) + 1), /maxBytes/);
  await assert.rejects(inspectSave(file, -1), /maxBytes/);
  await assert.rejects(inspectSave(''), /filePath/);
  await assert.rejects(inspectSave(textFixture('depth.ck3', 'x=' + '{'.repeat(513) + '}'.repeat(513))), /nesting/);
  const report = await inspectSave(file);
  assert.equal(report.metadata.find(f => f.key === 'version').value, '1.19.0.6');
});
