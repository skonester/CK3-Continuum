//! Legacy fields can survive an earlier Continuum conversion. Migrate by shape,
//! not just the version label. Every edit is scoped to a persisted database path.
use super::definitions::{GROUPS, OBLIGATIONS};
use super::*;

fn remove(e: &Entry, rule: &'static str, path: String) -> Patch {
    patch(e.key_start, e.end, Vec::new(), rule, path)
}

// Strict anonymous-record lists, kept separate from assignment/database indexing.
fn records(data: &[u8], e: &Entry) -> Result<Vec<Entry>> {
    if !e.block {
        return Err(fail("expected anonymous record list"));
    }
    let mut out = Vec::new();
    let mut i = e.start + 1;
    while {
        i = whitespace(data, i, e.end - 1);
        i < e.end - 1
    } {
        if data[i] != b'{' {
            return Err(fail("unexpected token in anonymous record list"));
        }
        let end = value_end(data, i, e.end - 1)?;
        out.push(Entry {
            key_start: i,
            key_end: i,
            start: i,
            end,
            block: true,
        });
        i = end;
    }
    Ok(out)
}

fn opinions(data: &[u8], root: &[Entry], edits: &mut Vec<Patch>) -> Result<()> {
    let Some(manager) = unique(data, root, b"opinions")? else {
        return Ok(());
    };
    let fields = children(data, manager)?;
    let Some(active) = unique(data, &fields, b"active_opinions")? else {
        return Ok(());
    };
    for (i, opinion) in records(data, active)?.iter().enumerate() {
        for (j, temp) in children(data, opinion)?
            .iter()
            .filter(|e| key(data, e) == b"temporary_opinion")
            .enumerate()
        {
            let path = format!("opinions/active_opinions/{i}/temporary_opinion/{j}");
            let fields = children(data, temp)?;
            let value = required(data, &fields, b"value")?;
            let converging = unique(data, &fields, b"converging")?;
            if unique(data, &fields, b"modify")?.is_some() {
                if converging.is_some() {
                    return Err(fail(format!("mixed opinion decay schema at {path}")));
                }
                continue;
            }
            if let Some(conv) = converging {
                if unique(data, &fields, b"days")?.is_some() {
                    return Err(fail("mixed opinion duration schema"));
                }
                let cf = children(data, conv)?;
                if cf
                    .iter()
                    .any(|e| !matches!(key(data, e), b"opinion" | b"days" | b"total_days"))
                {
                    return Err(fail("unsupported converging opinion field"));
                }
                let original = scalar(data, required(data, &cf, b"opinion")?)?;
                let initial = original
                    .parse::<f64>()
                    .map_err(|_| fail("invalid original opinion"))?;
                if !initial.is_finite() {
                    return Err(fail("invalid original opinion"));
                }
                // Old `days` is elapsed time; modern flat `days` is total duration.
                // Monthly-change opinions have no total_days: keep that absence,
                // letting the target modifier use its monthly_change definition.
                if let Some(elapsed) = unique(data, &cf, b"days")? {
                    scalar(data, elapsed)?
                        .parse::<u32>()
                        .map_err(|_| fail("invalid elapsed opinion days"))?;
                }
                let mut text = format!("modify={original}");
                if let Some(total) = unique(data, &cf, b"total_days")? {
                    let days = scalar(data, total)?;
                    days.parse::<u32>()
                        .map_err(|_| fail("invalid opinion duration"))?;
                    text.push_str(&format!(" days={days}"));
                }
                edits.push(patch(
                    conv.key_start,
                    conv.end,
                    text.into_bytes(),
                    "opinion-decay-schema",
                    path,
                ));
            } else {
                edits.push(patch(
                    temp.end - 1,
                    temp.end - 1,
                    format!(" modify={} ", scalar(data, value)?).into_bytes(),
                    "opinion-flat-adjustment",
                    path,
                ));
            }
        }
    }
    Ok(())
}

fn government_group(government: &str) -> Option<&'static str> {
    Some(match government {
        "administrative_government" => "admin_vassal",
        "celestial_government" => "celestial_vassal",
        "clan_government" => "clan_vassal",
        "ecclesiastical_government" | "theocracy_government" => "theocracy_vassal",
        "feudal_government" => "feudal_vassal",
        "herder_government" => "herder_vassal",
        "japan_administrative_government" => "japan_administrative_vassal",
        "japan_feudal_government" => "japan_feudal_vassal",
        "mandala_government" => "mandala_vassal",
        "meritocratic_government" | "steppe_admin_government" => "meritocratic_vassal",
        "nomad_government" => "nomad_vassal",
        "republic_government" => "republic_vassal",
        "tribal_government" => "tribal_vassal",
        "wanua_government" => "wanua_vassal",
        _ => return None,
    })
}

fn levels(data: &[u8], e: &Entry) -> Result<(usize, BTreeMap<usize, usize>)> {
    if !e.block {
        return Err(fail("expected sparse contract levels"));
    }
    let mut i = whitespace(data, e.start + 1, e.end - 1);
    let end = atom_end(data, i, e.end - 1)?;
    let count = std::str::from_utf8(&data[i..end])
        .map_err(fail)?
        .parse::<usize>()
        .map_err(|_| fail("invalid contract level count"))?;
    if count > 128 {
        return Err(fail("unsupported contract level count"));
    }
    let mut result = BTreeMap::new();
    i = end;
    while {
        i = whitespace(data, i, e.end - 1);
        i < e.end - 1
    } {
        let end = atom_end(data, i, e.end - 1)?;
        let index = std::str::from_utf8(&data[i..end])
            .map_err(fail)?
            .parse::<usize>()
            .map_err(|_| fail("invalid contract obligation index"))?;
        i = whitespace(data, end, e.end - 1);
        if data.get(i) != Some(&b'=') {
            return Err(fail("invalid sparse contract assignment"));
        }
        i = whitespace(data, i + 1, e.end - 1);
        let end = atom_end(data, i, e.end - 1)?;
        let level = std::str::from_utf8(&data[i..end])
            .map_err(fail)?
            .parse::<usize>()
            .map_err(|_| fail("invalid contract selection"))?;
        if index >= count || result.insert(index, level).is_some() {
            return Err(fail("duplicate or out-of-range contract obligation"));
        }
        i = end;
    }
    Ok((count, result))
}

fn contracts(data: &[u8], root: &[Entry], edits: &mut Vec<Patch>) -> Result<()> {
    if unique(data, root, b"vassal_contracts")?.is_none() {
        return Ok(());
    }
    let living = database(data, root, "living")?;
    for (id, contract) in database(data, root, "vassal_contracts/database")?
        .iter()
        .filter(|(_, e)| e.block)
    {
        let path = format!("vassal_contracts/database/{id}");
        let fields = children(data, contract)?;
        let group = required(data, &fields, b"contract_group")?;
        let saved = scalar(data, group)?;
        let name = if saved.is_empty() {
            let subject = scalar(data, required(data, &fields, b"vassal")?)?;
            let character = living
                .get(subject)
                .filter(|e| e.block)
                .ok_or_else(|| fail(format!("missing contract subject at {path}")))?;
            let gov = at(data, &children(data, character)?, "landed_data/government")?;
            government_group(scalar(data, &gov)?)
                .ok_or_else(|| fail(format!("unknown subject government at {path}")))?
        } else {
            saved
        };
        let spec = GROUPS
            .iter()
            .find(|g| g.name == name)
            .ok_or_else(|| fail(format!("unknown contract group {name}")))?;
        let lv = required(data, &fields, b"levels")?;
        let (count, selected) = levels(data, lv)?;
        let order = spec.old.iter().find(|o| o.len() == count);
        // This legacy shape cannot represent a modern republic's single fixed
        // obligation. Recover its government group and use that fixed default.
        // Journal the discarded malformed selections explicitly, never pretend
        // that they were transferable taxes/levies or guess a tributary relation.
        let fallback = saved.is_empty() && name == "republic_vassal" && count == 5;
        if order.is_none() && !fallback {
            return Err(fail(format!(
                "unrecognized historical contract shape at {path}"
            )));
        }
        let mut target = BTreeMap::new();
        for (new_index, obligation) in spec.target.iter().enumerate() {
            let def = OBLIGATIONS
                .iter()
                .find(|o| o.name == *obligation)
                .ok_or_else(|| fail("missing obligation definition"))?;
            let value = if let Some(old_index) =
                order.and_then(|o| o.iter().position(|o| o == obligation))
            {
                let level = *selected.get(&old_index).unwrap_or(&0);
                let identity = def.old.get(level).ok_or_else(|| {
                    fail(format!(
                        "invalid old obligation level at {path}/{obligation}"
                    ))
                })?;
                def.target
                    .iter()
                    .position(|l| l == identity)
                    .ok_or_else(|| fail("removed obligation level requires explicit mapping"))?
            } else {
                def.default
            };
            if value != 0 {
                target.insert(new_index, value);
            }
        }
        if saved.is_empty() {
            edits.push(patch(
                group.start,
                group.end,
                name.as_bytes().to_vec(),
                "contract-group-recovery",
                format!("{path}/contract_group"),
            ));
        }
        let text = format!(
            "{{ {}{} }}",
            spec.target.len(),
            target
                .iter()
                .map(|(i, v)| format!(" {i}={v}"))
                .collect::<String>()
        );
        if fallback || count != spec.target.len() || selected != target {
            edits.push(patch(
                lv.start,
                lv.end,
                text.into_bytes(),
                if fallback {
                    "contract-republic-default"
                } else {
                    "contract-obligation-map"
                },
                format!("{path}/levels"),
            ));
        }
    }
    Ok(())
}

fn accolades(data: &[u8], root: &[Entry], edits: &mut Vec<Patch>) -> Result<()> {
    if unique(data, root, b"accolades")?.is_none() {
        return Ok(());
    }
    let db = database(data, root, "accolades/database")?;
    for (id, e) in db.iter().filter(|(_, e)| e.block) {
        let fields = children(data, e)?;
        let primary = unique(data, &fields, b"primary")?;
        let secondary = unique(data, &fields, b"secondary")?;
        if primary.is_none() && secondary.is_none() {
            continue;
        }
        if primary.is_none() || unique(data, &fields, b"types")?.is_some() {
            return Err(fail("mixed accolade type schema"));
        }
        let primary = primary.unwrap();
        let name = scalar(data, primary)?;
        let glory = scalar(data, required(data, &fields, b"glory")?)?
            .parse::<f64>()
            .map_err(|_| fail("invalid accolade glory"))?;
        if !glory.is_finite() || glory < 0.0 {
            return Err(fail("invalid accolade glory"));
        }
        let rank = [100.0, 300.0, 600.0, 1000.0, 1500.0, 2100.0]
            .iter()
            .filter(|&&g| glory >= g)
            .count();
        // Old automatic six ranks become three chosen levels per attribute.
        // Keep both acquired identities even after glory loss, as modern
        // accolades can likewise have spent points exceeding their current rank.
        let mut text = format!(
            "types={{ {{ level={} type={} }}",
            rank.div_ceil(2).clamp(1, 3),
            quoted(name)
        );
        if let Some(second) = secondary {
            let second_name = scalar(data, second)?;
            if !second_name.is_empty() && second_name != name {
                text.push_str(&format!(
                    " {{ level={} type={} }}",
                    (rank / 2).clamp(1, 3),
                    quoted(second_name)
                ));
            }
            edits.push(remove(
                second,
                "accolade-secondary-schema",
                format!("accolades/database/{id}/secondary"),
            ));
        }
        text.push_str(" }");
        edits.push(patch(
            primary.key_start,
            primary.end,
            text.into_bytes(),
            "accolade-type-schema",
            format!("accolades/database/{id}/primary"),
        ));
        for icon in [b"primary_icon".as_slice(), b"secondary_icon"] {
            if let Some(e) = unique(data, &fields, icon)? {
                edits.push(remove(
                    e,
                    "accolade-obsolete-icon",
                    format!("accolades/database/{id}/{}", String::from_utf8_lossy(icon)),
                ));
            }
        }
    }
    for path in ["living", "dead_unprunable", "characters/dead_prunable"] {
        for (id, e) in database(data, root, path)?.iter().filter(|(_, e)| e.block) {
            let fs = children(data, e)?;
            let Some(playable) = unique(data, &fs, b"playable_data")? else {
                continue;
            };
            let pf = children(data, playable)?;
            let active = unique(data, &pf, b"active_accolades")?;
            let inactive = unique(data, &pf, b"inactive_accolades")?;
            if active.is_none() && inactive.is_none() {
                continue;
            }
            if unique(data, &pf, b"accolades")?.is_some() {
                return Err(fail("mixed character accolade schema"));
            }
            let mut ids = Vec::new();
            let mut seen = BTreeSet::new();
            for list in [active, inactive].into_iter().flatten() {
                for aid in atom_list(data, list)? {
                    if !seen.insert(aid.clone()) {
                        return Err(fail("duplicate character accolade reference"));
                    }
                    if !db.get(&aid).is_some_and(|e| e.block) {
                        return Err(fail("dangling character accolade reference"));
                    }
                    ids.push(aid);
                }
            }
            let anchor = active.or(inactive).unwrap();
            edits.push(patch(
                anchor.key_start,
                anchor.end,
                format!("accolades={{ {} }}", ids.join(" ")).into_bytes(),
                if inactive.is_some() {
                    "accolade-owned-list-with-inactive"
                } else {
                    "accolade-owned-list"
                },
                format!("{path}/{id}/playable_data/accolades"),
            ));
            if let (Some(_), Some(inactive)) = (active, inactive) {
                edits.push(remove(
                    inactive,
                    "accolade-inactive-list-schema",
                    format!("{path}/{id}/playable_data/inactive_accolades"),
                ));
            }
        }
    }
    Ok(())
}

fn alias(
    data: &[u8],
    e: &Entry,
    old: &str,
    new: &str,
    rule: &'static str,
    path: String,
    edits: &mut Vec<Patch>,
) -> Result<()> {
    if scalar(data, e)? == old {
        edits.push(patch(e.start, e.end, new.as_bytes().to_vec(), rule, path));
    }
    Ok(())
}

fn culture(data: &[u8], root: &[Entry], edits: &mut Vec<Patch>) -> Result<()> {
    if unique(data, root, b"culture_manager")?.is_none() {
        return Ok(());
    }
    for (id, e) in database(data, root, "culture_manager/cultures")?
        .iter()
        .filter(|(_, e)| e.block)
    {
        let fields = children(data, e)?;
        let path = format!("culture_manager/cultures/{id}");
        if let Some(exposure) = unique(data, &fields, b"exposure_marker")? {
            if unique(data, &fields, b"spread_marker")?.is_some() {
                return Err(fail("mixed culture spread schema"));
            }
            edits.push(patch(
                exposure.key_start,
                exposure.key_end,
                b"spread_marker".to_vec(),
                "culture-spread-schema",
                format!("{path}/exposure_marker"),
            ));
            alias(
                data,
                exposure,
                "innovation_bombard",
                "innovation_gunpowder",
                "innovation-name-alias",
                format!("{path}/spread_marker"),
                edits,
            )?;
        }
        for field in [b"exposure_type".as_slice(), b"none"] {
            if let Some(e) = unique(data, &fields, field)? {
                if field == b"none" && !scalar(data, e)?.is_empty() {
                    return Err(fail("nonempty obsolete culture none field"));
                }
                edits.push(remove(
                    e,
                    "culture-obsolete-field",
                    format!("{path}/{}", String::from_utf8_lossy(field)),
                ));
            }
        }
        if let Some(fascination) = unique(data, &fields, b"fascination_marker")? {
            alias(
                data,
                fascination,
                "innovation_bombard",
                "innovation_gunpowder",
                "innovation-name-alias",
                format!("{path}/fascination_marker"),
                edits,
            )?;
        }
        if let Some(innovations) = unique(data, &fields, b"culture_innovation")? {
            let mut names = BTreeSet::new();
            for (i, record) in records(data, innovations)?.iter().enumerate() {
                let fs = children(data, record)?;
                let kind = required(data, &fs, b"type")?;
                let name = scalar(data, kind)?;
                let target = if name == "innovation_bombard" {
                    "innovation_gunpowder"
                } else {
                    name
                };
                if !names.insert(target.to_owned()) {
                    return Err(fail("innovation alias collision"));
                }
                alias(
                    data,
                    kind,
                    "innovation_bombard",
                    "innovation_gunpowder",
                    "innovation-name-alias",
                    format!("{path}/culture_innovation/{i}/type"),
                    edits,
                )?;
            }
        }
    }
    Ok(())
}

fn queue(data: &[u8], root: &[Entry], edits: &mut Vec<Patch>) -> Result<()> {
    for (i, e) in root
        .iter()
        .filter(|e| key(data, e) == b"triggered_event")
        .enumerate()
    {
        let fs = children(data, e)?;
        if let Some(action) = unique(data, &fs, b"on_action")? {
            let name = scalar(data, action)?;
            if matches!(
                name,
                "bp2_parent_guardian_hostage_taker_pulse" | "faith_fervor_events_pulse"
            ) {
                edits.push(remove(
                    e,
                    "obsolete-queued-on-action",
                    format!("triggered_event/{i}/{name}"),
                ));
            }
        }
    }
    Ok(())
}

fn character_cleanup(data: &[u8], root: &[Entry], edits: &mut Vec<Patch>) -> Result<()> {
    for path in ["living", "dead_unprunable", "characters/dead_prunable"] {
        for (id, e) in database(data, root, path)?.iter().filter(|(_, e)| e.block) {
            let fs = children(data, e)?;
            if let Some(alive) = unique(data, &fs, b"alive_data")? {
                let af = children(data, alive)?;
                if let Some(perks) = unique(data, &af, b"perk")? {
                    let mut names = BTreeSet::new();
                    let mut i = perks.start + 1;
                    if !perks.block {
                        return Err(fail("expected character perk list"));
                    }
                    while {
                        i = whitespace(data, i, perks.end - 1);
                        i < perks.end - 1
                    } {
                        let end = atom_end(data, i, perks.end - 1)?;
                        let name = std::str::from_utf8(&data[i..end])
                            .map_err(fail)?
                            .trim_matches('"');
                        let target = if name == "scholar_perk" {
                            "erudite_perk"
                        } else {
                            name
                        };
                        if !names.insert(target.to_owned()) {
                            return Err(fail("perk alias collision"));
                        }
                        if target != name {
                            edits.push(patch(
                                i,
                                end,
                                target.as_bytes().to_vec(),
                                "perk-name-alias",
                                format!("{path}/{id}/alive_data/perk/{name}"),
                            ));
                        }
                        i = end;
                    }
                }
                empty_character_modifiers(data, &af, &format!("{path}/{id}/alive_data"), edits)?;
            }
            empty_character_modifiers(data, &fs, &format!("{path}/{id}"), edits)?;
        }
    }
    Ok(())
}

fn empty_character_modifiers(
    data: &[u8],
    fs: &[Entry],
    path: &str,
    edits: &mut Vec<Patch>,
) -> Result<()> {
    for (i, modifier) in fs
        .iter()
        .filter(|e| key(data, e) == b"modifier" && e.block)
        .enumerate()
    {
        let mf = children(data, modifier)?;
        if let Some(name) = unique(data, &mf, b"modifier")?
            && scalar(data, name)?.is_empty()
        {
            edits.push(remove(
                modifier,
                "empty-character-modifier",
                format!("{path}/modifier/{i}"),
            ));
        }
    }
    Ok(())
}

fn empty_references(data: &[u8], root: &[Entry], edits: &mut Vec<Patch>) -> Result<()> {
    if unique(data, root, b"artifacts")?.is_some() {
        for (id, e) in database(data, root, "artifacts/artifacts")?
            .iter()
            .filter(|(_, e)| e.block)
        {
            let fs = children(data, e)?;
            if let Some(modifiers) = unique(data, &fs, b"modifiers")? {
                if !modifiers.block {
                    return Err(fail("expected artifact modifier list"));
                }
                let mut i = modifiers.start + 1;
                while {
                    i = whitespace(data, i, modifiers.end - 1);
                    i < modifiers.end - 1
                } {
                    let end = atom_end(data, i, modifiers.end - 1)?;
                    if &data[i..end] == b"\"\"" {
                        edits.push(patch(
                            i,
                            end,
                            Vec::new(),
                            "empty-artifact-modifier",
                            format!("artifacts/artifacts/{id}/modifiers"),
                        ));
                    }
                    i = end;
                }
            }
        }
    }
    // Settings are repeated key references; only their empty scalar tokens are
    // invalid. The writer synchronizes outer and embedded metadata afterward.
    for path in ["meta_data/game_rules", "game_rules"] {
        let Ok(rules) = at(data, root, path) else {
            continue;
        };
        for e in children(data, &rules)?
            .iter()
            .filter(|e| matches!(key(data, e), b"settings" | b"setting"))
        {
            if e.block {
                let mut i = e.start + 1;
                while {
                    i = whitespace(data, i, e.end - 1);
                    i < e.end - 1
                } {
                    let end = atom_end(data, i, e.end - 1)?;
                    if &data[i..end] == b"\"\"" {
                        edits.push(patch(
                            i,
                            end,
                            Vec::new(),
                            "empty-game-rule",
                            format!("{path}/{}", String::from_utf8_lossy(key(data, e))),
                        ));
                    }
                    i = end;
                }
            } else if scalar(data, e)?.is_empty() {
                edits.push(remove(e, "empty-game-rule", format!("{path}/settings")));
            }
        }
    }
    Ok(())
}

pub(super) fn plan(data: &[u8], root: &[Entry], edits: &mut Vec<Patch>) -> Result<()> {
    opinions(data, root, edits)?;
    contracts(data, root, edits)?;
    accolades(data, root, edits)?;
    culture(data, root, edits)?;
    queue(data, root, edits)?;
    character_cleanup(data, root, edits)?;
    empty_references(data, root, edits)?;
    Ok(())
}
