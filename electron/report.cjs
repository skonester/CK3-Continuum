'use strict';
const fs = require('node:fs/promises');
const path = require('node:path');
const { randomUUID } = require('node:crypto');

function buildReport(saves, version) {
  return {
    reportVersion: 1,
    application: { name: 'CK3 Continuum', version },
    generatedAt: new Date().toISOString(),
    capability: 'read-only-inspection',
    compatibility: 'not-validated-in-game',
    conversionPerformed: false,
    targetResearchProfile: '1.19.0.6',
    limitations: [
      'Section differences can reflect campaign history, DLC, or mods as well as version changes.',
      'Mod dependencies are not detected by this prototype.',
      'ZIP integrity and syntactic inspection do not establish engine compatibility.',
      'Outer and inner metadata are not compared by the current inspector.'
    ],
    saves: { source: saves.source || null, reference: saves.reference || null }
  };
}

async function writeReport(destination, report) {
  if (path.extname(destination).toLowerCase() !== '.json') throw new Error('Choose a .json report filename.');
  // Link a completely written temporary sibling to a new name: never replace
  // an existing report, save, symlink, or hardlink. Windows/NTFS supported.
  const temporary = path.join(path.dirname(destination), `.continuum-${randomUUID()}.tmp`);
  try {
    const file = await fs.open(temporary, 'wx');
    try { await file.writeFile(JSON.stringify(report, null, 2) + '\n', 'utf8'); await file.sync(); }
    finally { await file.close(); }
    await fs.link(temporary, destination);
  } catch (error) {
    if (error.code === 'EEXIST') throw new Error('That file already exists. Choose a new report filename.');
    throw error;
  } finally {
    await fs.unlink(temporary).catch(() => {});
  }
}
module.exports = { buildReport, writeReport };
