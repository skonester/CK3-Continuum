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

## v0.4.0: 1.19.0.6 → 1.20.0.3 (2026-10-05)

- **27 automated tests pass**, including existing parser/writer/regional regressions and new faith/rite migration, campaign/reference ID collisions, holy-order affiliations, custom faiths, unsupported versions/maps, missing references, ambiguous schemas, journal integrity, and input preservation.
- Native build, Rust formatting and clippy pass. Svelte/TypeScript reports **zero errors and warnings**, and the production frontend builds. The two existing vendored Jomini dead-code warnings remain.
- Development desktop smoke passes for both the legacy synthetic pair and the supplied Barbara/Murchad pair. The new path checks automatic profile selection, wrong/missing references, incompatible profiles, conversion/report publication, cancellation, collision refusal, renderer isolation, and unchanged input hashes.
- The packaged Windows app passes the same desktop flow with the real save pair. The standalone portable executable passes extraction, startup, UI, renderer-isolation, and bridge checks from a folder containing only the executable.
- The supplied Barbara save is embedded **1.19.0.6**, schema 15, dated **1283.8.10**; the Murchad reference is **1.20.0.3**, schema 17, dated **1066.9.17**. Both have **13,269 province IDs**.
- The candidate has **91,816 changed spans**: 91,290 scoped affiliation-key changes, 520 holy-site rewrites, two trait aliases, two metadata changes, faith relocation, and one insertion of the new managers. It retains **140 faiths**, creates **140 rites and organizations**, and preserves all **35,847 living**, **58,471 unprunable dead**, and **20,406 prunable dead** character records outside the affiliation-key changes.
- An independent Python audit verifies whole-file/gamestate hashes, every journal hash and unchanged byte range, metadata copies, ZIP CRC/addressing, character IDs and preserved traits/XP/DNA/families, faith links, county/title/holy-order values, and unchanged culture/dynasty/province/army/war/player sections. Both writer controls retain their input gamestate bytes. It confirms **87,383 explicit character affiliations** and **390 secret affiliations** retain their numeric values; **27,341 missing character affiliations** remain absent.
- The unmatched `segrada_familia` holy site is preserved with an unbound barony and a report warning. No reference ruler, religious head, saint, artifact inventory, or political history is imported.

Local private outputs and the independent audit are under ignored `test-output/migration-1.20/`; the candidate and controls were generated in the supplied test-save folder. At the time of this build, CK3 testing was pending. The user's subsequent test loaded Barbara's character screen, then crashed during a domicile update; **v0.4.0 is superseded by v0.4.1**. See [the crash investigation](domicile-crash-1.20.md).

## v0.4.1: domicile owner correction (2026-10-05)

- The latest Paradox minidump identifies a write through an invalid domicile owner-title handle. Camp 731's owner is recovered as campaign title 16790283, rather than the county at its current location. Runtime type information and the dump's object IDs match the source records.
- **29 automated tests pass**, including ownership recovery for camps and estates, preservation of valid links and resources, orphan tombstones, conflicting/duplicate owner rejection, missing-domicile rejection, and the retained `secret_faith` field. The report explicitly warns about removed orphan assets.
- Native build, Rust formatting and clippy pass. The Svelte production check/build passes with zero errors or warnings. The two existing vendored Jomini warnings remain.
- The packaged Windows v0.4.1 app passes the desktop smoke with the real Barbara/Murchad pair. The standalone portable executable passes isolated extraction/startup, UI, bridge, and renderer checks.
- The replacement candidate restores **229** camp/estate owner handles and tombstones **nine** unowned records (one camp, eight estates). Its **91,664** changed spans include the existing faith/rite migration; all **390** secret-faith fields now remain byte-identical to the source.
- The independent audit verifies every journal hash and unchanged byte span, source/reference hashes, archive CRC/addressing, metadata copies, all **114,724** character IDs and retained data, religious handles, the reciprocal title/domicile graph, and every owned domicile's buildings, construction queues, provisions, herd, and location bytes. Neither original input changed.
- Candidate SHA-256: `4fb3e4fd07438ca15b151e9f597bb226a92ee8f61e8f928b24248ede8cd927b8`. The audit is preserved locally as `test-output/migration-1.20/independent-audit-v0.4.1.json`.

The new candidate is `barbara-1.20.0.3-v0.4.1.ck3`, generated from the original source/reference pair, with a separate conversion journal. The subsequent user test confirms successful simulation; save/reload, church politics, succession, and extended simulation remain pending. Old accolade/opinion schema and removed-content errors remain unresolved; the ownership correction is not proof that all 1.20 migration work is complete.

## v0.4.1 engine simulation and log review (2026-10-05)

The user reports simulation with no crashes. Preserved debug logs show dated effects through **1283.9.12**, at least **33 days** after the candidate's **1283.8.10** start, followed by `Quit: Quit from inside game`. No engine re-save/reload checkpoint was provided, so the evidence establishes simulation and normal exit only.

The error log reaches exactly **100,000 timestamped errors** and stops midway through the accolade database before simulation. This appears to be a logging limit, so it cannot establish that no further errors occurred. The main visible issues are **88,519** rejected opinion-decay fields, **5,579** accolade-field errors, **636** culture-exposure errors, and **701** missing queued actions. Parent-record inspection also identifies **3,398** blank vassal-contract groups that may affect taxes and obligations. The earlier `secret_rite` error is absent. See [the cleanup review](error-log-cleanup-1.20.md) for data-preserving migration priorities and remaining fidelity concerns. The session logs and grouped summary are preserved under ignored `test-output/logs-1.20-20261005-055214/`.

## v0.4.2: legacy-schema cleanup (2026-10-05)

- Rust formatting/clippy and the native build pass. All 33 regression tests pass, including fractional opinion adjustments, elapsed/total-day separation, contract obligation shifts/defaults, valid tributary retention, malformed republic fallbacks, accolade choices/history/list retention, existing assignments on inactive accolades, alias collisions, scoped queue cleanup, and empty references.
- Svelte/TypeScript and the Windows portable build pass. The packaged desktop converts the real Barbara/Murchad pair and passes reference guards, cancellation, renderer isolation, collision protection and original-hash checks. The single-file portable passes extraction/startup/UI/bridge checks.
- The real candidate contains 223,703 patches and is 31,670,055 bytes. SHA-256: `af8444f99dae9350766f274dcf2bc9f7f8cec8e795d784f0948078644b46876f`.
- Independent checks preserve all 114,724 character IDs and unaffected DNA/family/history fields, 390 secret faiths and 27,341 absent affiliations. They verify 107,538 relationship/opinion records, 103,566 temporary opinions with cached values and timing fields, 4,185 contract identities with mapped selections, 4,473 accolade IDs/attributes/owners/knights/history, 280 cultures with research progress, and 229 owned domiciles with resources intact. Every v0.4.1 migration journal entry remains identical.
- The audit verifies source/reference/output hashes, every journal/unchanged span, synchronized metadata, ZIP CRC and embedded offsets. Its report is preserved under ignored `test-output/migration-1.20/independent-audit-v0.4.2.json`.
- The 1,741 incompatible republic layouts use the fixed target obligation; 16 contain nonzero discarded malformed selections. Accolade level allocation and inactive-list handling are explicit migration policies, with report warnings. See [the cleanup review](error-log-cleanup-1.20.md).

The new candidate is `barbara-1.20.0.3-v0.4.2.ck3`, generated from the original pair. **CK3 loading/simulation and error-count reduction for v0.4.2 are unverified.** Remaining individual content migrations, save/exit/reload, church politics, succession, and extended simulation are pending. The v0.4.1 simulation evidence does not validate these new changes.
