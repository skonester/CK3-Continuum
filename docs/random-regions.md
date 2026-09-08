# Current version: v0.3.1

The user confirmed v0.3.0 corrected titles and poses, but it crashed on the first simulation tick. [Crash diagnosis and fix](first-tick-crash.md). New holdings now receive fresh regiments and valid new vassal-contract groups.

The user reported a successful v0.3.1 simulation and save on 2026-09-08. The preserved `regions-v0_3_1-retest_ck3.ck3` independently confirms **85 days of advancement, 1243.5.10 to 1243.8.3**. All 2,517 generated characters remain living; all 839 generated county owners are unchanged; all 3,036 new holding military references resolve to the correct province. No new county has a missing/unheld liege title, and no new political links lead into the original titles. Exit/reload, succession, and longer simulation remain untested. See [validation](validation.md) for political changes observed during that checkpoint.

# Random regional kingdoms: v0.3.0

Use `experimental-random-regions-1.16.1-to-1.19.0.6` with the original 1.16.1 campaign and a 1.19.0.6 reference save. This is now the desktop default. The older structural-only profile remains a diagnostic control.

## What changed

The user supplied an unmodded target-engine re-save, `converter-bug-repro.ck3`. It contains Yamashiro held by character 33606963 (the existing Holy Roman Emperor), with an incorrect de jure reference to title 94. It does not contain `h_china`, `e_japan`, `k_chrysanthemum_throne`, or `k_yongson_throne`. The preceding converter never initialized these regions.

The new profile first performs the existing scoped religion/name migrations, then registers missing territorial titles by stable key and allocates fresh runtime IDs. Source titles retain their owners, actual lieges, claims, heirs, and history. New de jure children can be appended to existing parent lists. Existing counties and province records are preserved.

New geographic counties receive fresh families, grouped under new regional kingdom titles. Each county has a new ruler, spouse, adult heir, house, and dynasty. County vassals have explicit title lieges and reciprocal contract records on their new kings. Reference character IDs, relationships, rulers, title histories, wars, estates, armies, and claims are not imported. New baronies within new counties belong to those counties' rulers. Named reference dynasty seats and mercenary companies are excluded; county-tier names alone do not establish geography.

Randomization is deterministically seeded by the source gamestate hash. Names come from sex/culture-matched reference name pools; ages, skills, and traits are generated. IDs allocate above occupied slots, including dead characters, deleted character handles, and historical lookup references. Repeating a conversion of the same pair produces the same save.

Missing cultures, faiths, religions, and holy sites receive separate IDs with typed mappings. Existing culture records, including generated cultures, are preserved. The `template_cultures` lookup is reconstructed in target reference order, translating each entry back into the output's culture namespace. Imported cultural defaults exclude the reference campaign's heads, acceptance state, parent relationships, variables, and progress markers. Existing faith membership lists gain only newly initialized faiths.

New provinces use the reference's baseline holdings and buildings. Existing province records remain intact. Tagged colors such as `color=rgb { ... }` are handled as complete values rather than losing their color block.

## Portrait finding

Darya (66148) retains her spouse task, chronicler job, family, and original character data in the reproduction save. The installed `gfx/portraits/portrait_animations/animations.txt` idle emperor rule compares `primary_title` against `title:h_china` and checks the Japanese/Korean throne titles. Those titles are missing in the reproduction. A missing-title comparison against an unlanded character is the suspected source of the emperor prop pose. Explicitly registering those titles addresses that condition. **The user confirmed the poses are corrected in v0.3.0.** No existing DNA, portrait version, marriage, or job is changed to hide the symptom.

## Scope of the test

The generated realms currently use **feudal regional kingdoms with county vassals**. Special eastern governments, estates, imperial situations, and other new managers are not initialized. New counties use the reference's development/building/cultural-innovation baseline (1066 for the supplied reference), not a simulation of their history through 1243. Barony ownership is initially grouped under the county rulers. Renames, splits, removed definitions, and changed existing geography still need reconciliation. Some new counties whose de jure kingdom already existed remain independent to avoid changing the old kingdom's owner or attaching them to an old ruler.

This is an experimental recovery profile with confirmed portrait/title correction and an 85-day saved simulation checkpoint. Exit/reload, succession, longer stability, and the political changes noted in validation remain open checks.

## Validation

The original source is never overwritten. The conversion journal records exact byte edits, source/output/reference hashes, and the profile. Writer controls remain available. Native regression tests exercise colliding source/reference IDs, deleted/historical character ID allocation, mixed color/list syntax, family preservation, kingdom/vassal relationships, culture lookup order, reference dynasty-seat exclusion, deterministic output, and missing/ambiguous reference rejection.

The separate Python audit in `../analysis/audit_random_regions.py` verifies the real output's source character bytes, original title owners/history/claims, existing cultures/counties/provinces, new domain/family/contract reciprocity, isolated political links, every journal span/hash, ZIP CRC, metadata copies, and archive-relative directory offsets. Engine validation is recorded separately.
