# 1.20 domicile crash investigation (v0.4.1)

The user's v0.4.0 Barbara candidate loaded the character screen, then crashed in CK3 1.20.0.3. The latest report was `ck3_20261005_052912`, created on 2026-10-05 at 05:29:12 America/Chicago, after about 48 seconds of process uptime. The report and dump were preserved locally under ignored `test-output/crash-1.20-20261005-052912/`.

## Evidence

- The dump records exception `C0000005` at `ck3.exe + 0x25ba530`. The faulting instruction writes a county-title ID through the engine's read-only null-title object.
- MSVC runtime type information identifies the objects involved as `CDomicile` and `CLandedTitle`. The domicile has ID **731** and `owner_title=4294967295`, the invalid handle. The routine reads that owner handle before trying to update the title's location field.
- Campaign title **16790283**, `x_d_laamp_1620` (Keepers of Allah), already points to `domicile=731`. This supplies the correct owner without importing reference IDs or assigning the camp to the county where it is located.
- All **238** active source domiciles have invalid owner handles. **229** have exactly one campaign title pointing to them. The remaining **nine** have no title link: one camp and eight estates.
- The converted save in the game's save directory matches the v0.4.0 output hash. The crash report's `last_save.ck3` instead matches the Murchad reference byte-for-byte; it is not the Barbara input and was not used as the migration source.
- The loader also rejected the conversion's `secret_rite` keys. The installed 1.20 executable contains `secret_faith` and its related effects, with no `secret_rite` key. Secret affiliation must keep its original faith field and handle.

## Correction

The 1.20 migration now restores the 229 missing owner handles from existing campaign title links. Camp/estate IDs, buildings, construction queues, provisions, herd, and province data remain unchanged. Already valid reciprocal owner links remain untouched. Multiple owners, conflicting owner handles, and title references to missing domiciles fail before output publication.

The nine unowned records become `none` tombstones in the same database slots. Their unused buildings/resources are removed, and each change is recorded as `orphan-domicile-tombstone` with a visible report warning. No campaign characters, families, or titles are deleted. Every retained `secret_faith` field stays byte-identical, with its handle checked against the migrated faith database.

The new candidate is generated from the original 1.19.0.6 Barbara save and the original 1.20.0.3 Murchad reference. It does not patch the failed candidate in place. See [validation](validation.md) for automated checks and [migration limits](migration-1.20.md) for remaining work.

## Retest status

The dump identifies the immediate failed write; automated checks verify the repaired ownership graph. The user's subsequent v0.4.1 test simulates without crashes, and preserved debug logs show advancement through 1283.9.12, at least 33 days, followed by a normal exit. A save/reload checkpoint remains pending. Old accolade fields, temporary-opinion `converging` blocks, removed definitions, and other loader errors remain separate migration work; see [the log cleanup review](error-log-cleanup-1.20.md).
