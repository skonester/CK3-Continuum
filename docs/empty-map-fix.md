# Empty-map failure and ZIP-offset correction

The user loaded the v0.2.0 experimental save and reported a blank world dated 1 January, 1. Captured debug.log reports initialization with zero living characters. Logs were preserved before further testing in the local incident directory; they also contain missing mod-rule references, which remain separate unresolved dependencies.

The writer used whole-file addresses in two ZIP fields. Both original CK3 saves instead use addresses relative to the embedded ZIP after the SAV header and outer metadata.

For the failed experimental file:

| Field | v0.2.0 | v0.2.1 |
|---|---:|---:|
| Embedded ZIP begins | 22,902 | 22,902 |
| Local-header offset stored in central directory | 22,902 | 0 |
| Central-directory offset stored in EOCD | 24,822,523 | 24,799,621 |

Only four bytes differ between the failed and repaired file. All differences are inside the two 32-bit offset fields. The header, metadata, compressed payload, CRC, and migration edits are otherwise identical. The same defect affected both writer-control files; both are regenerated.

General-purpose ZIP readers accepted the earlier files by accommodating prepended data. Thus decompression, CRC, and gamestate equality checks passed without checking the engine's observed embedded-archive convention. The new regression reads directory offsets strictly from the ZIP origin, with no prefix adjustment. It failed against the old native binary and passes against the repaired one.

The exact engine consequence is inferred from the malformed addressing and zero-character load; the corrected file still needs a user CK3 retest. No successful engine load or full migration is claimed. Map, mods, traits, portraits, and other migration limitations are unchanged.

v0.2.1 verification: 17 tests pass, Svelte diagnostics are clean, packaged desktop conversion/cancellation checks pass, and independent real-save checks confirm source hashes, metadata, all changed/untouched ranges, and strict ZIP addressing. Corrected saves use new filenames; previous files were retained.
