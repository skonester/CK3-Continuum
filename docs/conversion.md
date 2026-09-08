# Current default: random regional kingdoms (v0.3.1)

See [the new-region profile](random-regions.md) for the implementation, reproduction evidence, and test limits. Select an original 1.16.1 campaign and a 1.19.0.6 reference. The test-candidate script now uses this profile.

The regional profile passed an 85-day CK3 simulation and save checkpoint. Reload and extended gameplay remain untested; see [validation](validation.md).

The structural-only profile below remains available as a diagnostic control.

# Experimental conversion (v0.2.0)

Choose **Migration plan → Create a test save**. Writer control rebuilds the supported container with identical gamestate/metadata and unchanged version. The structural profile requires exactly 1.16.1 and creates a separate 1.19.0.6 candidate.

## Implemented rules

1. Rename `template` at `religion/religions/<numeric ID>` to `religion_type`, and at `religion/faiths/<numeric ID>` to `faith_type`, preserving values and IDs.
2. Move direct scalar `name`, `adj`, `article`, `pre`, and `localization_key` fields at `landed_titles/landed_titles/<numeric ID>` into `title_name_data`. Preserve exact value text. Duplicate fields, conflicting target structures, and block-valued name fields are rejected.
3. Update `meta_data/version` to 1.19.0.6, synchronize metadata copies, and update header lengths. This label marks a candidate, not certified compatibility.

No global replacements or reference-world copying occurs. Player IDs, historical characters, traits/lookup indices, portraits version/DNA, generated cultures, geography, and save-game format version remain unchanged.

## Unresolved work

Changed maps and absent title definitions, trait aliases/XP (including poet), portraits, source mods, administrative/economic changes, new managers, and imported world initialization are unresolved. These omissions can prevent loading or cause incorrect gameplay. These are limitations of this older structural-only profile. The current regional profile has separate, partial engine validation recorded above.

## Output guarantees and limits

The app checks the inspected gamestate hash, whole-source hash during conversion, syntax, metadata agreement, patch boundaries, ZIP CRC/size, and output readback. Each new `.ck3` has a `.ck3.conversion.json` journal recording input/output hashes, byte spans, replacement lengths, and warnings. Existing files cannot be overwritten.

Only UTF-8 SAV kind 00 or unified compressed text kind 02 is accepted. Binary/split formats, extra ZIP entries, comments/ZIP64, conflicting schemas, and mismatched metadata are refused. Conversion input and expanded data are capped at 512 MiB each. A successful inspection does not imply writer support. Publication requires a hard-link-capable filesystem such as NTFS.

## In-game test order

1. Load the unchanged modern original as a control.
2. Load the modern writer-control copy, advance, save, exit, and reload. Failure here points to the writer or pre-existing engine behavior.
3. Test an untouched copy of the old campaign in the exact target build and record any automatic upgrades.
4. Load the experimental candidate. Inspect player/date, realm, holdings, family, culture, faith, portraits, and administrative state.
5. Advance one day, one month, and one year in separate checkpoints; save, exit, and reload.
6. Capture separate logs and inspect succession, travel, warfare, administrative appointments, and new/unresolved regions.

A successful load is not full compatibility. Identify the old mod playset before attributing missing content solely to engine-version changes.

To create the three test files from local inputs:

```powershell
node scripts/create-test-candidates.mjs "C:\path\old.ck3" "C:\path\modern.ck3" "C:\path\test-output"
```
