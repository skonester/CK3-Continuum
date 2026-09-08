# Migration roadmap

The desktop inspector and experimental regional converter are implemented. v0.3.1 has a confirmed 85-day engine simulation and saved checkpoint. The next gates are exit/reload and extended gameplay, including succession; see [validation](validation.md).

| Milestone | State | Acceptance gate |
| --- | --- | --- |
| Native inspection + desktop | Implemented prototype | Synthetic and real-save inspection, UI/export tests, packaged Windows smoke |
| Byte-preserving writer | Experimental; engine gate pending | Exact untouched-span preservation, metadata synchronization, validated no-op output that loads/saves/reloads in CK3 |
| Narrow schema migration | Three experimental structural rule families | Isolated tests for religion/faith naming, title names, trait indices, required fields |
| World and ID reconciliation | Experimental regional initialization; 3,036 military references verified after simulation | Correct changed-map mappings, namespace-safe references, coherent imported world at the source date |
| Mods and system preservation | Research | Explicit source dependencies, generated cultures, administrative state, portraits, history |
| Engine validation | Partial: load, corrected titles/poses, 85-day simulation and save passed | Load, simulate, succession, travel, save/reload, and extended stability tests |

## Research constraints

The original research pair has a modded 1.16.1 campaign and an unmodded 1.19.0.6 reference. Different campaigns do not define a complete schema diff. Findings include additional province/title definitions, 89 old static title keys absent from target definitions, shifted trait indices, religion/faith field changes, and changed portrait/system data. These findings are research context, not runtime diagnostics applied to every save.

Do not copy the reference world's numeric IDs into the source. Do not discard generated cultures, dead characters, or histories merely because they differ from a fresh game. Unknown source mods need explicit investigation.

## Writer design

Use immutable decompressed bytes and parser-derived byte spans. Record each operation's rule, entity/path, expected old bytes, replacement, and display summary. Reject overlaps, stale spans, ambiguous duplicate selection, and invalid fragments. Copy unchanged ranges in a single pass.

Rebuild the supported envelope explicitly, synchronize inner/outer metadata where required, update byte lengths and ZIP offsets, preserve required archive members, and verify the result before committing under a new name.

Repeat exports from one immutable source plus the complete plan, or re-index a committed output before further edits. This avoids the repeated-save regression observed in the reviewed Python editor.

## Engine gate

A changed version label, valid CRC, balanced syntax, or successful parser run is insufficient. Converted campaigns must load to the map, continue time, preserve the player/family/world, pass succession and administrative transitions, save and reload repeatedly, and survive longer simulation. Keep a clean modern control to distinguish writer errors and existing engine issues.
