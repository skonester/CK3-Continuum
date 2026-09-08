import { inspectSave, writeCandidate } from '../native/index.mjs';
import { mkdir, mkdtemp, rm, readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import conversion from '../electron/conversion.cjs';
const [old, modern, destination] = process.argv.slice(2);
if (!old || !modern || !destination) throw new Error('Usage: node scripts/create-test-candidates.mjs <1.16.1.ck3> <modern.ck3> <output-directory>');
const outputDir = path.resolve(destination);
await mkdir(outputDir, { recursive: true });
const runs = [
  [modern, 'roundtrip', 'modern-writer-control.ck3'],
  [old, 'roundtrip', 'old-writer-control.ck3'],
  [old, 'experimental-random-regions-1.16.1-to-1.19.0.6', 'old-to-1.19.0.6-experimental.ck3']
];
const hash = async file => createHash('sha256').update(await readFile(file)).digest('hex');
for (const [input, mode, name] of runs) {
  const before = await hash(input);
  const inspected = await inspectSave(input);
  const stage = await mkdtemp(path.join(outputDir, '.continuum-convert-'));
  try {
    const stagedFile = path.join(stage, 'candidate.ck3');
    const result = await writeCandidate(input, stagedFile, inspected.gamestateSha256, mode, modern);
    if (await hash(input) !== before || result.sourceSha256 !== before) throw new Error('Original source changed during the test.');
    const report = conversion.reportFor(result, path.basename(input), name);
    const summary = await conversion.commitCandidate(stagedFile, path.join(outputDir, name), report);
    console.log(JSON.stringify({ name, changes: result.changes.length, counts: summary.counts, bytes: result.outputBytes, sourceUnchanged: true }));
  } finally {
    if (path.dirname(stage) === outputDir && path.basename(stage).startsWith('.continuum-convert-')) {
      await rm(stage, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
    }
  }
}
