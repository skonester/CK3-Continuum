# 1.20 error-log cleanup review (2026-10-05)

The v0.4.1 Barbara test now advances without the domicile crash. The preserved debug log shows dates through **1283.9.12**, at least **33 days** after the candidate's 1283.8.10 start, and a normal in-game quit. Save/reload evidence remains pending.

The session started at 05:49:13 America/Chicago. A copy of its logs and grouped error report is preserved under ignored `test-output/logs-1.20-20261005-055214/`. The candidate was compared with the supplied 1.20.0.3 Murchad save and the installed game's definitions. This review records the findings and migration requirements.

The user's sibling `ck3-mod-base` repository is verified as 1.20.0.3 and supplies government groups, obligation ordering, opinion rules, accolade ranks, and compatibility lookups. Nine sampled definition files match the installed game. See [the reference sources and confirmed rules](ck3-1.20-reference.md). Its version history reveals that new obligations shift existing contract positions, so the repair must map obligation names and selected levels, including contracts whose group names are already valid.

## Logging limit

`error.log` contains **100,000 timestamped error records**, plus 60 continuation lines, and ends midway through the accolade database at 05:49:41. The debug/game logs continue through simulation and normal exit at 05:50:56. The earlier crash log also stops at exactly 100,000 error records, but slightly earlier in the same database. This strongly suggests a logging limit: the counts below are recorded errors, and absence of later errors does not establish that later state loaded cleanly.

## Cleanup priorities

| Area | Extent | Evidence and required work |
| --- | ---: | --- |
| Temporary opinions | 88,519 | `opinions/active_opinions` still contains rejected nested `converging` blocks. Modern temporary opinions use flat `modify` and optional `days` fields alongside `value` and dates. Map decay and adjustment state while preserving owners, targets, relationships, current opinion values, and expiration dates. Removing the whole opinion would lose campaign state. |
| Vassal contracts | 3,398 blank records | The campaign has `contract_group=""` in 3,398 of its 4,185 contracts. Empty key-reference errors include these records. Recover contract groups using the subject's government, then remap obligation positions by their historical names. 1.20 inserts rights into existing groups, shifting saved indices even for valid group names. Preserve vassals, lieges, dates, selected levels, rights, and obligations. This may affect taxes and levies; assigning every contract a feudal group would misclassify other governments. |
| Accolades | 5,579 | 1,437 rejected character `active_accolades`/`inactive_accolades` fields and 4,142 rejected database attribute/icon fields are visible before logging stops. All 4,473 campaign accolade records still use the old shape. The reference uses character `accolades` and database `types={ { level=... type=... } }`. Preserve IDs, owners, glory, names, history, and successors; determine attribute levels and inactive-state behavior before migrating. These loader rejections can affect accolade bonuses and ownership lists. |
| Culture exposure | 636 | The loader rejects 199 `exposure_marker`, 200 `exposure_type`, and 237 `none` fields. The modern reference uses `spread_marker` and keeps `culture_marker`. Reconcile the changed exposure/spread system without resetting culture research or fascination. |
| Queued actions | 701 | 638 missing `bp2_parent_guardian_hostage_taker_pulse` and 63 missing `faith_fervor_events_pulse` actions. The installed BP2 script explicitly marks the former deprecated, noting its events are also in the regular yearly pulse. Remove or replace only confirmed obsolete queue entries; retain unrelated scheduled events. |
| Definition changes | Multiple | 157 `scholar_perk` failures: the installed definitions now use `erudite_perk`. 246 `innovation_bombard` failures: the new late-medieval gunpowder definition is `innovation_gunpowder`. Validate aliases and collisions before transferring perk/research state. Removed modifiers, story cycles, regiments, holy sites, and title templates require individual mappings or explicit unresolved-content reporting. |

There are **3,447** timestamped empty-key-reference failures. Sampled parent records include the blank vassal-contract groups, empty game-rule settings, and artifact modifier lists containing empty strings. The candidate has three artifact modifier lists with empty tokens. Clean only the invalid field or reference while preserving each parent record and its valid data. Missing source-mod definitions include legacy titles, regiment types, modifiers, and `segrada_familia`. Startup warnings that occur before the campaign is loaded need a fresh vanilla-session comparison before attributing them to the converter.

`game.log` separately reports unknown dynasty templates falling back to cultural names, 88 duplicate empty title names in `error.log`, and council-task inconsistencies during simulation. They merit fidelity checks even though the session exits normally. Debug-only river sound and GUI-animation messages have lower priority than rejected save data.

## v0.4.2 cleanup implemented

The fresh candidate is generated from the original Barbara/Murchad pair, with all v0.4.1 faith/rite and domicile patches retained. Its 223,703 journal entries include:

| Repair | Campaign extent |
| --- | ---: |
| Opinion decay and flat adjustment | 88,519 converging records + 15,047 constant opinions |
| Blank contract groups recovered | 3,398 |
| Contract obligation positions remapped | 2,208 |
| Malformed republic layouts defaulted | 1,741; 16 contain nonzero discarded selections |
| Accolade database types | 4,473, with 8,946 obsolete icon fields removed |
| Character accolade owner lists | 1,390; 58 include former inactive records |
| Culture spread markers | 199, with 437 obsolete fields removed |
| Innovation/perk aliases | 247 innovation references; 157 scholar perks |
| Removed queued on-actions | 701 |
| Invalid empty references | 43 character modifiers, 3 artifact tokens, 10 game-rule tokens across the two serialized copies |

Original cached opinion values, dates, relationships, and punishments are retained. Modern `days` receives only explicit old `total_days`; the old elapsed counter is not a duration. Fractional original adjustments are retained. Known contract selections are mapped by obligation and level names. The two-slot administrative layout reveals older state surviving in this 1.19-labelled campaign.

The five-slot blank republic layouts cannot describe the target republic group's single fixed obligation. v0.4.2 restores that group from the subject's government and uses its fixed default, with an explicit `contract-republic-default` warning. It does not infer a tributary relationship merely from the five-slot length. Nonzero malformed selections in 16 records are discarded; their original state remains available in the untouched source save.

Accolade attribute choices and campaign records are retained, but the redesigned system requires policy decisions: old glory ranks are allocated alternately across the two acquired attributes, each with levels 1–3, and former inactive IDs stay in their owners' lists with existing assignments intact. All list entries match their database owner. Of 64 former inactive IDs across 58 owners, 53 are vacant and 11 already reference a knight. Bonuses may differ; previously inactive knights may reactivate, and vacant records may become eligible for automatic succession. No new knight is chosen by the converter. See [the implementation and limits](migration-1.20.md).

Independent checks verify all character IDs and unaffected family/DNA/history fields, 107,538 relationship/opinion records, 103,566 temporary opinions, 4,185 contract records, 4,473 accolades, 280 cultures, and all owned domiciles. The original hashes, every changed/unchanged span, metadata, ZIP CRC, and archive offsets pass. The Windows package passes desktop conversion and portable startup checks. These are automated preservation checks; they do not establish engine behavior.

## Next validation gate

Load v0.4.2, simulate, and inspect a fresh error log. The schema cleanup should expose errors previously hidden by the apparent logging limit; error-count reductions remain unmeasured until that test. Individually reconcile removed modifiers, regiment types (`teutonic_knights`, `chu_ko_nu`), house-feud stories, unknown title/dynasty templates, and unmatched holy sites. They are retained rather than assigned guessed replacements. A save/exit/reload checkpoint, religious-system validation, succession, and extended simulation remain pending.
