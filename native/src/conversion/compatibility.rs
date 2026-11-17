//! Scoped compatibility checks inspired by ck3-save-updater's lint.py/idschema.py.
//! Copyright (c) 2026 CK3 Continuum contributors; upstream copyright (c) 2026 Dustin.
//! SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const NONE: &str = "4294967295";
pub(super) const CHARACTERS: &[&str] = &["living", "dead_unprunable", "characters/dead_prunable"];

#[napi(object)]
#[derive(Clone)]
pub struct CompatibilityFinding {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub count: u32,
    pub examples: Vec<String>,
}

#[napi(object)]
#[derive(Clone)]
pub struct CompatibilityReport {
    pub gamestate_sha256: String,
    pub error_count: u32,
    pub warning_count: u32,
    pub findings: Vec<CompatibilityFinding>,
    pub checked: Vec<String>,
    pub skipped: Vec<String>,
    pub engine_tested: bool,
}

pub(super) struct Audit {
    pub report: CompatibilityReport,
    pub errors: BTreeMap<(String, String), u32>,
}

impl Audit {
    pub fn new(data: &[u8]) -> Self {
        Self {
            report: CompatibilityReport {
                gamestate_sha256: hash(data),
                error_count: 0,
                warning_count: 0,
                findings: Vec::new(),
                checked: Vec::new(),
                skipped: Vec::new(),
                engine_tested: false,
            },
            errors: BTreeMap::new(),
        }
    }
    pub fn add(&mut self, code: &str, severity: &str, message: &str, path: String) {
        if severity == "error" {
            self.report.error_count += 1;
            *self.errors.entry((code.into(), path.clone())).or_default() += 1;
        } else {
            self.report.warning_count += 1;
        }
        if let Some(f) = self
            .report
            .findings
            .iter_mut()
            .find(|f| f.code == code && f.severity == severity)
        {
            f.count += 1;
            if f.examples.len() < 8 {
                f.examples.push(path);
            }
        } else {
            self.report.findings.push(CompatibilityFinding {
                code: code.into(),
                severity: severity.into(),
                message: message.into(),
                count: 1,
                examples: vec![path],
            });
        }
    }
}

pub(super) fn at(data: &[u8], root: &[Entry], path: &str) -> Result<Option<Entry>> {
    let mut fields = root.to_vec();
    let mut found = None;
    for part in path.split('/') {
        let Some(e) = unique(data, &fields, part.as_bytes())? else {
            return Ok(None);
        };
        found = Some(e.clone());
        fields = if e.block {
            children(data, e)?
        } else {
            Vec::new()
        };
    }
    Ok(found)
}

pub(super) fn atoms(data: &[u8], e: &Entry) -> Result<Vec<(String, usize, usize)>> {
    if !e.block {
        return Ok(vec![(scalar(data, e)?.into(), e.start, e.end)]);
    }
    let mut pos = e.start + 1;
    let mut values = Vec::new();
    while {
        pos = whitespace(data, pos, e.end - 1);
        pos < e.end - 1
    } {
        let end = value_end(data, pos, e.end - 1)?;
        if data[pos] == b'{' || data[whitespace(data, end, e.end - 1)..e.end - 1].starts_with(b"=")
        {
            return Err(fail("expected scalar list in compatibility field"));
        }
        let text = std::str::from_utf8(&data[pos..end])
            .map_err(fail)?
            .trim_matches('"')
            .to_owned();
        values.push((text, pos, end));
        pos = end;
    }
    Ok(values)
}

pub(super) fn version(data: &[u8], root: &[Entry]) -> Result<String> {
    Ok(at(data, root, "meta_data/version")?
        .map(|e| scalar(data, &e).map(str::to_owned))
        .transpose()?
        .unwrap_or_default())
}

pub(super) fn modded(data: &[u8], root: &[Entry]) -> Result<bool> {
    for path in ["meta_data/mods", "meta_data/mod_names", "mods"] {
        if let Some(e) = at(data, root, path)? {
            if e.block {
                if whitespace(data, e.start + 1, e.end - 1) < e.end - 1 {
                    return Ok(true);
                }
            } else if !scalar(data, &e)?.is_empty() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

type Database = BTreeMap<String, Entry>;
fn database(
    data: &[u8],
    root: &[Entry],
    paths: &[&str],
    space: &str,
    audit: &mut Audit,
) -> Result<Database> {
    let mut db: Database = BTreeMap::new();
    let mut slots = BTreeMap::new();
    let mut locations = BTreeMap::new();
    let mut present = false;
    for path in paths {
        let Some(section) = at(data, root, path)? else {
            continue;
        };
        present = true;
        for entry in children(data, &section)?.into_iter().filter(|e| e.block) {
            let id = std::str::from_utf8(key(data, &entry))
                .map_err(fail)?
                .trim_matches('"')
                .to_owned();
            let Ok(number) = id.parse::<u32>() else {
                continue;
            };
            if id == NONE {
                continue;
            }
            if let Some(previous) = db.get(&id) {
                // The target game can repeat identical dead_unprunable records.
                if *path != "dead_unprunable"
                    || locations.get(&id) != Some(path)
                    || data[previous.start..previous.end] != data[entry.start..entry.end]
                {
                    audit.add(
                        "duplicate-id",
                        "error",
                        "Two records use the same ID in one namespace.",
                        format!("{space}/{id}"),
                    );
                }
                continue;
            }
            let slot = number & 0x00ff_ffff;
            if let Some(previous) = slots.insert(slot, id.clone()) {
                audit.add(
                    "duplicate-slot",
                    "error",
                    "Different generations occupy the same database slot.",
                    format!("{space}/{previous},{id}"),
                );
            }
            locations.insert(id.clone(), *path);
            db.insert(id, entry);
        }
    }
    if present {
        audit
            .report
            .checked
            .push(format!("{space} IDs and 24-bit slots"));
    } else {
        audit
            .report
            .skipped
            .push(format!("{space} namespace: section absent"));
    }
    Ok(db)
}

fn link(
    data: &[u8],
    fields: &[Entry],
    name: &str,
    db: &Database,
    audit: &mut Audit,
    path: &str,
    severity: &str,
) -> Result<()> {
    if let Some(e) = unique(data, fields, name.as_bytes())? {
        let id = scalar(data, e)?;
        // In title records capital_barony is also a yes/no classification flag.
        if name == "capital_barony" && matches!(id, "yes" | "no") {
            return Ok(());
        }
        if id != NONE && id != "none" && !db.contains_key(id) {
            audit.add(
                "dangling-reference",
                severity,
                "A saved reference points to a record that does not exist.",
                format!("{path}/{name}={id}"),
            );
        }
    }
    Ok(())
}

fn cycles(parents: &BTreeMap<String, String>, audit: &mut Audit) {
    let mut finished = BTreeSet::new();
    for start in parents.keys() {
        let mut chain = Vec::new();
        let mut seen = BTreeMap::new();
        let mut current = start;
        while !finished.contains(current) {
            if let Some(&i) = seen.get(current) {
                for id in &chain[i..] {
                    audit.add(
                        "title-liege-cycle",
                        "error",
                        "A title's liege chain loops instead of ending.",
                        format!("title/{id}/de_facto_liege"),
                    );
                }
                break;
            }
            seen.insert(current.clone(), chain.len());
            chain.push(current.clone());
            let Some(parent) = parents.get(current) else {
                break;
            };
            current = parent;
        }
        finished.extend(chain);
    }
}

fn run(data: &[u8]) -> Result<Audit> {
    let root = entries(data, 0, data.len())?;
    let mut a = Audit::new(data);
    let chars = database(data, &root, CHARACTERS, "character", &mut a)?;
    let titles = database(
        data,
        &root,
        &["landed_titles/landed_titles"],
        "title",
        &mut a,
    )?;
    let faiths = database(
        data,
        &root,
        &["religion/faiths", "faiths/database"],
        "faith",
        &mut a,
    )?;
    let rites = database(data, &root, &["rites/database"], "rite", &mut a)?;
    let religions = database(data, &root, &["religion/religions"], "religion", &mut a)?;
    let cultures = database(
        data,
        &root,
        &["culture_manager/cultures"],
        "culture",
        &mut a,
    )?;
    let houses = database(data, &root, &["dynasties/dynasty_house"], "house", &mut a)?;
    let contracts = database(
        data,
        &root,
        &["vassal_contracts/active", "vassal_contracts/database"],
        "contract",
        &mut a,
    )?;
    let organizations = database(
        data,
        &root,
        &["organization_manager/database"],
        "organization",
        &mut a,
    )?;
    for (space, path) in [
        ("dynasty", "dynasties/dynasties"),
        ("artifact", "artifacts/artifacts"),
        ("regiment", "armies/regiments"),
        ("army_regiment", "armies/army_regiments"),
        ("domicile", "domicile_manager/domiciles"),
        ("accolade", "accolades/database"),
    ] {
        database(data, &root, &[path], space, &mut a)?;
    }
    let modern = at(data, &root, "rites")?.is_some() || version(data, &root)? == "1.20.0.3";
    let living = at(data, &root, "living")?;
    for (id, e) in &chars {
        let fields = children(data, e)?;
        let path = format!("character/{id}");
        let name = if modern { "rite" } else { "faith" };
        // Vanilla 1.19 legitimately leaves faith implicit on many living people.
        // Missing explicit affiliation is a finding only for the newer rite schema.
        if modern
            && living
                .as_ref()
                .is_some_and(|b| e.start > b.start && e.end < b.end)
            && unique(data, &fields, name.as_bytes())?.is_none()
        {
            a.add(
                "missing-affiliation",
                "warning",
                "A character has no explicit religious affiliation; none is guessed.",
                path.clone(),
            );
        }
        link(
            data,
            &fields,
            name,
            if modern { &rites } else { &faiths },
            &mut a,
            &path,
            "error",
        )?;
        link(
            data,
            &fields,
            "secret_faith",
            &faiths,
            &mut a,
            &path,
            "error",
        )?;
        if !cultures.is_empty() {
            link(
                data, &fields, "culture", &cultures, &mut a, &path, "warning",
            )?;
        }
        if !houses.is_empty() {
            link(
                data,
                &fields,
                "dynasty_house",
                &houses,
                &mut a,
                &path,
                "warning",
            )?;
        }
        if let Some(family) = unique(data, &fields, b"family_data")? {
            for f in children(data, family)? {
                if matches!(
                    key(data, &f),
                    b"father"
                        | b"mother"
                        | b"real_father"
                        | b"spouse"
                        | b"primary_spouse"
                        | b"child"
                        | b"former_spouses"
                ) {
                    for (value, _, _) in atoms(data, &f)? {
                        if value != NONE && !chars.contains_key(&value) {
                            a.add(
                                "family-reference",
                                "warning",
                                "A family link points to an absent character.",
                                format!(
                                    "{path}/family_data/{}={value}",
                                    String::from_utf8_lossy(key(data, &f))
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
    let mut identities = BTreeMap::new();
    let mut parents = BTreeMap::new();
    for (id, e) in &titles {
        let fields = children(data, e)?;
        let path = format!("title/{id}");
        if let Some(k) = unique(data, &fields, b"key")? {
            let value = scalar(data, k)?;
            if let Some(previous) = identities.insert(value.to_owned(), id) {
                a.add(
                    "duplicate-title-key",
                    "error",
                    "Two titles share one stable title key.",
                    format!("{previous},{id}: {value}"),
                );
            }
        }
        for field in ["de_jure_liege", "de_facto_liege", "capital_barony"] {
            link(data, &fields, field, &titles, &mut a, &path, "error")?;
        }
        if !chars.is_empty() {
            link(data, &fields, "holder", &chars, &mut a, &path, "warning")?;
        }
        if let Some(liege) = unique(data, &fields, b"de_facto_liege")? {
            let value = scalar(data, liege)?;
            if value != NONE {
                parents.insert(id.clone(), value.into());
            }
        }
    }
    cycles(&parents, &mut a);
    for (id, e) in &contracts {
        let fields = children(data, e)?;
        for field in ["vassal", "liege"] {
            if unique(data, &fields, field.as_bytes())?.is_none() {
                a.add(
                    "missing-contract-party",
                    "error",
                    "A vassal contract lacks a party.",
                    format!("contract/{id}/{field}"),
                );
            } else if !chars.is_empty() {
                link(
                    data,
                    &fields,
                    field,
                    &chars,
                    &mut a,
                    &format!("contract/{id}"),
                    "error",
                )?;
            }
        }
    }
    for (id, e) in &faiths {
        let fields = children(data, e)?;
        let path = format!("faith/{id}");
        link(
            data, &fields, "religion", &religions, &mut a, &path, "error",
        )?;
        if modern {
            link(data, &fields, "main_rite", &rites, &mut a, &path, "error")?;
            link(
                data,
                &fields,
                "organization",
                &organizations,
                &mut a,
                &path,
                "error",
            )?;
        }
        link(
            data,
            &fields,
            "religious_head",
            &titles,
            &mut a,
            &path,
            "error",
        )?;
    }
    for (id, e) in &rites {
        let fields = children(data, e)?;
        link(
            data,
            &fields,
            "faith",
            &faiths,
            &mut a,
            &format!("rite/{id}"),
            "error",
        )?;
        if !chars.is_empty() {
            link(
                data,
                &fields,
                "head_of_rite",
                &chars,
                &mut a,
                &format!("rite/{id}"),
                "warning",
            )?;
        }
    }
    if let Some(counties) = at(data, &root, "county_manager/counties")? {
        for county in children(data, &counties)?.iter().filter(|e| e.block) {
            let fields = children(data, county)?;
            let path = format!("county/{}", String::from_utf8_lossy(key(data, county)));
            link(
                data,
                &fields,
                if modern { "rite" } else { "faith" },
                if modern { &rites } else { &faiths },
                &mut a,
                &path,
                "error",
            )?;
        }
    }
    a.report.checked.push(
        "Explicit affiliations, title hierarchies, contract parties, and family links".into(),
    );
    super::content::diagnose(data, &root, &mut a)?;
    super::portraits::diagnose(data, &root, &mut a)?;
    a.report
        .skipped
        .push("CK3 load, simulation, succession, and save/reload: require game testing".into());
    Ok(a)
}

pub(super) fn audit(data: &[u8]) -> Result<Audit> {
    match run(data) {
        Ok(a) => Ok(a),
        Err(error) => {
            let mut a = Audit::new(data);
            a.add(
                "ambiguous-schema",
                "error",
                "A scoped compatibility check could not select an unambiguous field.",
                error.to_string(),
            );
            a.report.skipped.push(
                "Remaining compatibility checks: schema ambiguity must be resolved first".into(),
            );
            Ok(a)
        }
    }
}

pub struct LintTask {
    input: String,
    expected: String,
}
#[napi]
impl Task for LintTask {
    type Output = CompatibilityReport;
    type JsValue = CompatibilityReport;
    fn compute(&mut self) -> Result<Self::Output> {
        let mut raw = Vec::new();
        fs::File::open(&self.input)
            .map_err(fail)?
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut raw)
            .map_err(fail)?;
        if raw.len() > LIMIT {
            return Err(fail("compatibility input exceeds 512 MiB"));
        }
        let (_, _, game) = read_game(&raw)?;
        if hash(&game) != self.expected {
            return Err(fail("source changed since inspection; inspect it again"));
        }
        Ok(audit(&game)?.report)
    }
    fn resolve(&mut self, _env: Env, report: Self::Output) -> Result<Self::JsValue> {
        Ok(report)
    }
}
#[napi]
pub fn lint_save(input: String, expected: String) -> AsyncTask<LintTask> {
    AsyncTask::new(LintTask { input, expected })
}
