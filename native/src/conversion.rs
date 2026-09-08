//! Experimental, path-scoped migration and byte-preserving container writer.
use super::{fail, inspect, scan};
use flate2::{Compression, write::DeflateEncoder};
use jomini::envelope::{JominiFile, JominiFileKind, SaveHeader, SaveHeaderKind};
use napi::{Env, Result, Task, bindgen_prelude::AsyncTask};
use napi_derive::napi;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    time::Instant,
};

const LIMIT: usize = 512 * 1024 * 1024;
const PROFILE: &str = "experimental-1.16.1-to-1.19.0.6";
const WORLD_PROFILE: &str = "experimental-random-regions-1.16.1-to-1.19.0.6";
#[path = "world.rs"]
mod world;

fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}

#[derive(Clone, Debug)]
struct Entry {
    key_start: usize,
    key_end: usize,
    start: usize,
    end: usize,
    block: bool,
}
fn whitespace(data: &[u8], mut i: usize, end: usize) -> usize {
    loop {
        while i < end && data[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < end && data[i] == b'#' {
            while i < end && data[i] != b'\n' {
                i += 1;
            }
        } else {
            return i;
        }
    }
}
fn atom_end(data: &[u8], start: usize, end: usize) -> Result<usize> {
    if start >= end {
        return Err(fail("missing value"));
    }
    if data[start] == b'"' {
        let mut i = start + 1;
        while i < end {
            if data[i] == b'\\' {
                i += 2;
            } else if data[i] == b'"' {
                return Ok(i + 1);
            } else {
                i += 1;
            }
        }
        return Err(fail("unterminated quoted value"));
    }
    let mut i = start;
    while i < end && !data[i].is_ascii_whitespace() && !b"{}=#\"".contains(&data[i]) {
        i += 1;
    }
    if i == start {
        return Err(fail("unsupported token at source span"));
    }
    Ok(i)
}
fn value_end(data: &[u8], start: usize, end: usize) -> Result<usize> {
    if start >= end {
        return Err(fail("missing value"));
    }
    if data[start] != b'{' {
        return atom_end(data, start, end);
    }
    let mut depth = 1;
    let mut i = start + 1;
    while i < end {
        match data[i] {
            b'"' => {
                i = atom_end(data, i, end)?;
                continue;
            }
            b'#' => {
                while i < end && data[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            b'{' => {
                depth += 1;
                if depth > 512 {
                    return Err(fail("nesting limit exceeded"));
                }
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err(fail("unclosed source block"))
}
// Only immediate assignments are indexed; anonymous/mixed values are skipped,
// never interpreted as database identifiers. Jomini validates the full grammar first.
fn entries(data: &[u8], start: usize, end: usize) -> Result<Vec<Entry>> {
    let mut result = Vec::new();
    let mut i = start;
    while {
        i = whitespace(data, i, end);
        i < end
    } {
        let key_start = i;
        let key_end = value_end(data, i, end)?;
        i = whitespace(data, key_end, end);
        if i < end && data[i] == b'=' {
            let start = whitespace(data, i + 1, end);
            let mut finish = value_end(data, start, end)?;
            // Tagged colors are one value, including their anonymous block.
            if matches!(&data[start..finish], b"rgb" | b"hsv" | b"hsv360") {
                let tagged = whitespace(data, finish, end);
                if tagged < end && data[tagged] == b'{' {
                    finish = value_end(data, tagged, end)?;
                }
            }
            if result.len() >= 1_000_000 {
                return Err(fail("too many immediate entries for writer"));
            }
            result.push(Entry {
                key_start,
                key_end,
                start,
                end: finish,
                block: data[start] == b'{',
            });
            i = finish;
        }
    }
    Ok(result)
}
fn key<'a>(data: &'a [u8], e: &Entry) -> &'a [u8] {
    &data[e.key_start..e.key_end]
}
fn children(data: &[u8], e: &Entry) -> Result<Vec<Entry>> {
    if !e.block {
        return Err(fail("expected a block at migration path"));
    }
    entries(data, e.start + 1, e.end - 1)
}
fn unique<'a>(data: &[u8], list: &'a [Entry], name: &[u8]) -> Result<Option<&'a Entry>> {
    let matches: Vec<_> = list.iter().filter(|e| key(data, e) == name).collect();
    if matches.len() > 1 {
        return Err(fail(format!(
            "ambiguous duplicate field: {}",
            String::from_utf8_lossy(name)
        )));
    }
    Ok(matches.first().copied())
}
fn required<'a>(data: &[u8], list: &'a [Entry], name: &[u8]) -> Result<&'a Entry> {
    unique(data, list, name)?.ok_or_else(|| {
        fail(format!(
            "missing required field: {}",
            String::from_utf8_lossy(name)
        ))
    })
}
fn scalar<'a>(data: &'a [u8], e: &Entry) -> Result<&'a str> {
    if e.block {
        return Err(fail("expected scalar"));
    }
    let raw = std::str::from_utf8(&data[e.start..e.end]).map_err(fail)?;
    Ok(raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw))
}

#[napi(object)]
pub struct ConversionChange {
    pub rule: String,
    pub path: String,
    pub start: f64,
    pub end: f64,
    pub replacement_bytes: f64,
    pub before_sha256: String,
    pub after_sha256: String,
}
struct Patch {
    start: usize,
    end: usize,
    bytes: Vec<u8>,
    rule: &'static str,
    path: String,
}
fn patch(start: usize, end: usize, bytes: Vec<u8>, rule: &'static str, path: String) -> Patch {
    Patch {
        start,
        end,
        bytes,
        rule,
        path,
    }
}
fn apply(data: &[u8], edits: &[Patch]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(data.len());
    let mut cursor = 0;
    for edit in edits {
        if edit.start < cursor || edit.end < edit.start || edit.end > data.len() {
            return Err(fail("overlapping or invalid migration spans"));
        }
        output.extend_from_slice(&data[cursor..edit.start]);
        output.extend_from_slice(&edit.bytes);
        cursor = edit.end;
        if output.len() > LIMIT {
            return Err(fail("converted gamestate exceeds 512 MiB"));
        }
    }
    output.extend_from_slice(&data[cursor..]);
    if output.len() > LIMIT {
        return Err(fail("converted gamestate exceeds 512 MiB"));
    }
    Ok(output)
}
fn plan(data: &[u8], mode: &str) -> Result<Vec<Patch>> {
    if mode == "roundtrip" {
        return Ok(Vec::new());
    }
    if mode != PROFILE {
        return Err(fail("unknown conversion profile"));
    }
    let root = entries(data, 0, data.len())?;
    let meta = children(data, required(data, &root, b"meta_data")?)?;
    let version = required(data, &meta, b"version")?;
    if scalar(data, version)? != "1.16.1" {
        return Err(fail(
            "experimental profile requires embedded version 1.16.1",
        ));
    }
    let mut edits = vec![patch(
        version.start,
        version.end,
        b"\"1.19.0.6\"".to_vec(),
        "version-label",
        "meta_data/version".into(),
    )];
    let religion = children(data, required(data, &root, b"religion")?)?;
    for (database, new_key) in [("religions", "religion_type"), ("faiths", "faith_type")] {
        let db = required(data, &religion, database.as_bytes())?;
        let records = children(data, db)?;
        for record in records.iter().filter(|e| e.block) {
            let id = std::str::from_utf8(key(data, record)).map_err(fail)?;
            if !id.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let fields = children(data, record)?;
            if let Some(old) = unique(data, &fields, b"template")? {
                if unique(data, &fields, new_key.as_bytes())?.is_some() {
                    return Err(fail("both legacy and target religion/faith fields exist"));
                }
                edits.push(patch(
                    old.key_start,
                    old.key_end,
                    new_key.as_bytes().to_vec(),
                    "religion-faith-key",
                    format!("religion/{database}/{id}/template"),
                ));
            }
        }
    }
    let titles_outer = children(data, required(data, &root, b"landed_titles")?)?;
    let titles = children(data, required(data, &titles_outer, b"landed_titles")?)?;
    let name_keys: &[&[u8]] = &[b"name", b"adj", b"article", b"pre", b"localization_key"];
    for title in titles.iter().filter(|e| e.block) {
        let id = std::str::from_utf8(key(data, title)).map_err(fail)?;
        if !id.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let fields = children(data, title)?;
        if unique(data, &fields, b"name")?.is_none() {
            continue;
        }
        if unique(data, &fields, b"title_name_data")?.is_some() {
            return Err(fail("title contains both legacy and target name structure"));
        }
        for name in name_keys {
            unique(data, &fields, name)?;
        }
        let selected: Vec<_> = fields
            .iter()
            .filter(|e| name_keys.contains(&key(data, e)))
            .collect();
        let mut replacement = b"title_name_data={\n".to_vec();
        for field in &selected {
            if field.block {
                return Err(fail("unsupported block-valued title display field"));
            }
            replacement.extend_from_slice(&data[field.key_start..field.end]);
            replacement.push(b'\n');
        }
        replacement.push(b'}');
        for (index, field) in selected.iter().enumerate() {
            edits.push(patch(
                field.key_start,
                field.end,
                if index == 0 {
                    replacement.clone()
                } else {
                    Vec::new()
                },
                "title-name-block",
                format!(
                    "landed_titles/landed_titles/{id}/{}",
                    String::from_utf8_lossy(key(data, field))
                ),
            ));
        }
    }
    edits.sort_by_key(|p| p.start);
    if edits.len() > 250_000 {
        return Err(fail("too many migration changes"));
    }
    Ok(edits)
}

fn read_game(raw: &[u8]) -> Result<(SaveHeader, Vec<u8>, Vec<u8>)> {
    let envelope = JominiFile::from_slice(raw).map_err(fail)?;
    let header = envelope.header().clone();
    if !matches!(
        header.kind(),
        SaveHeaderKind::Text | SaveHeaderKind::UnifiedText
    ) {
        return Err(fail(
            "writer supports only SAV text (00) and unified compressed text (02); binary/split/other saves are rejected",
        ));
    }
    if header.metadata_len() > 4 * 1024 * 1024 {
        return Err(fail("metadata exceeds writer limit"));
    }
    let mut game = Vec::new();
    match envelope.kind() {
        JominiFileKind::Zip(zip) => {
            if zip.gamestate_uncompressed_hint() > LIMIT as u64 {
                return Err(fail("gamestate exceeds 512 MiB"));
            }
            zip.gamestate_verified()
                .map_err(fail)?
                .take((LIMIT + 1) as u64)
                .read_to_end(&mut game)
                .map_err(fail)?;
        }
        JominiFileKind::Uncompressed(_) => {
            envelope
                .gamestate()
                .map_err(fail)?
                .take((LIMIT + 1) as u64)
                .read_to_end(&mut game)
                .map_err(fail)?;
        }
    }
    if game.len() > LIMIT {
        return Err(fail("gamestate exceeds 512 MiB"));
    }
    std::str::from_utf8(&game).map_err(fail)?;
    scan(&game[..])?;
    let h = header.header_len();
    let m = header.metadata_len() as usize;
    let outer = raw
        .get(h..h + m)
        .ok_or_else(|| fail("truncated outer metadata"))?
        .to_vec();
    if game.get(..m) != Some(&outer[..]) {
        return Err(fail(
            "outer and inner metadata differ; refusing ambiguous rewrite",
        ));
    }
    let root = entries(&game, 0, game.len())?;
    let meta = required(&game, &root, b"meta_data")?;
    if meta.key_start != 0
        || !meta.block
        || meta.end > m
        || !game[meta.end..m].iter().all(u8::is_ascii_whitespace)
    {
        return Err(fail("unsupported metadata boundary"));
    }
    // Reject extra archive members rather than silently discarding them.
    if header.kind() == SaveHeaderKind::UnifiedText {
        let zip = &raw[h + m..];
        let eocd = zip
            .len()
            .checked_sub(22)
            .ok_or_else(|| fail("invalid ZIP trailer"))?;
        if &zip[eocd..eocd + 4] != b"PK\x05\x06" || zip[eocd + 8..eocd + 12] != [1, 0, 1, 0] {
            return Err(fail(
                "writer requires a single-entry ZIP with no archive comment or ZIP64",
            ));
        }
        let filename_len = u16::from_le_bytes(
            zip.get(26..28)
                .ok_or_else(|| fail("bad ZIP header"))?
                .try_into()
                .unwrap(),
        ) as usize;
        if zip.get(30..30 + filename_len) != Some(b"gamestate") {
            return Err(fail("unexpected ZIP entry"));
        }
    }
    Ok((header, outer, game))
}
fn u16le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn u32le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn container(mut header: SaveHeader, meta: &[u8], game: &[u8]) -> Result<Vec<u8>> {
    header.set_metadata_len(meta.len() as u64);
    let mut out = Vec::new();
    header.write(&mut out).map_err(fail)?;
    if header.kind() == SaveHeaderKind::Text {
        out.extend_from_slice(game);
        return Ok(out);
    }
    out.extend_from_slice(meta);
    let offset = out.len() as u32;
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(game).map_err(fail)?;
    let compressed = encoder.finish().map_err(fail)?;
    let crc = crc32fast::hash(game);
    let packed = compressed.len() as u32;
    let expanded = game.len() as u32;
    u32le(&mut out, 0x04034b50);
    for n in [20, 0, 8, 0, 33] {
        u16le(&mut out, n);
    }
    for n in [crc, packed, expanded] {
        u32le(&mut out, n);
    }
    u16le(&mut out, 9);
    u16le(&mut out, 0);
    out.extend_from_slice(b"gamestate");
    out.extend_from_slice(&compressed);
    let central = out.len() as u32;
    u32le(&mut out, 0x02014b50);
    for n in [20, 20, 0, 8, 0, 33] {
        u16le(&mut out, n);
    }
    for n in [crc, packed, expanded] {
        u32le(&mut out, n);
    }
    for n in [9, 0, 0, 0, 0] {
        u16le(&mut out, n);
    }
    u32le(&mut out, 0);
    // CK3 opens the embedded ZIP after the SAV header and metadata.
    // Directory addresses are relative to that ZIP, not the whole .ck3 file.
    u32le(&mut out, 0);
    out.extend_from_slice(b"gamestate");
    let central_size = out.len() as u32 - central;
    u32le(&mut out, 0x06054b50);
    for n in [0, 0, 1, 1] {
        u16le(&mut out, n);
    }
    u32le(&mut out, central_size);
    u32le(&mut out, central - offset);
    u16le(&mut out, 0);
    Ok(out)
}

#[napi(object)]
pub struct ConversionResult {
    pub profile: String,
    pub source_sha256: String,
    pub output_sha256: String,
    pub source_gamestate_sha256: String,
    pub output_gamestate_sha256: String,
    pub output_bytes: f64,
    pub unchanged_spans_verified: bool,
    pub output_verified: bool,
    pub elapsed_ms: f64,
    pub changes: Vec<ConversionChange>,
    pub reference_sha256: Option<String>,
}
fn convert(
    input: &str,
    output: &str,
    expected: &str,
    mode: &str,
    reference: Option<&str>,
) -> Result<ConversionResult> {
    let start = Instant::now();
    let mut raw = Vec::new();
    fs::File::open(input)
        .map_err(fail)?
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut raw)
        .map_err(fail)?;
    if raw.len() > LIMIT {
        return Err(fail("writer input exceeds 512 MiB"));
    }
    let source_hash = hash(&raw);
    let (header, outer, game) = read_game(&raw)?;
    let original_game_hash = hash(&game);
    if original_game_hash != expected {
        return Err(fail("source changed since inspection; inspect it again"));
    }
    let mut edits = plan(&game, if mode == WORLD_PROFILE { PROFILE } else { mode })?;
    let reference_sha256 = if mode == WORLD_PROFILE {
        let path = reference
            .ok_or_else(|| fail("random-region conversion requires a 1.19.0.6 reference save"))?;
        let mut raw_reference = Vec::new();
        fs::File::open(path)
            .map_err(fail)?
            .take((LIMIT + 1) as u64)
            .read_to_end(&mut raw_reference)
            .map_err(fail)?;
        if raw_reference.len() > LIMIT {
            return Err(fail("reference exceeds 512 MiB"));
        }
        let reference_hash = hash(&raw_reference);
        let (_, _, reference_game) = read_game(&raw_reference)?;
        edits.extend(world::plan_world(&game, &reference_game)?);
        edits.sort_by_key(|p| p.start);
        Some(reference_hash)
    } else {
        None
    };
    let next_game = apply(&game, &edits)?;
    let metadata_edits: Vec<_> = edits
        .iter()
        .filter(|p| p.start < outer.len())
        .map(|p| Patch {
            start: p.start,
            end: p.end,
            bytes: p.bytes.clone(),
            rule: p.rule,
            path: p.path.clone(),
        })
        .collect();
    let next_meta = apply(&outer, &metadata_edits)?;
    if next_game.get(..next_meta.len()) != Some(&next_meta[..]) {
        return Err(fail("metadata synchronization failed"));
    }
    scan(&next_game[..])?;
    // Prove each unchanged range and each replacement appears at its planned
    // output offset; record exact changed ranges and hashes in the journal.
    let mut src = 0;
    let mut dst = 0;
    for p in &edits {
        let unchanged = p.start - src;
        if next_game[dst..dst + unchanged] != game[src..p.start] {
            return Err(fail("untouched span differs"));
        }
        dst += unchanged;
        if next_game[dst..dst + p.bytes.len()] != p.bytes {
            return Err(fail("replacement verification failed"));
        }
        dst += p.bytes.len();
        src = p.end;
    }
    if next_game[dst..] != game[src..] {
        return Err(fail("untouched tail differs"));
    }
    let next_hash = hash(&next_game);
    let encoded = container(header, &next_meta, &next_game)?;
    let (_, verified_meta, verified_game) = read_game(&encoded)?;
    if verified_meta != next_meta || verified_game != next_game {
        return Err(fail("container round-trip mismatch"));
    }
    drop(verified_game);
    drop(raw);
    // Detect all source changes, including envelope-only edits since this read.
    let mut observer = fs::File::open(input).map_err(fail)?;
    let mut digest = Sha256::new();
    std::io::copy(&mut observer, &mut digest).map_err(fail)?;
    if format!("{:x}", digest.finalize()) != source_hash {
        return Err(fail("source changed during conversion"));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(fail)?;
    file.write_all(&encoded).map_err(fail)?;
    file.sync_all().map_err(fail)?;
    drop(file);
    let verified = inspect(output, LIMIT as u32)?;
    if verified.gamestate_sha256 != next_hash {
        return Err(fail("output readback hash mismatch"));
    }
    Ok(ConversionResult {
        profile: mode.into(),
        source_sha256: source_hash,
        output_sha256: hash(&encoded),
        source_gamestate_sha256: original_game_hash,
        output_gamestate_sha256: next_hash,
        output_bytes: encoded.len() as f64,
        unchanged_spans_verified: true,
        output_verified: true,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
        changes: edits
            .iter()
            .map(|p| ConversionChange {
                rule: p.rule.into(),
                path: p.path.clone(),
                start: p.start as f64,
                end: p.end as f64,
                replacement_bytes: p.bytes.len() as f64,
                before_sha256: hash(&game[p.start..p.end]),
                after_sha256: hash(&p.bytes),
            })
            .collect(),
        reference_sha256,
    })
}
pub struct ConvertTask {
    input: String,
    output: String,
    expected: String,
    mode: String,
    reference: Option<String>,
}
#[napi]
impl Task for ConvertTask {
    type Output = ConversionResult;
    type JsValue = ConversionResult;
    fn compute(&mut self) -> Result<Self::Output> {
        convert(
            &self.input,
            &self.output,
            &self.expected,
            &self.mode,
            self.reference.as_deref(),
        )
    }
    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}
#[napi]
pub fn write_candidate(
    input: String,
    output: String,
    expected: String,
    mode: String,
    reference: Option<String>,
) -> AsyncTask<ConvertTask> {
    AsyncTask::new(ConvertTask {
        input,
        output,
        expected,
        mode,
        reference,
    })
}
