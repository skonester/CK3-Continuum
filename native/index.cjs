'use strict';
if (process.platform !== 'win32' || process.arch !== 'x64') {
  throw new Error('No CK3 native binary is built for this platform/architecture.');
}
const binding = require('./ck3_save_native.win32-x64-msvc.node');
// Public callers enqueue one job at a time to bound simultaneous native work.
let queue = Promise.resolve();
exports.inspectSave = function inspectSave(filePath, maxBytes = 512 * 1024 * 1024) {
  if (typeof filePath !== 'string' || filePath.length === 0 || filePath.includes('\0')) {
    return Promise.reject(new TypeError('filePath must be a nonempty path without NUL bytes.'));
  }
  if (!Number.isInteger(maxBytes) || maxBytes < 1 || maxBytes > 2 * 1024 * 1024 * 1024) {
    return Promise.reject(new RangeError('maxBytes must be an integer from 1 to 2147483648.'));
  }
  const job = queue.then(() => binding.inspectSave(filePath, maxBytes));
  queue = job.catch(() => {});
  return job;
};

exports.writeCandidate = function writeCandidate(input, output, expected, mode, reference) {
  for (const value of [input, output]) {
    if (typeof value !== 'string' || !value || value.includes('\0')) return Promise.reject(new TypeError('Expected a nonempty path without NUL bytes.'));
  }
  if (typeof expected !== 'string' || !/^[a-f0-9]{64}$/.test(expected)) return Promise.reject(new TypeError('Expected the inspected gamestate SHA-256.'));
  if (!['roundtrip', 'experimental-1.16.1-to-1.19.0.6', 'experimental-random-regions-1.16.1-to-1.19.0.6'].includes(mode)) return Promise.reject(new TypeError('Unknown conversion profile.'));
  if (reference !== undefined && (typeof reference !== 'string' || !reference || reference.includes('\0'))) return Promise.reject(new TypeError('Expected a nonempty reference path without NUL bytes.'));
  const job = queue.then(() => binding.writeCandidate(input, output, expected, mode, reference));
  queue = job.catch(() => {});
  return job;
};