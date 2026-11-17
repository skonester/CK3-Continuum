//! Gene/template translation adapted from ck3-save-updater's dna.py (R19).
//! Copyright (c) 2026 Dustin and CK3 Continuum contributors. GPL-3.0-or-later.
//! A matching byte length is never used to infer the source layout.
use super::compatibility::{self as c, Audit};
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::collections::{BTreeMap, BTreeSet};

struct Gene {
    name: String,
    color: bool,
    templates: BTreeMap<u8, String>,
}
struct Layout {
    genes: Vec<Gene>,
}

fn layout(version: &str) -> Result<Option<Layout>> {
    let text = match version {
        "1.16.1" => include_str!("data/portraits-1.16.1.tsv"),
        "1.16.2.3" => include_str!("data/portraits-1.16.2.3.tsv"),
        "1.17.0.1" => include_str!("data/portraits-1.17.0.1.tsv"),
        "1.17.1" => include_str!("data/portraits-1.17.1.tsv"),
        "1.18.0.1" => include_str!("data/portraits-1.18.0.1.tsv"),
        "1.18.0.2" => include_str!("data/portraits-1.18.0.2.tsv"),
        "1.18.1" => include_str!("data/portraits-1.18.1.tsv"),
        "1.18.1.1" => include_str!("data/portraits-1.18.1.1.tsv"),
        "1.18.2" => include_str!("data/portraits-1.18.2.tsv"),
        "1.18.3.1" => include_str!("data/portraits-1.18.3.1.tsv"),
        "1.18.4" => include_str!("data/portraits-1.18.4.tsv"),
        "1.19.0.6" => include_str!("data/portraits-1.19.0.6.tsv"),
        "1.20.0.3" => include_str!("data/portraits-1.20.0.3.tsv"),
        _ => return Ok(None),
    };
    let mut genes: Vec<Gene> = Vec::new();
    let mut names = BTreeSet::new();
    for row in text.lines().filter(|s| !s.starts_with('#')) {
        let parts: Vec<_> = row.split('\t').collect();
        if parts.len() != 3 {
            return Err(fail("invalid bundled portrait table"));
        }
        match parts[0] {
            "gene" => {
                if !names.insert(parts[2]) {
                    return Err(fail("duplicate bundled portrait gene"));
                }
                genes.push(Gene {
                    name: parts[2].into(),
                    color: parts[1] == "color_genes",
                    templates: BTreeMap::new(),
                });
            }
            "template" => {
                let index = parts[1].parse::<u8>().map_err(fail)?;
                let gene = genes
                    .last_mut()
                    .ok_or_else(|| fail("portrait template without gene"))?;
                if gene.templates.insert(index, parts[2].into()).is_some() {
                    return Err(fail("duplicate bundled template index"));
                }
            }
            _ => return Err(fail("invalid bundled portrait row")),
        }
    }
    Ok(Some(Layout { genes }))
}

fn decode(text: &str, layout: &Layout) -> std::result::Result<Vec<u8>, &'static str> {
    let raw = STANDARD
        .decode(text)
        .map_err(|_| "DNA is not canonical base64")?;
    if raw.len() != layout.genes.len() * 4 {
        return Err("DNA length does not match the versioned source layout");
    }
    for (i, gene) in layout.genes.iter().enumerate().filter(|(_, g)| !g.color) {
        if !gene.templates.contains_key(&raw[i * 4])
            || !gene.templates.contains_key(&raw[i * 4 + 2])
        {
            return Err(
                "DNA references an undefined source template; the source layout may differ",
            );
        }
    }
    Ok(raw)
}

fn translate(
    raw: &[u8],
    source: &Layout,
    target: &Layout,
    losses: &mut BTreeSet<String>,
) -> Vec<u8> {
    let source_genes: BTreeMap<_, _> = source
        .genes
        .iter()
        .enumerate()
        .map(|(i, g)| (g.name.as_str(), (i, g)))
        .collect();
    let target_names: BTreeSet<_> = target.genes.iter().map(|g| g.name.as_str()).collect();
    for g in &source.genes {
        if !target_names.contains(g.name.as_str()) {
            losses.insert(format!("Dropped gene: {}", g.name));
        }
    }
    let mut out = Vec::with_capacity(target.genes.len() * 4);
    for gene in &target.genes {
        let Some((i, old)) = source_genes.get(gene.name.as_str()) else {
            out.extend_from_slice(&[0, 128, 0, 128]);
            losses.insert(format!("Defaulted new gene: {}", gene.name));
            continue;
        };
        let mut value = [raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2], raw[i * 4 + 3]];
        if !gene.color {
            for allele in [0, 2] {
                let name = &old.templates[&value[allele]];
                value[allele] = gene
                    .templates
                    .iter()
                    .find(|(_, n)| *n == name)
                    .map(|(i, _)| *i)
                    .unwrap_or_else(|| {
                        losses.insert(format!(
                            "Defaulted missing template: {}/{}",
                            gene.name, name
                        ));
                        0
                    });
            }
        }
        out.extend_from_slice(&value);
    }
    out
}

#[napi(object)]
#[derive(Clone)]
pub struct PortraitMigration {
    pub status: String,
    pub examined: u32,
    pub changed: u32,
    pub preserved: u32,
    pub source_version: String,
    pub target_version: String,
    pub notes: Vec<String>,
}

pub(super) fn plan(
    data: &[u8],
    target: &str,
    reference: Option<&[u8]>,
    edits: &mut Vec<Patch>,
) -> Result<PortraitMigration> {
    let root = entries(data, 0, data.len())?;
    let source_version = c::version(data, &root)?;
    let mut report = PortraitMigration {
        status: "skipped".into(),
        examined: 0,
        changed: 0,
        preserved: 0,
        source_version: source_version.clone(),
        target_version: target.into(),
        notes: Vec::new(),
    };
    if c::modded(data, &root)? {
        report.notes.push("The source lists mods. Its old portrait layout is not established; DNA and portraits_version are preserved.".into());
        return Ok(report);
    }
    // Earlier converters could change the game label while retaining old DNA.
    // The bundled vanilla 1.19.0.6 reference establishes portrait schema 5.
    if source_version == "1.19.0.6"
        && c::at(data, &root, "meta_data/portraits_version")?
            .map(|e| scalar(data, &e).map(|v| v != "5"))
            .transpose()?
            .unwrap_or(true)
    {
        report.notes.push("The 1.19.0.6 label has an older or missing portrait schema. The original DNA layout may predate that label; DNA and portraits_version are preserved.".into());
        return Ok(report);
    }
    let (Some(source), Some(target_layout)) = (layout(&source_version)?, layout(target)?) else {
        report.notes.push(
            "No exact versioned source/target gene table; DNA and portraits_version are preserved."
                .into(),
        );
        return Ok(report);
    };
    if let Some(reference) = reference {
        let roots = entries(reference, 0, reference.len())?;
        if c::modded(reference, &roots)? {
            report.notes.push("The reference lists mods; target portrait layout is not established. DNA and portraits_version are preserved.".into());
            return Ok(report);
        }
    }
    let mut candidates = Vec::new();
    let mut invalid = Vec::new();
    let mut losses = BTreeSet::new();
    for path in c::CHARACTERS {
        let Some(section) = c::at(data, &root, path)? else {
            continue;
        };
        for record in children(data, &section)?.iter().filter(|e| e.block) {
            let fields = children(data, record)?;
            if let Some(dna) = unique(data, &fields, b"dna")? {
                report.examined += 1;
                let identity = format!("{path}/{}", String::from_utf8_lossy(key(data, record)));
                match decode(scalar(data, dna)?, &source) {
                    Ok(raw) => {
                        let converted = translate(&raw, &source, &target_layout, &mut losses);
                        if converted != raw {
                            candidates.push(patch(
                                dna.start,
                                dna.end,
                                format!("\"{}\"", STANDARD.encode(converted)).into_bytes(),
                                "portrait-dna-layout",
                                identity,
                            ));
                        }
                    }
                    Err(why) => {
                        if invalid.len() < 8 {
                            invalid.push(format!("{identity}: {why}"));
                        }
                    }
                }
            }
        }
    }
    if !invalid.is_empty() {
        report.preserved = report.examined;
        report.notes.push("At least one DNA record does not fit the exact source layout. The entire portrait migration was deferred to avoid mixed layouts or a misleading global portrait version.".into());
        report.notes.extend(invalid);
        return Ok(report);
    }
    let target_portraits = if let Some(reference) = reference {
        let roots = entries(reference, 0, reference.len())?;
        c::at(reference, &roots, "meta_data/portraits_version")?
            .map(|e| scalar(reference, &e).map(str::to_owned))
            .transpose()?
    } else {
        None
    };
    let Some(target_portraits) = target_portraits else {
        report.preserved = report.examined;
        report.notes.push("A reference with portraits_version is required; DNA and the source portrait version were preserved.".into());
        return Ok(report);
    };
    target_portraits
        .parse::<u32>()
        .map_err(|_| fail("invalid reference portraits_version"))?;
    if target_portraits != "5" {
        report.preserved = report.examined;
        report.notes.push("The reference portrait schema differs from the verified vanilla schema 5. DNA and portraits_version are preserved.".into());
        return Ok(report);
    }
    let Some(old_portraits) = c::at(data, &root, "meta_data/portraits_version")? else {
        report.preserved = report.examined;
        report
            .notes
            .push("The source has no portraits_version; portrait migration was deferred.".into());
        return Ok(report);
    };
    report.changed = candidates.len() as u32;
    report.preserved = report.examined - report.changed;
    if scalar(data, &old_portraits)? != target_portraits {
        candidates.push(patch(
            old_portraits.start,
            old_portraits.end,
            target_portraits.into_bytes(),
            "portrait-schema-version",
            "meta_data/portraits_version".into(),
        ));
    }
    edits.extend(candidates);
    report.status = "checked".into();
    report.notes.push("Vanilla layouts were selected by exact save version. Genes/templates are mapped by identity; allele values and color bytes are retained. Special genes are not stored in DNA.".into());
    report.notes.extend(losses);
    Ok(report)
}

pub(super) fn diagnose(data: &[u8], root: &[Entry], audit: &mut Audit) -> Result<()> {
    if c::modded(data, root)? {
        audit.report.skipped.push(
            "Portrait DNA: source lists mods; the historical mod layout is unavailable".into(),
        );
        return Ok(());
    }
    let version = c::version(data, root)?;
    let Some(layout) = layout(&version)? else {
        audit
            .report
            .skipped
            .push(format!("Portrait DNA: no exact table for {version}"));
        return Ok(());
    };
    for path in c::CHARACTERS {
        let Some(section) = c::at(data, root, path)? else {
            continue;
        };
        for record in children(data, &section)?.iter().filter(|e| e.block) {
            if let Some(dna) = unique(data, &children(data, record)?, b"dna")?
                && let Err(why) = decode(scalar(data, dna)?, &layout)
            {
                audit.add(
                    "portrait-layout",
                    "warning",
                    "DNA does not fit the known vanilla layout for this version.",
                    format!(
                        "{path}/{}: {why}",
                        String::from_utf8_lossy(key(data, record))
                    ),
                );
            }
        }
    }
    audit.report.checked.push(format!(
        "DNA length and template indices for vanilla {version}"
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remaps_both_alleles_by_name_and_preserves_values_and_colors() {
        let gene = |name: &str, templates: &[(u8, &str)]| Gene {
            name: name.into(),
            color: false,
            templates: templates.iter().map(|(i, n)| (*i, (*n).into())).collect(),
        };
        let source = Layout {
            genes: vec![
                gene("shape", &[(0, "small"), (1, "large")]),
                Gene {
                    name: "color".into(),
                    color: true,
                    templates: BTreeMap::new(),
                },
            ],
        };
        let target = Layout {
            genes: vec![
                Gene {
                    name: "color".into(),
                    color: true,
                    templates: BTreeMap::new(),
                },
                gene("shape", &[(0, "large"), (1, "small")]),
                gene("new", &[(0, "default")]),
            ],
        };
        let mut losses = BTreeSet::new();
        assert_eq!(
            translate(
                &[1, 73, 0, 152, 9, 25, 42, 250],
                &source,
                &target,
                &mut losses
            ),
            [9, 25, 42, 250, 0, 73, 1, 152, 0, 128, 0, 128]
        );
        assert!(losses.contains("Defaulted new gene: new"));
    }
    #[test]
    fn shipped_layouts_have_no_duplicate_indices_or_unknown_categories() {
        for v in [
            "1.16.1", "1.16.2.3", "1.17.0.1", "1.17.1", "1.18.0.1", "1.18.0.2", "1.18.1",
            "1.18.1.1", "1.18.2", "1.18.3.1", "1.18.4", "1.19.0.6", "1.20.0.3",
        ] {
            let l = layout(v).unwrap().unwrap();
            assert_eq!(l.genes.len(), 104);
            assert!(
                l.genes
                    .iter()
                    .filter(|g| !g.color)
                    .all(|g| g.templates.contains_key(&0))
            );
        }
    }
}
