'use strict';
const fs = require('node:fs/promises');
const path = require('node:path');
const { version: writerVersion } = require('../package.json');
const WARNINGS = [
  'Experimental structural conversion only; CK3 load, simulation, and save/reload compatibility have not been established.',
  'World/map reconciliation is not implemented. Missing or changed provinces and title definitions remain unresolved.',
  'Source trait lookup and indices are preserved. The poet/lifestyle_poet change and experience mapping are not resolved.',
  'Portrait versions and DNA are preserved; portrait migration is not implemented.',
  'Mod dependencies, generated-world initialization, administrative/economic changes, and new managers are not reconciled.',
  'The target version label identifies this test candidate; it is not a compatibility certificate.'
];
function reportFor(result, sourceName, outputName) {
  const counts = {};
  for (const change of result.changes) counts[change.rule] = (counts[change.rule] || 0) + 1;
  return {
    reportVersion: 1, writerVersion, kind: 'conversion-test-candidate', createdAt: new Date().toISOString(),
    sourceName, outputName, engineTested: false, compatibility: 'unverified',
    warnings: result.profile === 'roundtrip'
      ? ['No migration was applied. This writer-control copy still needs a CK3 load/save/reload test.']
      : result.profile === 'experimental-random-regions-1.16.1-to-1.19.0.6' ? [
        'Fresh families rule independent feudal regional kingdoms with county vassals; no reference character histories or political owners were imported.',
        'Missing static titles are registered, including China and Japanese/Korean throne titles used by portrait triggers. Portrait correction still needs visual confirmation in CK3.',
        'Original character records, DNA, cultures, counties, and provinces are preserved; the culture-template lookup is rebuilt for the target definitions.',
        'New regions use the reference save baseline for development, buildings, and cultural innovations. Special eastern governments, estates, and imperial situations are not initialized.',
        'Changed existing geography, source mods, traits/XP, and advanced systems remain unresolved. Load, advance, save, and reload testing is required.'
      ] : WARNINGS,
    counts, ...result
  };
}
async function commitCandidate(stagedSave, destination, report) {
  if (path.extname(destination).toLowerCase() !== '.ck3') throw new Error('Choose a .ck3 output filename.');
  const reportPath = destination + '.conversion.json';
  const stagedReport = path.join(path.dirname(stagedSave), 'conversion.json');
  await fs.writeFile(stagedReport, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
  const handle = await fs.open(stagedReport, 'r+');
  try { await handle.sync(); } finally { await handle.close(); }
  let linkedReport = false;
  try {
    await fs.link(stagedReport, reportPath);
    linkedReport = true;
    await fs.link(stagedSave, destination);
  } catch (error) {
    // Roll back only the report link created by this attempt, and only while
    // it still identifies our staged file. Existing files are never replaced.
    if (linkedReport) {
      const [ours, current] = await Promise.all([fs.stat(stagedReport), fs.stat(reportPath).catch(() => null)]);
      if (current && ours.dev === current.dev && ours.ino === current.ino) await fs.unlink(reportPath);
    }
    if (error.code === 'EEXIST') throw new Error('The save or conversion report already exists. Choose a new filename.');
    throw error;
  }
  return { outputName: path.basename(destination), reportName: path.basename(reportPath),
    profile: report.profile, counts: report.counts, outputBytes: report.outputBytes,
    outputSha256: report.outputSha256, warnings: report.warnings, engineTested: false };
}
module.exports = { reportFor, commitCandidate, WARNINGS };
