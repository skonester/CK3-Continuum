# Compatibility checks, removed content, and portraits

These additions are included in the **CK3 Continuum v0.3.4** local test build. They extend the existing native writer and report, with no Python or game-installation requirement at runtime. They have not been validated by loading their output in CK3.

Open a campaign and select **Check campaign** below the workspace content. This reads the save without writing it. Conversion automatically checks both the source and candidate; its report includes `compatibilityBefore`, `compatibilityAfter`, `contentMigration`, and `portraitMigration`. Exporting an inspection report also includes the last completed source audit.

## Audit scope

- Duplicate IDs and low-24-bit database slots within indexed namespaces. Living, dead-unprunable, and dead-prunable characters share one namespace. Identical repeats within `dead_unprunable`, which the game can write, are accepted; repeats across sections are not.
- Explicit character/county faith or rite handles, secret faiths, faith/religion/rite/organization links, and religious-head titles.
- Duplicate title keys, explicit title references, and cycles in de facto liege chains. `capital_barony=yes/no` is a classification flag, not a title handle.
- Required vassal-contract parties and their character references; selected family, culture, house, and holder links. Missing affiliations are reported for living characters in the newer rite schema without inventing a religion. Legacy faith can be implicit, and historical dead characters often omit it legitimately.
- Curated removed-content references for exact supported vanilla versions, plus DNA length and source-template indices for known vanilla layouts.

Findings include severity, count, and at most eight examples per check/severity. Missing sections and unavailable definition checks are reported as skipped. A clean report covers these checks only: it does not validate the full CK3 schema, all reciprocal links, DLC availability, every mod definition, or gameplay. A conversion that introduces a new error identified by the audit is refused before creating its output. Existing source errors remain in the report; writer-control copies preserve the original data.

## Removed-content policy

The tables contain **49** curated mappings for **1.19.0.6** and **56** for **1.20.0.3**. Mappings are selected from ck3-save-updater's R25 lineage research and independently checked against each exact target snapshot: the old key must be absent and its successor present in the relevant definition family.

Examples include `teutonic_knights` to `order_serjeants` and `chu_ko_nu` to `shenbigong` for 1.20.0.3, artifact scheme-power modifiers to corresponding phase-duration modifiers, the medium animal-hide artifact look, corrected Hawk of Quraysh modifier names, and the Rice Fields/Agricultural Innovation modifier. Target balance applies, including changed holy-order infantry toughness. Approximate artifact-bonus combinations and stronger tax-relief replacements were excluded from this first implementation.

Edits target regiment `type`, artifact `type`/modifier lists, and supported character/county modifier records. Names, descriptions, histories, script variables, ownership, strength, and unrelated content stay untouched. If an artifact list already contains the successor, its obsolete duplicate reference is removed. Every replacement/removal appears in the byte-span journal. Content outside the curated table remains for further investigation.

If either source or reference lists mods, these new vanilla mappings are skipped: a mod may still define an old key. Existing migration rules retain their prior policies. Arbitrary mod support and user-provided definition overlays are not implemented.

## Portrait policy

DNA is four bytes per stored gene: dominant template/value and recessive template/value. Genes and templates are matched by identity, retaining allele values and color bytes. Non-stored special genes are excluded. Tables use file order followed by declaration order, not category grouping. Equal DNA lengths do not establish equal layouts.

Exact source tables cover 1.16.1, 1.16.2.3, 1.17.0.1, 1.17.1, 1.18.0.1, 1.18.0.2, 1.18.1, 1.18.1.1, 1.18.2, 1.18.3.1, 1.18.4, and 1.19.0.6. Target tables cover 1.19.0.6 and 1.20.0.3. Each has 104 stored genes. Other point versions can still use structural conversion, but portrait migration is deferred until their exact layouts are established.

A reference from the target version with verified vanilla portrait schema **5** is required, including for the otherwise reference-optional structural profile. Neither input may list mods. If any saved DNA is malformed, has an unexpected size, or uses an undefined source template, **all** portrait edits and the global `portraits_version` change are deferred. Modded DNA is never interpreted as vanilla merely because its size fits. A 1.19.0.6 save with an older/missing portrait schema is also deferred: an earlier converter may have changed its game label while retaining older DNA.

Successful migration updates both serialized metadata copies through the existing writer. The report counts examined, changed, and retained DNA records. Any dropped gene, defaulted new gene, or missing target-template fallback is named in its notes. Actual 1.16-to-1.19 changes include shifted complexion and teeth indices. The 1.19-to-1.20 gene/template tables are identical; eligible faces can remain byte-identical after verification.

CFP/EPE and other historical portrait-mod layouts remain unsupported in this first pass. Rendered appearance still requires visual checks in CK3; valid DNA alone cannot prove identical appearance.

## Provenance and reproduction

Adapted upstream: [ck3-save-updater](https://github.com/djdarcy/ck3-save-updater), local version `0.5.33_main_185-20261117-071b6076`, GPL-3.0-or-later, copyright (c) 2026 Dustin. See [third-party notices](../THIRD_PARTY_NOTICES.md) and the preserved [upstream license](../native/CK3-SAVE-UPDATER-LICENSE.txt). Continuum retains its parser, immutable spans, transaction, and journal verification.

Historical `ck3-mod-base` target commits: `3b2b243d957a12a5ec38d30295c5edfc12269cdf` for 1.19.0.6 and `0ec13350cde9e410b37a46d3616966838f4ba994` for 1.20.0.3. The moving checkout contains 1.20.1.2 and was not substituted for either target. Each generated table records its version and commit. Only factual identifiers/indices are exported, not game scripts, geometry, or gene behavior.

```powershell
python scripts/build-compatibility-data.py --definitions-repo ../ck3-mod-base
npm run native:build
npm test
cargo test --manifest-path native/Cargo.toml --lib --locked
npm run build
npm run test:desktop
```

Keep the original campaign. For engine validation, load a candidate in its exact target version, inspect the player/family/world and portraits, advance time, save, exit, and reload. Compare error logs and test succession before treating it as a continuation.
