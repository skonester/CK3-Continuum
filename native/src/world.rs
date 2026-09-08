//! Fresh regional populations. Reference records supply geography and cultural
//! defaults only; no reference character, political owner, history, or ID survives.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
type Index = BTreeMap<String, Entry>;
fn index(d: &[u8], list: Vec<Entry>) -> Result<Index> {
    let mut out = Index::new();
    for e in list {
        let k = String::from_utf8_lossy(key(d, &e)).into_owned();
        out.insert(k, e);
    }
    Ok(out)
}
fn fields(d: &[u8], e: &Entry) -> Result<Index> {
    index(d, children(d, e)?)
}
fn node<'a>(db: &'a Index, k: &str) -> Result<&'a Entry> {
    db.get(k)
        .ok_or_else(|| fail(format!("missing world field {k}")))
}
fn at(d: &[u8], path: &str) -> Result<Entry> {
    let mut list = entries(d, 0, d.len())?;
    let mut result = None;
    for k in path.split('/') {
        let e = required(d, &list, k.as_bytes())?.clone();
        list = if e.block {
            children(d, &e)?
        } else {
            Vec::new()
        };
        result = Some(e);
    }
    result.ok_or_else(|| fail("empty world path"))
}
fn db(d: &[u8], path: &str) -> Result<Index> {
    let records = children(d, &at(d, path)?)?;
    let count = records.len();
    let out = index(d, records)?;
    if out.len() != count {
        return Err(fail(format!("duplicate database record at {path}")));
    }
    Ok(out)
}
fn val(d: &[u8], f: &Index, k: &str) -> Result<String> {
    Ok(scalar(d, node(f, k)?)?.into())
}
fn opt(d: &[u8], f: &Index, k: &str) -> Option<String> {
    f.get(k).and_then(|e| scalar(d, e).ok()).map(str::to_owned)
}
fn ids(d: &[u8], e: &Entry) -> Result<Vec<String>> {
    if !e.block {
        return Err(fail("expected ID list"));
    }
    let raw = std::str::from_utf8(&d[e.start + 1..e.end - 1]).map_err(fail)?;
    raw.split_whitespace()
        .map(|s| {
            if s.bytes().all(|b| b.is_ascii_digit()) {
                Ok(s.into())
            } else {
                Err(fail("unsupported ID list"))
            }
        })
        .collect()
}
fn list(v: impl IntoIterator<Item = String>) -> String {
    format!("{{ {} }}", v.into_iter().collect::<Vec<_>>().join(" "))
}
fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
fn append(
    edits: &mut Vec<Patch>,
    d: &[u8],
    path: &str,
    bytes: String,
    rule: &'static str,
) -> Result<()> {
    if !bytes.is_empty() {
        let e = at(d, path)?;
        edits.push(patch(
            e.end - 1,
            e.end - 1,
            bytes.into_bytes(),
            rule,
            path.into(),
        ));
    }
    Ok(())
}
fn replace(edits: &mut Vec<Patch>, e: &Entry, bytes: String, rule: &'static str, path: String) {
    edits.push(patch(e.start, e.end, bytes.into_bytes(), rule, path));
}
fn copy_fields(d: &[u8], e: &Entry, allow: &[&str]) -> Result<String> {
    let mut out = String::new();
    for f in children(d, e)? {
        if allow.iter().any(|k| key(d, &f) == k.as_bytes()) {
            out.push_str(std::str::from_utf8(&d[f.key_start..f.end]).map_err(fail)?);
            out.push('\n');
        }
    }
    Ok(out)
}
fn next_id<'a>(keys: impl Iterator<Item = &'a String>) -> Result<u32> {
    let max = keys
        .filter_map(|k| k.parse::<u32>().ok())
        .filter(|n| *n != u32::MAX)
        .map(|n| n & 0x00ff_ffff)
        .max()
        .unwrap_or(0);
    if max >= 0x00ff_0000 {
        return Err(fail("world ID allocator exhausted"));
    }
    Ok(max + 1)
}
fn identity_map(
    old: &[u8],
    new: &[u8],
    path: &str,
    old_key: &str,
    new_key: &str,
) -> Result<(BTreeMap<String, String>, BTreeSet<String>)> {
    let a = db(old, path)?;
    let b = db(new, path)?;
    let mut by_name = BTreeMap::new();
    for (id, e) in &a {
        if e.block {
            let f = fields(old, e)?;
            if let Some(k) = opt(old, &f, old_key)
                && by_name.insert(k, id.clone()).is_some()
            {
                return Err(fail("duplicate stable identity in source"));
            }
        }
    }
    let mut next = next_id(a.keys())?;
    let mut map = BTreeMap::new();
    let mut added = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for (id, e) in &b {
        if !e.block {
            continue;
        }
        let f = fields(new, e)?;
        let k = val(new, &f, new_key)?;
        if !seen.insert(k.clone()) {
            return Err(fail("duplicate stable identity in reference"));
        }
        let dest = if let Some(existing) = by_name.get(&k) {
            existing.clone()
        } else {
            let n = next.to_string();
            next += 1;
            added.insert(id.clone());
            n
        };
        map.insert(id.clone(), dest);
    }
    Ok((map, added))
}
fn mapped(map: &BTreeMap<String, String>, id: &str) -> Result<String> {
    map.get(id)
        .cloned()
        .ok_or_else(|| fail(format!("unresolved imported reference {id}")))
}
fn remap_list(d: &[u8], e: &Entry, map: &BTreeMap<String, String>) -> Result<String> {
    Ok(list(
        ids(d, e)?
            .iter()
            .map(|id| mapped(map, id))
            .collect::<Result<Vec<_>>>()?,
    ))
}
fn roll(seed: &[u8], label: &str, n: usize) -> usize {
    let mut h = Sha256::new();
    h.update(seed);
    h.update(label.as_bytes());
    let bytes = h.finalize();
    (u64::from_le_bytes(bytes[..8].try_into().unwrap()) % n as u64) as usize
}
#[derive(Default)]
struct Ruler {
    id: String,
    spouse: String,
    heir: String,
    house: String,
    dynasty: String,
    culture: String,
    faith: String,
    county: String,
    kingdom: Option<String>,
    domain: Vec<String>,
    contracts: Vec<String>,
}

pub(super) fn plan_world(old: &[u8], reference: &[u8]) -> Result<Vec<Patch>> {
    if scalar(reference, &at(reference, "meta_data/version")?)? != "1.19.0.6" {
        return Err(fail("world reference must be version 1.19.0.6"));
    }
    let date = scalar(old, &at(old, "date")?)?.to_owned();
    let year: i32 = date.split('.').next().unwrap_or("").parse().map_err(fail)?;
    if year < 100 {
        return Err(fail("unsupported campaign year for generated families"));
    }
    let seed = Sha256::digest(old);
    let mut edits = Vec::new();
    let (culture_map, new_cultures) = identity_map(
        old,
        reference,
        "culture_manager/cultures",
        "culture_template",
        "culture_template",
    )?;
    let (faith_map, new_faiths) =
        identity_map(old, reference, "religion/faiths", "template", "faith_type")?;
    let (religion_map, new_religions) = identity_map(
        old,
        reference,
        "religion/religions",
        "template",
        "religion_type",
    )?;
    let (holy_map, new_holy) = identity_map(
        old,
        reference,
        "religion/holy_sites",
        "holy_site_type",
        "holy_site_type",
    )?;
    let rc = db(reference, "culture_manager/cultures")?;
    let rf = db(reference, "religion/faiths")?;
    let rr = db(reference, "religion/religions")?;
    let rh = db(reference, "religion/holy_sites")?;
    let mut text = String::new();
    for id in &new_cultures {
        let body = copy_fields(
            reference,
            node(&rc, id)?,
            &[
                "culture_template",
                "culture_era_data",
                "culture_innovation",
                "name",
                "color",
                "collective_noun",
                "prefix",
                "name_order_convention",
                "house_coa_frame",
                "house_coa_mask_scale",
                "house_coa_mask_offset",
                "name_list",
                "ethnicities",
                "building_gfx",
                "clothing_gfx",
                "unit_gfx",
                "coa_gfx",
                "traditions",
                "ethos",
                "heritage",
                "language",
                "martial_custom",
                "head_determination",
            ],
        )?;
        text += &format!(
            "\n{}={{\n{body}created={date}\nculture_reformation={{ start_time=1.1.1 progress=0 }}\n}}\n",
            culture_map[id]
        );
    }
    append(
        &mut edits,
        old,
        "culture_manager/cultures",
        text,
        "new-region-cultures",
    )?;
    // This is a target-definition-ordered lookup, not a list of all culture IDs.
    replace(
        &mut edits,
        &at(old, "culture_manager/template_cultures")?,
        remap_list(
            reference,
            &at(reference, "culture_manager/template_cultures")?,
            &culture_map,
        )?,
        "culture-template-index",
        "culture_manager/template_cultures".into(),
    );
    let mut text = String::new();
    for id in &new_holy {
        text += &format!(
            "\n{}={{ {} }}\n",
            holy_map[id],
            copy_fields(reference, node(&rh, id)?, &["holy_site_type"])?
        );
    }
    append(
        &mut edits,
        old,
        "religion/holy_sites",
        text,
        "new-region-holy-sites",
    )?;
    let mut text = String::new();
    for id in &new_faiths {
        let e = node(&rf, id)?;
        let f = fields(reference, e)?;
        let body = copy_fields(
            reference,
            e,
            &[
                "faith_type",
                "tag",
                "color",
                "icon",
                "texticon",
                "graphical_faith",
                "piety_icon_group",
                "doctrine_background_icon",
                "doctrine",
            ],
        )?;
        let holy = remap_list(reference, node(&f, "holy_sites")?, &holy_map)?;
        text += &format!(
            "\n{}={{\n{body}religion={}\nholy_sites={holy}\nfervor=50\nreligious_head=4294967295\nchanges={{}}\n}}\n",
            faith_map[id],
            mapped(&religion_map, &val(reference, &f, "religion")?)?
        );
    }
    append(
        &mut edits,
        old,
        "religion/faiths",
        text,
        "new-region-faiths",
    )?;
    let mut text = String::new();
    for (id, e) in &rr {
        if !e.block {
            continue;
        }
        let f = fields(reference, e)?;
        if new_religions.contains(id) {
            let body = copy_fields(
                reference,
                e,
                &[
                    "religion_type",
                    "tag",
                    "graphical_faith",
                    "piety_icon_group",
                    "doctrine_background_icon",
                    "family",
                ],
            )?;
            text += &format!(
                "\n{}={{\n{body}faiths={}\n}}\n",
                religion_map[id],
                remap_list(reference, node(&f, "faiths")?, &faith_map)?
            );
        } else {
            let additions = ids(reference, node(&f, "faiths")?)?
                .into_iter()
                .filter(|id| new_faiths.contains(id))
                .map(|id| faith_map[&id].clone())
                .collect::<Vec<_>>();
            if !additions.is_empty() {
                let path = format!("religion/religions/{}/faiths", religion_map[id]);
                let e = at(old, &path)?;
                let mut all = ids(old, &e)?;
                all.extend(additions);
                replace(&mut edits, &e, list(all), "religion-faith-membership", path);
            }
        }
    }
    append(
        &mut edits,
        old,
        "religion/religions",
        text,
        "new-region-religions",
    )?;
    let geographic_counties = db(reference, "county_manager/counties")?;
    let ot = db(old, "landed_titles/landed_titles")?;
    let rt = db(reference, "landed_titles/landed_titles")?;
    let mut old_keys = BTreeMap::new();
    for (id, e) in &ot {
        if e.block {
            let f = fields(old, e)?;
            if let Some(k) = opt(old, &f, "key")
                && old_keys.insert(k, id.clone()).is_some()
            {
                return Err(fail("duplicate source title key"));
            }
        }
    }
    let mut title_map = BTreeMap::new();
    let mut new_titles = BTreeSet::new();
    let mut title_keys = BTreeMap::new();
    let mut title_fields = BTreeMap::new();
    let mut next_title = next_id(ot.keys())?;
    for (id, e) in &rt {
        if !e.block {
            continue;
        }
        let f = fields(reference, e)?;
        let k = val(reference, &f, "key")?;
        if !["b_", "c_", "d_", "k_", "e_", "h_"]
            .iter()
            .any(|p| k.starts_with(p))
        {
            continue;
        }
        if !old_keys.contains_key(&k)
            && [
                "landless",
                "mercenary",
                "holy_order",
                "nomad",
                "noble_family",
                "require_landless",
            ]
            .iter()
            .any(|flag| opt(reference, &f, flag).as_deref() == Some("yes"))
        {
            continue;
        }
        // Named noble-family seats also use c_ keys but are not map counties.
        // Their template flags need not be repeated in the serialized record.
        if !old_keys.contains_key(&k)
            && k.starts_with("c_")
            && !geographic_counties.contains_key(&k)
        {
            continue;
        }
        let dest = if let Some(id) = old_keys.get(&k) {
            id.clone()
        } else {
            let id = next_title.to_string();
            next_title += 1;
            new_titles.insert(id.clone());
            id
        };
        title_map.insert(id.clone(), dest.clone());
        title_keys.insert(dest.clone(), k);
        title_fields.insert(dest, f);
    }
    let oc = db(old, "county_manager/counties")?;
    let counties = db(reference, "county_manager/counties")?;
    let mut parent = BTreeMap::new();
    for (id, f) in &title_fields {
        if let Some(p) = opt(reference, f, "de_jure_liege") {
            parent.insert(id.clone(), mapped(&title_map, &p)?);
        }
    }
    let mut char_keys = BTreeSet::new();
    for path in ["living", "dead_unprunable", "characters/dead_prunable"] {
        char_keys.extend(db(old, path)?.into_keys());
    }
    if let Ok(e) = at(old, "deleted_characters") {
        char_keys.extend(ids(old, &e)?);
    }
    if let Ok(lookup) = at(old, "character_lookup") {
        for e in children(old, &lookup)? {
            let v = scalar(old, &e)?;
            if v.parse::<u32>().is_ok() {
                char_keys.insert(v.into());
            }
        }
    }
    let mut next_char = next_id(char_keys.iter())?;
    let mut next_house = next_id(db(old, "dynasties/dynasty_house")?.keys())?;
    let mut next_dynasty = next_id(db(old, "dynasties/dynasties")?.keys())?;
    let mut rulers: BTreeMap<String, Ruler> = BTreeMap::new();
    let mut kingdom_capital: BTreeMap<String, String> = BTreeMap::new();
    let mut text = String::new();
    for id in &new_titles {
        let k = &title_keys[id];
        if !k.starts_with("c_") || !counties.contains_key(k) {
            continue;
        }
        if oc.contains_key(k) {
            return Err(fail("county exists without its title; ambiguous geography"));
        }
        let f = fields(reference, node(&counties, k)?)?;
        let culture = val(reference, &f, "culture")?;
        let faith = val(reference, &f, "faith")?;
        let mut ancestor = id.clone();
        let mut seen = BTreeSet::new();
        let mut kingdom = None;
        while let Some(p) = parent.get(&ancestor) {
            if !seen.insert(p.clone()) {
                return Err(fail("reference de jure cycle"));
            }
            ancestor = p.clone();
            if title_keys[p].starts_with("k_") {
                if new_titles.contains(p) {
                    kingdom = Some(p.clone());
                }
                break;
            }
        }
        if let Some(k) = &kingdom {
            kingdom_capital.entry(k.clone()).or_insert(id.clone());
        }
        let ruler = Ruler {
            id: next_char.to_string(),
            spouse: (next_char + 1).to_string(),
            heir: (next_char + 2).to_string(),
            house: next_house.to_string(),
            dynasty: next_dynasty.to_string(),
            culture,
            faith,
            county: id.clone(),
            kingdom,
            domain: vec![id.clone()],
            contracts: Vec::new(),
        };
        next_char += 3;
        next_house += 1;
        next_dynasty += 1;
        text += &format!(
            "\n{k}={{\n{}culture={}\nfaith={}\ncounty_control=100\ndevelopment_progress=0\n}}\n",
            copy_fields(reference, node(&counties, k)?, &["development"])?,
            culture_map[&ruler.culture],
            faith_map[&ruler.faith]
        );
        rulers.insert(id.clone(), ruler);
    }
    append(
        &mut edits,
        old,
        "county_manager/counties",
        text,
        "new-region-counties",
    )?;
    if rulers.is_empty() {
        return Err(fail("reference contains no missing counties to initialize"));
    }
    let mut holders = BTreeMap::new();
    for (id, r) in &rulers {
        holders.insert(id.clone(), r.id.clone());
    }
    for (k, c) in &kingdom_capital {
        holders.insert(k.clone(), rulers[c].id.clone());
        rulers.get_mut(c).unwrap().domain.insert(0, k.clone());
    }
    // Every new barony under a new county belongs to that county's fresh ruler.
    // Existing county ownership is never used as a default for a new county.
    for id in &new_titles {
        if title_keys[id].starts_with("b_")
            && let Some(c) = parent.get(id)
            && let Some(r) = rulers.get_mut(c)
        {
            holders.insert(id.clone(), r.id.clone());
            r.domain.push(id.clone());
        }
    }
    let mut next_contract = next_id(db(old, "vassal_contracts/database")?.keys())?;
    let mut contracts = String::new();
    let mut political_parent = BTreeMap::new();
    let county_ids = rulers.keys().cloned().collect::<Vec<_>>();
    for c in &county_ids {
        let r = &rulers[c];
        if let Some(k) = &r.kingdom {
            let cap = &kingdom_capital[k];
            if cap != c {
                let king = &rulers[cap];
                let contract = next_contract.to_string();
                next_contract += 1;
                contracts += &format!(
                    "\n{contract}={{ vassal={} liege={} date={date} liege_dynasty_date={date} levels={{ 11 0=2 1=2 }} contract_group=feudal_vassal }}\n",
                    r.id, king.id
                );
                political_parent.insert(c.clone(), k.clone());
                rulers.get_mut(cap).unwrap().contracts.push(contract);
            }
        }
    }
    append(
        &mut edits,
        old,
        "vassal_contracts/database",
        contracts,
        "new-region-vassal-contracts",
    )?;
    for id in &new_titles {
        if title_keys[id].starts_with("b_") && holders.contains_key(id) {
            political_parent.insert(id.clone(), parent[id].clone());
        }
    }
    let mut new_title_text = String::new();
    let mut extra_children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (refid, dest) in &title_map {
        if !new_titles.contains(dest) {
            continue;
        }
        let f = &title_fields[dest];
        let k = &title_keys[dest];
        let e = node(&rt, refid)?;
        let mut body = format!("key={k}\n");
        if let Some(p) = parent.get(dest) {
            body += &format!("de_jure_liege={p}\n");
            if !new_titles.contains(p) {
                extra_children
                    .entry(p.clone())
                    .or_default()
                    .push(dest.clone());
            }
        }
        let children = parent
            .iter()
            .filter(|(_, p)| *p == dest)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if !children.is_empty() {
            body += &format!("de_jure_vassals={}\n", list(children));
        }
        if let Some(p) = political_parent.get(dest) {
            body += &format!("de_facto_liege={p}\n");
        }
        if let Some(h) = holders.get(dest) {
            body += &format!("holder={h}\ndate={date}\nhistory={{ {date}={h} }}\n");
        } else {
            body += "date=9999.1.1\n";
        }
        if let Some(cap) = kingdom_capital.get(dest) {
            body += &format!("capital={cap}\n");
        } else if let Some(cap) = opt(reference, f, "capital")
            && let Some(id) = title_map.get(&cap)
        {
            body += &format!("capital={id}\n");
        }
        body += &copy_fields(
            reference,
            e,
            &["capital_barony", "duchy_capital_barony", "definite_form"],
        )?;
        if let Some(names) = f.get("title_name_data") {
            body += &format!(
                "title_name_data={{\n{}}}\n",
                copy_fields(
                    reference,
                    names,
                    &["name", "adj", "article", "pre", "localization_key"]
                )?
            );
        }
        if holders.contains_key(dest) && !k.starts_with("b_") {
            body += "laws={ confederate_partition_succession_law male_preference_law }\n";
        }
        body += "treasury={}\nnomad_name_type=default\n";
        new_title_text += &format!("\n{dest}={{\n{body}}}\n");
    }
    append(
        &mut edits,
        old,
        "landed_titles/landed_titles",
        new_title_text,
        "new-region-titles",
    )?;
    for (id, added) in extra_children {
        let path = format!("landed_titles/landed_titles/{id}");
        let e = at(old, &path)?;
        let f = fields(old, &e)?;
        if let Some(v) = f.get("de_jure_vassals") {
            let mut all = ids(old, v)?;
            all.extend(added);
            replace(
                &mut edits,
                v,
                list(all),
                "new-de-jure-children",
                format!("{path}/de_jure_vassals"),
            );
        } else {
            append(
                &mut edits,
                old,
                &path,
                format!("\nde_jure_vassals={}\n", list(added)),
                "new-de-jure-children",
            )?;
        }
    }
    let op = db(old, "provinces")?;
    let rp = db(reference, "provinces")?;
    let original_regiments = db(old, "armies/regiments")?;
    let reference_regiments = db(reference, "armies/regiments")?;
    let mut next_regiment = next_id(original_regiments.keys())?;
    let mut regiment_text = String::new();
    let mut text = String::new();
    for (id, e) in &rp {
        if op.contains_key(id) {
            continue;
        }
        let f = fields(reference, e)?;
        let mut body = copy_fields(reference, e, &["fort_level"])?;
        if let Some(h) = f.get("holding") {
            let holding_fields = fields(reference, h)?;
            let mut holding = copy_fields(
                reference,
                h,
                &[
                    "type",
                    "buildings",
                    "special_building",
                    "special_building_slot",
                    "income",
                    "barter_goods",
                ],
            )?;
            // These are handles into armies/regiments, NOT soldier counts.
            // Allocate fresh unraised regiments; never copy raised-army chunks.
            for kind in ["levy", "garrison"] {
                if let Some(reference_id) = opt(reference, &holding_fields, kind) {
                    let rf = fields(reference, node(&reference_regiments, &reference_id)?)?;
                    if val(reference, &rf, "origin")? != *id {
                        return Err(fail(format!(
                            "reference regiment {reference_id} origin does not match province {id}"
                        )));
                    }
                    let expected_source = if kind == "garrison" {
                        "garrison"
                    } else {
                        "levy"
                    };
                    let source = opt(reference, &rf, "source").unwrap_or_else(|| "levy".into());
                    if source != expected_source {
                        return Err(fail("reference regiment has incompatible source type"));
                    }
                    let max = opt(reference, &rf, "max").unwrap_or_else(|| "0".into());
                    max.parse::<u32>()
                        .map_err(|_| fail("invalid regiment maximum"))?;
                    let fresh = next_regiment.to_string();
                    next_regiment += 1;
                    holding += &format!("{kind}={fresh}\n");
                    let garrison = if kind == "garrison" {
                        format!("source=garrison\nchunks={{ {{ max={max} current=0 }} }}\n")
                    } else {
                        String::new()
                    };
                    regiment_text += &format!("\n{fresh}={{ origin={id} max={max}\n{garrison}}}\n");
                }
            }
            body += &format!("holding={{\n{holding}}}\n");
        }
        text += &format!("\n{id}={{\n{body}}}\n");
    }
    append(&mut edits, old, "provinces", text, "new-region-provinces")?;
    append(
        &mut edits,
        old,
        "armies/regiments",
        regiment_text,
        "new-region-regiments",
    )?;
    let mut pools: BTreeMap<(String, bool), BTreeSet<String>> = BTreeMap::new();
    for path in ["living", "dead_unprunable", "characters/dead_prunable"] {
        for (_, e) in db(reference, path)? {
            if !e.block {
                continue;
            }
            let f = fields(reference, &e)?;
            if let (Some(c), Some(name)) = (
                opt(reference, &f, "culture"),
                opt(reference, &f, "first_name"),
            ) {
                pools
                    .entry((c, opt(reference, &f, "female").as_deref() == Some("yes")))
                    .or_default()
                    .insert(name);
            }
        }
    }
    let traits = ids_or_names(old, &at(old, "traits_lookup")?)?;
    let personality = [
        "brave",
        "calm",
        "diligent",
        "gregarious",
        "just",
        "temperate",
        "patient",
        "ambitious",
        "humble",
        "generous",
        "honest",
    ];
    let mut characters = String::new();
    let mut houses = String::new();
    let mut dynasties = String::new();
    for (county, r) in &rulers {
        let cf = fields(reference, node(&rc, &r.culture)?)?;
        let mut ethnicity = "".to_owned();
        if let Some(e) = cf.get("ethnicities") {
            let values = children(reference, e)?;
            if let Some(first) = values.first() {
                ethnicity = scalar(reference, first)?.into();
            }
        }
        if ethnicity.is_empty() {
            return Err(fail("missing generated character ethnicity"));
        }
        let name_for = |female: bool, label: &str| -> Result<String> {
            let values = pools
                .get(&(r.culture.clone(), female))
                .ok_or_else(|| {
                    fail(format!(
                        "no reference name pool for culture {} female={female}",
                        r.culture
                    ))
                })?
                .iter()
                .collect::<Vec<_>>();
            Ok(quote(values[roll(&seed, label, values.len())]))
        };
        let name = name_for(false, &format!("{county}/ruler"))?;
        let spouse_name = name_for(true, &format!("{county}/spouse"))?;
        let heir_name = name_for(false, &format!("{county}/heir"))?;
        let age = 32 + roll(&seed, &format!("{county}/age"), 19) as i32;
        let house_name = if let Some(n) = title_fields[county].get("title_name_data") {
            let names = fields(reference, n)?;
            val(reference, &names, "name")?
        } else {
            title_keys[county].clone()
        };
        houses += &format!(
            "\n{}={{ name={} found_date={date} head_of_house={} dynasty={} historical={{ {} }} motto=\"\" }}\n",
            r.house,
            quote(&house_name),
            r.id,
            r.dynasty,
            r.id
        );
        dynasties += &format!(
            "\n{}={{ dynasty_head={} prestige={{ currency=0 accumulated=0 }} }}\n",
            r.dynasty, r.id
        );
        let common_for = |role: &str| -> Result<String> {
            let mut chosen = BTreeSet::new();
            let mut round = 0;
            while chosen.len() < 3 {
                let t = personality[roll(
                    &seed,
                    &format!("{county}/{role}/trait/{round}"),
                    personality.len(),
                )];
                if let Some(i) = traits.iter().position(|x| x == t) {
                    chosen.insert(i.to_string());
                } else {
                    return Err(fail(format!("missing trait {t}")));
                }
                round += 1;
            }
            let education = traits
                .iter()
                .position(|x| x == "education_stewardship_2")
                .ok_or_else(|| fail("missing education trait"))?;
            chosen.insert(education.to_string());
            let skills = (0..6)
                .map(|n| (4 + roll(&seed, &format!("{county}/{role}/skill/{n}"), 9)).to_string())
                .collect::<Vec<_>>();
            Ok(format!(
                "culture={} faith={} ethnicity={} skill={} traits={}\n",
                culture_map[&r.culture],
                faith_map[&r.faith],
                ethnicity,
                list(skills),
                list(chosen)
            ))
        };
        let common = common_for("ruler")?;
        let alive = "alive_data={ fertility=0.5 health=5 gold={ value=50 } piety={ currency=0 accumulated=0 } prestige={ currency=100 accumulated=100 } }";
        characters += &format!(
            "\n{}={{ first_name={name} birth={}.1.1 dynasty_house={} {common}family_data={{ primary_spouse={} spouse={} child={{ {} }} }}\n{alive}\nlanded_data={{ domain={} vassal_contracts={} became_ruler_date={date} government=feudal_government realm_capital={} laws={{ crown_authority_0 confederate_partition_succession_law male_preference_law }} succession={{ {} }} }}\nplayable_data={{}}\n}}\n",
            r.id,
            year - age,
            r.house,
            r.spouse,
            r.spouse,
            r.heir,
            list(r.domain.clone()),
            list(r.contracts.clone()),
            r.county,
            r.heir
        );
        let common = common_for("spouse")?;
        characters += &format!(
            "\n{}={{ first_name={spouse_name} birth={}.1.1 female=yes {common}family_data={{ primary_spouse={} spouse={} child={{ {} }} }} {alive} court_data={{ employer={} join_court_date={date} }} }}\n",
            r.spouse,
            year - age + 3,
            r.id,
            r.id,
            r.heir,
            r.id
        );
        let common = common_for("heir")?;
        characters += &format!(
            "\n{}={{ first_name={heir_name} birth={}.1.1 dynasty_house={} {common}family_data={{ father={} mother={} }} {alive} court_data={{ employer={} join_court_date={date} }} }}\n",
            r.heir,
            year - 16,
            r.house,
            r.id,
            r.spouse,
            r.id
        );
    }
    append(
        &mut edits,
        old,
        "living",
        characters,
        "new-region-characters",
    )?;
    append(
        &mut edits,
        old,
        "dynasties/dynasty_house",
        houses,
        "new-region-houses",
    )?;
    append(
        &mut edits,
        old,
        "dynasties/dynasties",
        dynasties,
        "new-region-dynasties",
    )?;
    // Never permit imported political references to resolve to an old ruler.
    for (title, owner) in &holders {
        if !new_titles.contains(title) || char_keys.contains(owner) {
            return Err(fail("new-region owner leaked into the source campaign"));
        }
    }
    for (child, liege) in &political_parent {
        if !new_titles.contains(child) || !new_titles.contains(liege) {
            return Err(fail(
                "new-region political liege leaked into the source campaign",
            ));
        }
    }
    Ok(edits)
}
fn ids_or_names(d: &[u8], e: &Entry) -> Result<Vec<String>> {
    Ok(std::str::from_utf8(&d[e.start + 1..e.end - 1])
        .map_err(fail)?
        .split_whitespace()
        .map(|s| s.trim_matches('"').to_owned())
        .collect())
}
