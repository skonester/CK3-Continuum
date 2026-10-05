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
// Mirrors supported_source_version in native/src/conversion.rs: 1.16.1 through 1.18.x.
function supportedSourceVersion(version, mode) {
  if (mode === 'experimental-1.19.0.6-to-1.20.0.3') return version === '1.19.0.6';
  const m = /^1\.(\d+)(?:\.|$)/.exec(version || '');
  if (!m) return false;
  const minor = Number(m[1]);
  return minor >= 16 && minor <= 18;
}
function targetVersion(mode) {
  return mode === 'experimental-1.19.0.6-to-1.20.0.3' ? '1.20.0.3' : '1.19.0.6';
}
function reportFor(result, sourceName, outputName) {
  const counts = {};
  for (const change of result.changes) counts[change.rule] = (counts[change.rule] || 0) + 1;
  return {
    reportVersion: 1, writerVersion, kind: 'conversion-test-candidate', createdAt: new Date().toISOString(),
    sourceName, outputName, engineTested: false, compatibility: 'unverified',
    warnings: result.profile === 'roundtrip'
      ? ['No migration was applied. This writer-control copy still needs a CK3 load/save/reload test.']
      : result.profile === 'experimental-1.19.0.6-to-1.20.0.3' ? [
        'Experimental 1.20 schema migration; CK3 load, simulation, and save/reload testing is required.',
        'Campaign faith IDs remain stable. Each old faith has its own rite and religious organization; old faiths that became rites in 1.20 remain independent custom faiths instead of being merged.',
        'Explicit character, county, state, and holy-order affiliations use the new rite schema. Secret religion retains secret_faith and its campaign faith ID. Missing source affiliations remain absent; no religion is guessed.',
        'New clerical regions, leases, saints, house relations, great projects, and situation sub-managers begin empty. Bishops, church politics, and active situations need in-game validation.',
        'Scholar/poet lookup names are migrated while indices and XP arrays are preserved. New poet XP is not initialized. Portrait versions and DNA are preserved.',
        'Province IDs must match the reference. Existing map data, campaign histories, mods, and advanced systems are preserved, but changed definitions and engine behavior remain unverified.',
        ...(counts['domicile-owner-title'] ? [`${counts['domicile-owner-title']} missing camp/estate owners were restored from campaign title links; buildings, provisions, and locations were preserved.`] : []),
        ...(counts['orphan-domicile-tombstone'] ? [`${counts['orphan-domicile-tombstone']} unowned camps/estates had no campaign title link and were replaced with database tombstones; their unused buildings and resources were removed. See orphan-domicile-tombstone journal entries.`] : []),
        ...(counts['unresolved-holy-site'] ? [`${counts['unresolved-holy-site']} holy sites have no target reference definition. Their IDs are preserved with no barony binding; see unresolved-holy-site journal entries.`] : []),
        ...(counts['opinion-decay-schema'] ? ['Legacy opinion decay uses flat modify and total-duration days; cached values, dates, owner/target pairs and punishments are preserved. Elapsed days are not copied as total duration. Changed modifier definitions can affect future decay.'] : []),
        ...(counts['contract-group-recovery'] ? [`${counts['contract-group-recovery']} blank contract groups were recovered from subject governments. Known obligations and selected levels are mapped by historical names; new obligations use target defaults.`] : []),
        ...(counts['contract-republic-default'] ? [`${counts['contract-republic-default']} malformed five-entry republic contracts were replaced with the target's single fixed republic obligation. Their old layout cannot be interpreted as valid republic rights; any nonzero selections in it are discarded. See contract-republic-default journal entries and retain the original save.`] : []),
        ...(counts['accolade-type-schema'] ? ['Old accolade attributes retain their IDs, names, glory, ownership and history. Their six automatic glory ranks become two three-level attributes, allocated alternately to primary and secondary with a minimum of one each. This is a migration policy; bonuses can change with the redesigned system.'] : []),
        ...(counts['accolade-owned-list-with-inactive'] ? ['Inactive accolade IDs and any existing acclaimed-knight assignments are retained in their owner lists. The modern system removed deactivation; previously inactive knights may reactivate and vacant records may become eligible for automatic succession. No new knight assignment is created by the converter.'] : []),
        ...(counts['culture-spread-schema'] ? ['Culture exposure becomes spread; research progress, fascination and the source culture are preserved. Spread now follows the target acceptance rules. Scholar perks and bombard innovations use their current definition names.'] : []),
        ...(counts['obsolete-queued-on-action'] ? [`${counts['obsolete-queued-on-action']} queued instances of confirmed removed on-actions were dropped; other scheduled events are preserved.`] : []),
        'Other removed or source-mod definitions (including modifiers, regiment types, story cycles and title templates) may still produce errors. They are retained for separate content migration; a fresh CK3 error log is required.'
      ] : result.profile === 'experimental-random-regions-1.16.1-to-1.19.0.6' ? [
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
module.exports = { reportFor, commitCandidate, supportedSourceVersion, targetVersion, WARNINGS };
