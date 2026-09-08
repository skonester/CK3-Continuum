# Validation

The executable checks for this prototype are:

- `npm run build`: Svelte/TypeScript diagnostics and production bundling.
- `npm test`: 20 native/report/transaction tests in v0.3.1.
- `npm run test:desktop`: real Electron, synthetic file selections, renderer isolation, native inspection, reference comparison/filtering, JSON export, collision protection, invalid-input recovery, and unchanged original hashes.
- `CONTINUUM_EXE` + the desktop smoke: the same UI flow against an unpacked Windows build.

The smoke generates screenshots and verification JSON under ignored `test-output/`. It never opens CK3 or rewrites a save. Optional real-save smoke input is described in the README; no real save or private output is included in the repository.

Local execution results for this build are recorded after verification below. CI is configured but has not run on GitHub until the user pushes this repository.

## Initial inspector results (2026-09-08; historical)

- Svelte/TypeScript check: **0 errors, 0 warnings**. Production Vite build passed.
- Native build: passed with vendored Jomini and the locked Cargo dependency graph. Two upstream Jomini dead-code warnings remain; no native implementation changes were needed.
- Native/report tests: **9 passed**.
- Development Electron smoke: passed with synthetic fixtures and with the supplied modded 1.16.1/unmodded 1.19.0.6 research pair. Both original save hashes remained unchanged.
- Cancellation: tested with a real utility process held at its request boundary, followed by successful recovery. This makes cancellation deterministic rather than racing a millisecond-scale fixture parse.
- Packaged Windows x64 executable: passed the same functional smoke with synthetic fixtures, including loading the external native addon, comparison, JSON export, collision rejection, cancellation, and error recovery. No renderer JavaScript errors were reported.
- UI screenshots: captured and reviewed from the development build. The packaged smoke skips screenshots because Chromium timed out capturing the hidden window; its functional assertions still ran and passed.
- Git ignore checks: saves, generated binaries, node_modules, test reports/profiles, target, and release outputs are excluded.

Local runtimes: Node 25.8.1, Electron 44.2.0, Rust 1.94.0, Windows x64. CI selects Node 24 and Rust 1.94.0; that runner configuration has not yet executed on GitHub. No CK3 load/simulation/conversion test was performed.

## v0.2.0 conversion results (2026-09-08)

- Svelte build/check: zero errors and warnings. Rust formatting and clippy for our crate passed; the same two upstream Jomini dead-code warnings remain.
- **16 native/report/transaction tests passed**, covering exact plain/ZIP round trips, scoped changes, Unicode/comments, stale data, ambiguous schema fields, metadata disagreement, and overwrite/rollback protection.
- Development and packaged Electron smoke passed with conversion output/report creation, existing-name refusal, deterministic cancellation, and recovery.
- Real research pair: both writer-control outputs have identical gamestate/metadata bytes to their originals. The experimental output changes 33 religion records, 110 faith records, and 13,171 title-name records, plus the version label (17,474 byte edits).
- An independent Python audit checked ZIP CRCs, header/metadata agreement, all journal ranges/hashes, unchanged ranges, root multiplicity, IDs, names, and preserved player/character/history/trait/culture/province sections. Source hashes remain unchanged. Private inputs/outputs and audit reports remain outside the Git repository.
- Icon: all nine original ICO image payloads are present in the executable; the packaged runtime ICO is byte-identical to the supplied file.
- Engine validation remains **not performed**. Structural validity is not proof of CK3 loadability or campaign fidelity under the target engine.

## v0.2.1 archive correction

17 tests pass, including a strict embedded-ZIP addressing regression that failed with v0.2.0. Packaged desktop smoke passes. Both real controls and the experimental candidate were independently reverified; only four offset bytes changed per file. The user-observed v0.2.0 empty-map failure is recorded in [the incident report](empty-map-fix.md). Corrected CK3 loading is awaiting retest.


## v0.3.0 regional generation results

- 19 native/report/transaction tests pass, including fresh regional rulers, colliding IDs, dynasty-seat exclusion, target culture lookup, contract reciprocity, source family preservation, and missing/ambiguous reference rejection.
- Svelte/TypeScript reports zero errors and warnings. Production build and Rust formatting pass. Clippy passes with warnings denied for this crate; the two pre-existing vendored Jomini warnings remain.
- Development desktop smoke passes. Packaged Windows desktop smoke passes with the **new regional profile and the real supplied save pair**, exercising the reference-save IPC through the native worker, conversion/report publication, collision protection, cancellation, renderer isolation, and original-file hash checks.
- The real final candidate adds 839 ruling families (2,517 characters), 56 regional kingdoms, and 753 vassal contracts. All 31,823 original living characters remain byte-identical. Existing title owners, actual lieges, histories, claims, heirs, cultures, counties, and province data are preserved. New political links stay inside the generated regions.
- An independent Python audit verifies generated family/domain/contract reciprocity and all journal hashes/spans, metadata copies, ZIP CRC, and embedded archive offsets. The source/reference originals remain unchanged.
- The candidate is copied to the CK3 save directory as `old-to-1.19-random-kingdoms-v0.3.0.ck3`. The subsequent user test confirmed realms and portraits, then crashed on unpause; this candidate is superseded by v0.3.1.


## v0.3.1 first-tick correction

The user confirmed v0.3.0 fixed titles and poses, then crashed after unpausing. See [the crash incident](first-tick-crash.md). Twenty tests now pass, including reference regiment collision/missing-handle rejection and valid feudal contract groups. Rust clippy and the Svelte production check/build pass. The real-save independent audit now covers military handles, origins, and old-regiment preservation. v0.3.1 subsequently passed unpause and a saved simulation checkpoint; reload remains pending.

The packaged v0.3.1 app also passed the full desktop smoke using the real source/reference pair and the regional profile. Comparison of v0.3.0/v0.3.1 journals confirms that only new province military bindings, new regiment records, and new contract groups changed.

## v0.3.1 engine checkpoint (2026-09-08)

The user reported a successful v0.3.1 simulation and save on 2026-09-08. The preserved `regions-v0_3_1-retest_ck3.ck3` independently confirms **85 days of advancement, 1243.5.10 to 1243.8.3**. All 2,517 generated characters remain living; all 839 generated county owners are unchanged; all 3,036 new holding military references resolve to the correct province. No new county has a missing/unheld liege title, and no new political links lead into the original titles. Exit/reload, succession, and longer simulation remain untested.

CK3 retained all 4,023 new title records. Four generated kingdoms (Jiangxi, Malayadvipa, Xingyuan, and Gobi) have dated destruction history during June/July, leaving 52 held kingdoms. Their 67 former vassal contracts are absent and the affected counties now have no liege, so these are coherent political changes rather than dangling links. Three surviving contracts changed to `tribal_vassal`; 1,223 barony holders changed or became absent. The audit records these changes without asserting their cause or full political fidelity. The player remains Barbara with culture 209 and faith 11, but metadata now displays Byzantine Empire instead of Holy Latin Empire; that naming change remains unresolved.

Evidence: [engine re-save audit](../../analysis/engine-retest-v0.3.1.json), [reproducible audit script](../../analysis/audit_engine_retest.py). The save and available session logs are preserved under `../../analysis/regional-month-retest-v0.3.1/`. Logs extend beyond the saved checkpoint; they are not proof of a reload.
