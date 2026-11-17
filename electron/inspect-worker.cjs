'use strict';
process.parentPort.once('message', async ({ data }) => {
  try {
    const native = require(data.nativePath);
    const value = data.operation === 'convert'
      ? await native.writeCandidate(data.filePath, data.outputPath, data.expected, data.mode, data.referencePath || undefined)
      : data.operation === 'lint'
        ? await native.lintSave(data.filePath, data.expected)
        : await native.inspectSave(data.filePath);
    process.parentPort.postMessage({ ok: true, value });
  } catch (error) {
    process.parentPort.postMessage({ ok: false, error: error instanceof Error ? error.message : String(error) });
  }
});
