//! 1.19 -> 1.20 schema migration. Reference IDs never become campaign IDs.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const NONE: &str = "4294967295";
mod cleanup;
mod definitions;

fn at(data: &[u8], root: &[Entry], path: &str) -> Result<Entry> {
    let mut list = root.to_vec();
    let mut result = None;
    for name in path.split('/') {
        let e = required(data, &list, name.as_bytes())?.clone();
        list = if e.block {
            children(data, &e)?
        } else {
            Vec::new()
        };
        result = Some(e);
    }
    result.ok_or_else(|| fail("empty migration path"))
}

fn database(data: &[u8], root: &[Entry], path: &str) -> Result<BTreeMap<String, Entry>> {
    let mut db = BTreeMap::new();
    for e in children(data, &at(data, root, path)?)? {
        let id = std::str::from_utf8(key(data, &e)).map_err(fail)?.to_owned();
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
            return Err(fail(format!("non-numeric database ID at {path}")));
        }
        if db.insert(id, e).is_some() {
            return Err(fail(format!("duplicate database ID at {path}")));
        }
    }
    Ok(db)
}

fn identities(
    data: &[u8],
    db: &BTreeMap<String, Entry>,
    field: &[u8],
) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for (id, e) in db.iter().filter(|(_, e)| e.block) {
        let fields = children(data, e)?;
        if let Some(value) = unique(data, &fields, field)? {
            let value = scalar(data, value)?;
            if !value.is_empty() && result.insert(value.into(), id.clone()).is_some() {
                return Err(fail(format!("duplicate stable identity: {value}")));
            }
        }
    }
    Ok(result)
}

fn field_text(data: &[u8], field: &Entry) -> String {
    String::from_utf8_lossy(&data[field.key_start..field.end]).into_owned()
}

fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

// Copy presentation defaults only. Never copy reference rulers, heads, saints,
// variables, history, artifact inventories, or religious/political affiliations.
fn faith_data(
    old: &[u8],
    fields: &[Entry],
    reference: &[u8],
    defaults: &[Entry],
) -> Result<String> {
    let mut result = String::new();
    let mut names = BTreeSet::new();
    let mut tenets = BTreeSet::new();
    for e in fields {
        let name = key(old, e);
        if matches!(
            name,
            b"faith_type" | b"religion" | b"holy_sites" | b"changes" | b"variables"
        ) {
            continue;
        }
        if name == b"doctrine" {
            let value = scalar(old, e)?;
            if value == "tenet_monasticism" {
                result.push_str("doctrine=doctrine_monasticism_accepted\n");
            } else if value.starts_with("tenet_") {
                result.push_str(&format!("tenet={value}\n"));
                tenets.insert(value.to_owned());
            } else {
                result.push_str(&field_text(old, e));
                result.push('\n');
            }
            continue;
        }
        let name = if name == b"doctrine_background_icon" {
            b"tenet_background_icon".as_slice()
        } else {
            name
        };
        if !names.insert(name.to_vec()) {
            return Err(fail("duplicate faith data field"));
        }
        result.push_str(std::str::from_utf8(name).map_err(fail)?);
        result.push('=');
        result.push_str(std::str::from_utf8(&old[e.start..e.end]).map_err(fail)?);
        result.push('\n');
    }
    if !tenets.is_empty() {
        result.push_str("tenets={\n");
        for value in tenets {
            result.push_str(&format!("{{ tenet={value} status=core }}\n"));
        }
        result.push_str("}\n");
    }
    for name in [
        "name",
        "adjective",
        "adherent",
        "adherent_plural",
        "desc",
        "tenet_background_icon",
        "tenet_heretical_background_icon",
        "tenet_neutral_background_icon",
        "tenet_unknown_background_icon",
    ] {
        if !names.contains(name.as_bytes()) {
            if let Some(e) = unique(reference, defaults, name.as_bytes())? {
                result.push_str(&field_text(reference, e));
                result.push('\n');
            } else if matches!(name, "name" | "adjective" | "adherent" | "adherent_plural") {
                let tag = scalar(old, required(old, fields, b"tag")?)?;
                result.push_str(&format!("{name}={}\n", quoted(tag)));
            }
        }
    }
    Ok(result)
}

fn rename_affiliation(
    data: &[u8],
    fields: &[Entry],
    old_name: &[u8],
    new_name: &[u8],
    path: &str,
    faith_ids: &BTreeSet<String>,
    edits: &mut Vec<Patch>,
) -> Result<()> {
    if unique(data, fields, new_name)?.is_some() {
        return Err(fail(format!(
            "mixed legacy and target rite schema at {path}"
        )));
    }
    if let Some(e) = unique(data, fields, old_name)? {
        let id = scalar(data, e)?;
        if id != NONE && !faith_ids.contains(id) {
            return Err(fail(format!("unresolved faith ID {id} at {path}")));
        }
        edits.push(patch(
            e.key_start,
            e.key_end,
            new_name.to_vec(),
            "faith-to-rite",
            format!("{path}/{}", String::from_utf8_lossy(old_name)),
        ));
    }
    Ok(())
}

// 1.20 updates the owner's title when a domicile changes location. An invalid
// owner handle reaches the engine's read-only null title and crashes. Recover
// owners from campaign title -> domicile links, never from the reference IDs.
fn repair_domicile_owners(
    data: &[u8],
    root: &[Entry],
    titles: &BTreeMap<String, Entry>,
    edits: &mut Vec<Patch>,
) -> Result<()> {
    let mut owners = BTreeMap::new();
    for (title_id, title) in titles.iter().filter(|(_, e)| e.block) {
        let fields = children(data, title)?;
        if let Some(field) = unique(data, &fields, b"domicile")? {
            let id = scalar(data, field)?;
            if id != NONE && owners.insert(id.to_owned(), title_id.clone()).is_some() {
                return Err(fail(format!("multiple title owners for domicile {id}")));
            }
        }
    }
    if unique(data, root, b"domiciles")?.is_none() {
        if !owners.is_empty() {
            return Err(fail("title references missing domicile database"));
        }
        return Ok(());
    }
    let domiciles = database(data, root, "domiciles/database")?;
    for id in owners.keys() {
        if !domiciles.get(id).is_some_and(|e| e.block) {
            return Err(fail(format!("title references missing domicile {id}")));
        }
    }
    for (id, domicile) in domiciles.iter().filter(|(_, e)| e.block) {
        let fields = children(data, domicile)?;
        let owner = required(data, &fields, b"owner_title")?;
        let current = scalar(data, owner)?;
        if let Some(title_id) = owners.get(id) {
            if current != NONE && current != title_id {
                return Err(fail(format!("conflicting owner title for domicile {id}")));
            }
            if current == NONE {
                edits.push(patch(
                    owner.start,
                    owner.end,
                    title_id.as_bytes().to_vec(),
                    "domicile-owner-title",
                    format!("domiciles/database/{id}/owner_title"),
                ));
            }
        } else {
            if current != NONE {
                return Err(fail(format!("domicile {id} has no reciprocal title link")));
            }
            // Keep the database slot as a tombstone. The unowned record has no
            // campaign title to restore; inventing an owner would transfer assets.
            edits.push(patch(
                domicile.start,
                domicile.end,
                b"none".to_vec(),
                "orphan-domicile-tombstone",
                format!("domiciles/database/{id}"),
            ));
        }
    }
    Ok(())
}

pub(super) fn plan_upgrade(old: &[u8], reference: &[u8]) -> Result<Vec<Patch>> {
    let root = entries(old, 0, old.len())?;
    let rr = entries(reference, 0, reference.len())?;
    let meta = children(old, required(old, &root, b"meta_data")?)?;
    let version = required(old, &meta, b"version")?;
    if scalar(old, version)? != "1.19.0.6" {
        return Err(fail("1.20 profile requires embedded version 1.19.0.6"));
    }
    if scalar(reference, &at(reference, &rr, "meta_data/version")?)? != "1.20.0.3" {
        return Err(fail("1.20 reference must be version 1.20.0.3"));
    }
    if scalar(
        reference,
        &at(reference, &rr, "meta_data/save_game_version")?,
    )? != "17"
    {
        return Err(fail("1.20 reference must use save_game_version 17"));
    }
    for name in [
        "faiths",
        "rites",
        "organization_manager",
        "clerical_region_manager",
        "house_relations",
        "barter_missions",
        "great_projects",
        "theocracy_lease_manager",
        "title_and_vassal_change_manager",
        "situation_participant_group_manager",
        "situation_sub_region_manager",
    ] {
        if unique(old, &root, name.as_bytes())?.is_some() {
            return Err(fail(format!("mixed legacy and target schema: {name}")));
        }
    }
    let provinces = database(old, &root, "provinces")?;
    let reference_provinces = database(reference, &rr, "provinces")?;
    if provinces.keys().ne(reference_provinces.keys()) {
        return Err(fail(
            "1.20 profile requires matching province IDs; map expansion needs a separate migration",
        ));
    }
    let titles = database(old, &root, "landed_titles/landed_titles")?;
    let title_keys = identities(old, &titles, b"key")?;
    let reference_titles = database(reference, &rr, "landed_titles/landed_titles")?;
    let reference_title_keys = identities(reference, &reference_titles, b"key")?;
    let old_faith_block = at(old, &root, "religion/faiths")?;
    let old_faiths = database(old, &root, "religion/faiths")?;
    let target_faiths = database(reference, &rr, "faiths/database")?;
    let target_rites = database(reference, &rr, "rites/database")?;
    let target_faith_types = identities(reference, &target_faiths, b"faith_type")?;
    let target_rite_types = identities(reference, &target_rites, b"rite_type")?;
    let faith_ids: BTreeSet<_> = old_faiths
        .iter()
        .filter(|(_, e)| e.block)
        .map(|(id, _)| id.clone())
        .collect();
    identities(old, &old_faiths, b"faith_type")?;
    let religions = database(old, &root, "religion/religions")?;
    let religion_ids: BTreeSet<_> = religions
        .iter()
        .filter(|(_, e)| e.block)
        .map(|(id, _)| id.clone())
        .collect();
    let mut edits = vec![patch(
        version.start,
        version.end,
        b"\"1.20.0.3\"".to_vec(),
        "version-label",
        "meta_data/version".into(),
    )];
    let format = required(old, &meta, b"save_game_version")?;
    if !matches!(scalar(old, format)?, "15" | "16") {
        return Err(fail("1.19 source must use save_game_version 15 or 16"));
    }
    edits.push(patch(
        format.start,
        format.end,
        b"17".to_vec(),
        "save-schema-version",
        "meta_data/save_game_version".into(),
    ));
    edits.push(patch(
        old_faith_block.key_start,
        old_faith_block.end,
        Vec::new(),
        "relocate-faith-database",
        "religion/faiths".into(),
    ));
    repair_domicile_owners(old, &root, &titles, &mut edits)?;
    cleanup::plan(old, &root, &mut edits)?;

    let mut faith_text = String::from("\nfaiths={ database={\n");
    let mut rite_text = String::from("\nrites={ database={\n");
    let mut org_text = String::from("\norganization_manager={ database={\n");
    let mut site_faiths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let holy_sites = database(old, &root, "religion/holy_sites")?;
    for (id, e) in &old_faiths {
        if !e.block {
            faith_text.push_str(&format!("{id}={}\n", scalar(old, e)?));
            continue;
        }
        let fields = children(old, e)?;
        for name in [
            b"main_rite".as_slice(),
            b"organization",
            b"tenets",
            b"tenet",
        ] {
            if unique(old, &fields, name)?.is_some() {
                return Err(fail("mixed legacy and target faith schema"));
            }
        }
        let kind = scalar(old, required(old, &fields, b"faith_type")?)?;
        let religion = scalar(old, required(old, &fields, b"religion")?)?;
        if !religion_ids.contains(religion) {
            return Err(fail("unresolved source religion ID"));
        }
        let target_faith = target_faith_types.get(kind).map(|id| &target_faiths[id]);
        let target_fields = target_faith
            .map(|e| children(reference, e))
            .transpose()?
            .unwrap_or_default();
        let target_rite = if let Some(main) = unique(reference, &target_fields, b"main_rite")? {
            Some(
                target_rites
                    .get(scalar(reference, main)?)
                    .filter(|e| e.block)
                    .ok_or_else(|| fail("unresolved reference main rite"))?,
            )
        } else {
            target_rite_types.get(kind).map(|id| &target_rites[id])
        };
        let rite_fields = target_rite
            .map(|e| children(reference, e))
            .transpose()?
            .unwrap_or_default();
        let defaults = if target_faith.is_some() {
            target_fields.clone()
        } else if let Some(data) = unique(reference, &rite_fields, b"data")? {
            children(reference, data)?
        } else {
            Vec::new()
        };
        let data = faith_data(old, &fields, reference, &defaults)?;
        // A historical faith that became a rite remains an independent custom
        // faith here, so campaign faith IDs and runtime references stay valid.
        faith_text.push_str(&format!("{id}={{\n"));
        if target_faith.is_some() {
            faith_text.push_str(&format!("faith_type={}\n", quoted(kind)));
        }
        faith_text.push_str(&format!(
            "religion={religion}\nmain_rite={id}\norganization={id}\n{data}"
        ));
        for name in [b"holy_sites".as_slice(), b"changes", b"variables"] {
            if let Some(field) = unique(old, &fields, name)? {
                faith_text.push_str(&field_text(old, field));
                faith_text.push('\n');
            }
        }
        faith_text.push_str("}\n");
        rite_text.push_str(&format!("{id}={{\n"));
        if let Some(kind) = unique(reference, &rite_fields, b"rite_type")? {
            rite_text.push_str(&field_text(reference, kind));
            rite_text.push('\n');
        }
        let head_title = unique(old, &fields, b"religious_head")?
            .map(|e| scalar(old, e))
            .transpose()?
            .unwrap_or(NONE);
        let head = if head_title == NONE {
            NONE
        } else {
            let title = titles
                .get(head_title)
                .filter(|e| e.block)
                .ok_or_else(|| fail("unresolved source religious-head title"))?;
            let title_fields = children(old, title)?;
            unique(old, &title_fields, b"holder")?
                .map(|e| scalar(old, e))
                .transpose()?
                .unwrap_or(NONE)
        };
        rite_text.push_str(&format!("faith={id}\nfounder={NONE}\nhead_of_rite={head}\norigin_rite={NONE}\ndata={{\n{data}}}\nconvert=yes\nenabled=yes\noverride_name=yes\nsource=historical\n}}\n"));
        org_text.push_str(&format!("{id}={{ faith={id} }}\n"));
        if let Some(sites) = unique(old, &fields, b"holy_sites")? {
            for site in atom_list(old, sites)? {
                if !holy_sites.get(&site).is_some_and(|e| e.block) {
                    return Err(fail("unresolved source holy-site ID"));
                }
                site_faiths.entry(site).or_default().push(id.clone());
            }
        }
    }
    faith_text.push_str("} saints={} }\n");
    rite_text.push_str("} }\n");
    org_text.push_str("} }\n");

    // Resolve target baronies by title key; their numeric IDs differ in the pair.
    let target_sites = database(reference, &rr, "religion/holy_sites")?;
    let target_site_types = identities(reference, &target_sites, b"holy_site_type")?;
    for (id, site) in holy_sites.iter().filter(|(_, e)| e.block) {
        let fields = children(old, site)?;
        let kind = unique(old, &fields, b"holy_site_type")?;
        let legacy = unique(old, &fields, b"holy_site_template")?;
        if kind.is_some() && legacy.is_some() {
            return Err(fail("mixed holy-site identity fields"));
        }
        let identity = scalar(
            old,
            kind.or(legacy)
                .ok_or_else(|| fail("missing holy-site identity"))?,
        )?;
        for name in [b"barony".as_slice(), b"inventory", b"faiths"] {
            if unique(old, &fields, name)?.is_some() {
                return Err(fail("mixed legacy and target holy-site schema"));
            }
        }
        let target = target_site_types.get(identity);
        let campaign_barony = if let Some(target) = target {
            let tf = children(reference, &target_sites[target])?;
            let barony = scalar(reference, required(reference, &tf, b"barony")?)?;
            let title = reference_titles
                .get(barony)
                .filter(|e| e.block)
                .ok_or_else(|| fail("unresolved reference holy-site barony"))?;
            let tfields = children(reference, title)?;
            let title_key = scalar(reference, required(reference, &tfields, b"key")?)?;
            title_keys
                .get(title_key)
                .ok_or_else(|| fail("holy-site barony missing from campaign"))?
                .as_str()
        } else {
            // Removed/mod sites keep their IDs, but no target location is guessed.
            NONE
        };
        let mut replacement = format!(
            "{{ holy_site_type={}\nbarony={campaign_barony}\ninventory={{ holy_site_owner={id} }}\nfaiths={{ {} }}\n",
            quoted(identity),
            site_faiths.get(id).map(|v| v.join(" ")).unwrap_or_default()
        );
        for e in fields
            .iter()
            .filter(|e| !matches!(key(old, e), b"holy_site_type" | b"holy_site_template"))
        {
            replacement.push_str(&field_text(old, e));
            replacement.push('\n');
        }
        replacement.push('}');
        edits.push(patch(
            site.start,
            site.end,
            replacement.into_bytes(),
            if target.is_some() {
                "holy-site-schema"
            } else {
                "unresolved-holy-site"
            },
            format!("religion/holy_sites/{id}/{identity}"),
        ));
    }
    // Verify static title identities without transplanting the reference world.
    if reference_title_keys.is_empty() {
        return Err(fail("reference contains no title identities"));
    }

    for path in [
        "living",
        "dead_unprunable",
        "characters/dead_prunable",
        "characters/unborn",
        "holy_orders/holy_orders",
    ] {
        // Unborn records are a database too; their affiliations may be absent.
        let names: Vec<_> = path.split('/').collect();
        if unique(old, &root, names[0].as_bytes())?.is_none() {
            continue;
        }
        if names.len() > 1 {
            let parent = children(old, required(old, &root, names[0].as_bytes())?)?;
            if unique(old, &parent, names[1].as_bytes())?.is_none() {
                continue;
            }
        }
        for (id, character) in database(old, &root, path)?.iter().filter(|(_, e)| e.block) {
            let fields = children(old, character)?;
            rename_affiliation(
                old,
                &fields,
                b"faith",
                b"rite",
                &format!("{path}/{id}"),
                &faith_ids,
                &mut edits,
            )?;
            // Secret religion still uses a faith handle in 1.20. The engine
            // rejects secret_rite; preserve the source secret_faith field and ID.
            if unique(old, &fields, b"secret_rite")?.is_some() {
                return Err(fail(format!("invalid secret_rite field at {path}/{id}")));
            }
            if let Some(secret) = unique(old, &fields, b"secret_faith")? {
                let faith_id = scalar(old, secret)?;
                if faith_id != NONE && !faith_ids.contains(faith_id) {
                    return Err(fail(format!(
                        "unresolved secret faith ID {faith_id} at {path}/{id}"
                    )));
                }
            }
        }
    }
    for county in children(old, &at(old, &root, "county_manager/counties")?)?
        .iter()
        .filter(|e| e.block)
    {
        let name = String::from_utf8_lossy(key(old, county));
        rename_affiliation(
            old,
            &children(old, county)?,
            b"faith",
            b"rite",
            &format!("county_manager/counties/{name}"),
            &faith_ids,
            &mut edits,
        )?;
    }
    for (id, title) in titles.iter().filter(|(_, e)| e.block) {
        rename_affiliation(
            old,
            &children(old, title)?,
            b"state_faith",
            b"state_rite",
            &format!("landed_titles/landed_titles/{id}"),
            &faith_ids,
            &mut edits,
        )?;
    }
    // Keep trait indices and all XP arrays intact. Only stable-name aliases change.
    let lookup = at(old, &root, "traits_lookup")?;
    let target_lookup: BTreeSet<_> = atom_list(reference, &at(reference, &rr, "traits_lookup")?)?
        .into_iter()
        .collect();
    let mut aliases = BTreeSet::new();
    let mut i = lookup.start + 1;
    while {
        i = whitespace(old, i, lookup.end - 1);
        i < lookup.end - 1
    } {
        let end = atom_end(old, i, lookup.end - 1)?;
        let name = std::str::from_utf8(&old[i..end])
            .map_err(fail)?
            .trim_matches('"');
        let new = match name {
            "scholar" => "erudite",
            "poet" => "lifestyle_poet",
            _ => name,
        };
        if !aliases.insert(new.to_owned()) {
            return Err(fail("duplicate trait identity after alias migration"));
        }
        if new != name {
            if !target_lookup.contains(new) {
                return Err(fail("target trait alias missing from reference"));
            }
            edits.push(patch(
                i,
                end,
                new.as_bytes().to_vec(),
                "trait-name-alias",
                format!("traits_lookup/{name}"),
            ));
        }
        i = end;
    }
    let mut new_roots = faith_text + &rite_text + &org_text;
    for name in [
        "clerical_region_manager",
        "house_relations",
        "barter_missions",
        "great_projects",
        "situation_participant_group_manager",
        "situation_sub_region_manager",
    ] {
        required(reference, &rr, name.as_bytes())?;
        new_roots.push_str(&format!("{name}={{ database={{}} }}\n"));
    }
    required(reference, &rr, b"theocracy_lease_manager")?;
    required(reference, &rr, b"title_and_vassal_change_manager")?;
    new_roots
        .push_str("theocracy_lease_manager={}\ntitle_and_vassal_change_manager={ next_id=1 }\n");
    edits.push(patch(
        old.len(),
        old.len(),
        new_roots.into_bytes(),
        "initialize-1.20-managers",
        "faiths,rites,organization_manager,new-managers".into(),
    ));
    edits.sort_by_key(|e| e.start);
    Ok(edits)
}

fn atom_list(data: &[u8], e: &Entry) -> Result<Vec<String>> {
    if !e.block {
        return Err(fail("expected ID/name list"));
    }
    let mut out = Vec::new();
    let mut i = e.start + 1;
    while {
        i = whitespace(data, i, e.end - 1);
        i < e.end - 1
    } {
        let end = atom_end(data, i, e.end - 1)?;
        out.push(
            std::str::from_utf8(&data[i..end])
                .map_err(fail)?
                .trim_matches('"')
                .into(),
        );
        i = end;
    }
    Ok(out)
}
