//! Scoped R25 successor mappings adapted from ck3-save-updater, copyright (c) 2026 Dustin.
//! Target existence/removal checks are reproduced in scripts/build-compatibility-data.py.
//! SPDX-License-Identifier: GPL-3.0-or-later
use super::compatibility::{self as c, Audit};
use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn table(target: &str) -> Option<&'static str> {
    match target {
        "1.19.0.6" => Some(include_str!("data/content-1.19.0.6.tsv")),
        "1.20.0.3" => Some(include_str!("data/content-1.20.0.3.tsv")),
        _ => None,
    }
}

#[napi(object)]
#[derive(Clone)]
pub struct ContentMigration {
    pub status: String,
    pub mapped: u32,
    pub duplicates_removed: u32,
    pub notes: Vec<String>,
}

struct Occurrence {
    old: String,
    new: String,
    start: usize,
    end: usize,
    path: String,
    duplicate: bool,
}

fn occurrences(data: &[u8], root: &[Entry], target: &str) -> Result<Vec<Occurrence>> {
    let Some(table) = table(target) else {
        return Ok(Vec::new());
    };
    let mut maps: BTreeMap<&str, BTreeMap<&str, &str>> = BTreeMap::new();
    for row in table.lines().filter(|row| !row.starts_with('#')) {
        let fields: Vec<_> = row.split('\t').collect();
        if fields.len() != 4 {
            return Err(fail("invalid bundled content table"));
        }
        maps.entry(fields[0])
            .or_default()
            .insert(fields[1], fields[2]);
    }
    let mut out = Vec::new();
    for (path, kind, field, list) in [
        ("armies/regiments", "regiment", "type", false),
        ("armies/army_regiments", "regiment", "type", false),
        ("artifacts/artifacts", "artifact-type", "type", false),
        (
            "artifacts/artifacts",
            "artifact-modifier",
            "modifiers",
            true,
        ),
    ] {
        let Some(db) = c::at(data, root, path)? else {
            continue;
        };
        for e in children(data, &db)?.iter().filter(|e| e.block) {
            let fields = children(data, e)?;
            if let Some(value) = unique(data, &fields, field.as_bytes())? {
                collect(
                    data,
                    value,
                    maps.get(kind),
                    list,
                    &format!("{path}/{}/{}", String::from_utf8_lossy(key(data, e)), field),
                    &mut out,
                )?;
            }
        }
    }
    // Modifier records only: never replace matching words in names, histories, or script variables.
    for path in c::CHARACTERS.iter().copied().chain([
        "county_manager/counties",
        "landed_titles/landed_titles",
        "titles/landed_titles",
    ]) {
        let Some(db) = c::at(data, root, path)? else {
            continue;
        };
        for e in children(data, &db)?.iter().filter(|e| e.block) {
            let fields = children(data, e)?;
            let entity = format!("{path}/{}", String::from_utf8_lossy(key(data, e)));
            modifier_records(data, &fields, maps.get("modifier"), &entity, &mut out)?;
            if let Some(alive) = unique(data, &fields, b"alive_data")? {
                modifier_records(
                    data,
                    &children(data, alive)?,
                    maps.get("modifier"),
                    &format!("{entity}/alive_data"),
                    &mut out,
                )?;
            }
        }
    }
    Ok(out)
}

fn modifier_records(
    data: &[u8],
    fields: &[Entry],
    map: Option<&BTreeMap<&str, &str>>,
    path: &str,
    out: &mut Vec<Occurrence>,
) -> Result<()> {
    for e in fields
        .iter()
        .filter(|e| key(data, e) == b"modifier" && e.block)
    {
        let records = children(data, e)?;
        if let Some(value) = unique(data, &records, b"modifier")? {
            collect(data, value, map, false, &format!("{path}/modifier"), out)?;
        }
    }
    Ok(())
}

fn collect(
    data: &[u8],
    e: &Entry,
    map: Option<&BTreeMap<&str, &str>>,
    list: bool,
    path: &str,
    out: &mut Vec<Occurrence>,
) -> Result<()> {
    let Some(map) = map else {
        return Ok(());
    };
    if !list && e.block {
        return Err(fail("unexpected block-valued content reference"));
    }
    let values = c::atoms(data, e)?;
    let mut kept: BTreeSet<String> = values.iter().map(|(s, _, _)| s.clone()).collect();
    for (old, start, end) in values {
        if let Some(new) = map.get(old.as_str()) {
            let duplicate = list && !kept.insert((*new).to_owned());
            out.push(Occurrence {
                old,
                new: (*new).into(),
                start,
                end,
                path: path.into(),
                duplicate,
            });
        }
    }
    Ok(())
}

pub(super) fn plan(
    data: &[u8],
    target: &str,
    reference: Option<&[u8]>,
    edits: &mut Vec<Patch>,
) -> Result<ContentMigration> {
    let root = entries(data, 0, data.len())?;
    let mut result = ContentMigration {
        status: "checked".into(),
        mapped: 0,
        duplicates_removed: 0,
        notes: vec![format!(
            "Only curated successors verified against vanilla {target} definitions are applied; names, histories, and script variables are preserved."
        )],
    };
    let reference_modded = if let Some(reference) = reference {
        c::modded(reference, &entries(reference, 0, reference.len())?)?
    } else {
        false
    };
    if c::modded(data, &root)? || reference_modded {
        result.status = "skipped".into();
        result.notes = vec!["An input lists mods. Vanilla successor tables cannot establish whether a mod still defines an old key; removed-content migration was skipped.".into()];
        return Ok(result);
    }
    if table(target).is_none() {
        result.status = "skipped".into();
        result.notes = vec!["No verified removed-content table for this target version.".into()];
        return Ok(result);
    }
    for item in occurrences(data, &root, target)? {
        if item.duplicate {
            result.duplicates_removed += 1;
        } else {
            result.mapped += 1;
        }
        let replacement = if item.duplicate {
            Vec::new()
        } else {
            format!("\"{}\"", item.new).into_bytes()
        };
        edits.push(patch(
            item.start,
            item.end,
            replacement,
            if item.duplicate {
                "removed-content-duplicate"
            } else {
                "removed-content-successor"
            },
            format!("{}: {} -> {}", item.path, item.old, item.new),
        ));
    }
    Ok(result)
}

pub(super) fn diagnose(data: &[u8], root: &[Entry], audit: &mut Audit) -> Result<()> {
    let v = c::version(data, root)?;
    if c::modded(data, root)? {
        audit.report.skipped.push("Removed-content definitions: source lists mods; matching mod definitions are unavailable".into());
        return Ok(());
    }
    if table(&v).is_none() {
        audit.report.skipped.push(format!(
            "Removed-content definitions: no verified table for {v}"
        ));
        return Ok(());
    }
    for item in occurrences(data, root, &v)? {
        audit.add(
            "removed-content",
            "warning",
            "The target vanilla version removed this key; a verified successor is available.",
            format!("{}: {} -> {}", item.path, item.old, item.new),
        );
    }
    audit.report.checked.push(format!(
        "Curated removed-content references for vanilla {v}"
    ));
    audit.report.skipped.push("Content outside the curated successor table, source-mod definitions, and DLC availability: not exhaustively checked".into());
    Ok(())
}
