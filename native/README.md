# Native save inspector

Rust edition 2024 cdylib using napi-rs (Node-API 8), SHA-256, and vendored Jomini 0.35.0 with the envelope feature. Windows x64 MSVC only.

From the repository root run `npm run native:build`. This verifies `jomini-source.json`, runs locked Cargo compilation, and emits `ck3_save_native.win32-x64-msvc.node` here. Generated artifacts are ignored.

```js
const { inspectSave } = require('./native/index.cjs');
const result = await inspectSave('C:/path/campaign.ck3');
// ESM: import { inspectSave } from './native/index.mjs';
```

See `index.d.ts` for the complete type. Results include ordered inner scalar metadata, repeated-root counts, immediate child-entry counts, compressed/uncompressed byte sizes, gamestate SHA-256, ZIP verification status, token count, maximum depth, and elapsed time.

Inspection is asynchronous. The Electron app invokes it in an isolated utility process and kills that process to cancel. There is no running-task cancellation method inside the addon itself.

Limits: 2 GiB input cap; 512 MiB default expanded gamestate cap (standalone API permits up to 2 GiB); depth 512; 1 MiB token buffer; bounded section and metadata results. Binary saves are explicitly rejected. The inspector alone does not compare metadata copies or validate the full CK3 schema. The separate experimental writer compares metadata and creates candidates.

`native/test/native.test.cjs` verifies syntax, CRC, input limits, errors, and recovery. Tests use synthetic data. The original Jomini license is retained alongside the addon and in the vendor directory.

## Experimental writer

writeCandidate(input, newOutput, expectedGamestateSha256, mode) supports roundtrip and experimental-1.16.1-to-1.19.0.6. It creates a new staged file only. Use the Electron transaction to publish it with a journal. Direct native callers must clean their own staging file after a write/readback failure.

Accepted: SAV kinds 00/02, UTF-8, identical outer/inner metadata, single gamestate ZIP entry, no archive comments or ZIP64. Other layouts are rejected. Writer limits: 512 MiB input/expanded data, 4 MiB metadata, one million immediate entries per index, 250,000 patches. Conversion retains full source/output buffers and can use several times the expanded size in peak memory.

Jomini validates grammar and CRC; a separate byte-span traversal selects exact paths. Output is re-read through Jomini. Journals include rule, path, input span, replacement length, and before/after hashes. Untouched spans are verified. Full graph and engine validation remain outstanding.
