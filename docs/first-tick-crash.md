# v0.3.0 first-tick crash and v0.3.1 correction

The user confirmed that the new titles/kingdoms and wives' poses looked correct in v0.3.0. The game crashed immediately after pressing Play. Do not continue that candidate.

Crash artifacts were preserved locally in `../analysis/regional-crash-20260908`. The exception is a write access violation at ck3.exe RVA `0x21fb6e9`, targeting `0x30000138`. Inspection of the dump and installed executable shows a lookup using handle 13430, a missing registry slot, followed by an attempted write through the fallback object. The fallback object is at `0x30000010`.

The initializer incorrectly treated `provinces/<id>/holding/levy` and `garrison` as ordinary numeric values. These fields are handles into `armies/regiments`. For example, the reference's province 10453 uses levy 13430; v0.3.0 copied that handle but did not add regiment 13430. Other copied handles collide with regiments belonging to old provinces. This is a converter defect. The original topology audit did not cover military references and missed it.

v0.3.1 allocates fresh regiment IDs above occupied source slots. Each new holding's levy/garrison points to a fresh record whose `origin` equals that province. Existing regiments remain unchanged. New levies are unraised; garrisons start with fresh chunks and no references to reference-world army regiments. Only numeric maximum size is taken from the reference. A mismatched reference origin or source type rejects conversion.

The logs separately showed one invalid empty `contract_group` reference for each generated vassal contract. Generated contracts now use the installed `feudal_vassal` group. Existing source contracts are preserved.

Regression tests were first run against v0.3.0 and failed, then passed with the fixes. The full suite contains 20 passing tests. The independent real-save audit now also checks every new levy/garrison ID, origin, uniqueness, preservation of existing regiments, and valid new contract groups. Passing those checks does not replace an engine unpause/save/reload test.

The replacement is generated from the untouched source as `old-to-1.19-random-kingdoms-v0.3.1.ck3`. It retains the same new families, regions, and portrait correction. The first unpause and saved simulation checkpoint have now passed.

The user reported a successful v0.3.1 simulation and save on 2026-09-08. The preserved `regions-v0_3_1-retest_ck3.ck3` independently confirms **85 days of advancement, 1243.5.10 to 1243.8.3**. All 2,517 generated characters remain living; all 839 generated county owners are unchanged; all 3,036 new holding military references resolve to the correct province. No new county has a missing/unheld liege title, and no new political links lead into the original titles. Exit/reload, succession, and longer simulation remain untested.

CK3 retained all 4,023 new title records. Four generated kingdoms (Jiangxi, Malayadvipa, Xingyuan, and Gobi) have dated destruction history during June/July, leaving 52 held kingdoms. Their 67 former vassal contracts are absent and the affected counties now have no liege, so these are coherent political changes rather than dangling links. Three surviving contracts changed to `tribal_vassal`; 1,223 barony holders changed or became absent. The audit records these changes without asserting their cause or full political fidelity. The player remains Barbara with culture 209 and faith 11, but metadata now displays Byzantine Empire instead of Holy Latin Empire; that naming change remains unresolved.

Evidence: [engine re-save audit](../../analysis/engine-retest-v0.3.1.json), [reproducible audit script](../../analysis/audit_engine_retest.py). The save and available session logs are preserved under `../../analysis/regional-month-retest-v0.3.1/`. Logs extend beyond the saved checkpoint; they are not proof of a reload.
