//! Read-only Node-API adapter over the local Jomini 0.35.0 checkout.
//! Streaming structural inventory, not a CK3 schema validator or save writer.
use jomini::{
    envelope::{JominiFile, JominiFileKind},
    text::{Operator, Token, TokenReader},
};
use napi::{Env, Error, Result, Status, Task, bindgen_prelude::AsyncTask};
use napi_derive::napi;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, Read},
    time::Instant,
};

const MAX_DEPTH: usize = 512;
const MAX_ROOT_KEYS: usize = 4096;
const TOKEN_BUFFER: usize = 1024 * 1024;

fn fail(message: impl ToString) -> Error {
    Error::new(Status::GenericFailure, message.to_string())
}

#[napi(object)]
pub struct MetadataField {
    pub key: String,
    pub value: String,
}

#[napi(object)]
pub struct SectionSummary {
    pub key: String,
    pub occurrences: u32,
    pub child_entries: u32,
}

#[napi(object)]
pub struct SaveInspection {
    pub format: String,
    pub file_bytes: f64,
    pub gamestate_bytes: f64,
    pub metadata_bytes: f64,
    pub gamestate_sha256: String,
    pub crc_verified: bool,
    pub metadata: Vec<MetadataField>,
    pub sections: Vec<SectionSummary>,
    pub token_count: f64,
    pub max_depth: u32,
    pub elapsed_ms: f64,
}

struct Meter<R> {
    inner: R,
    bytes: u64,
    limit: u64,
    hash: Sha256,
    last_error: Option<String>,
}

impl<R: Read> Read for Meter<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        // Read one byte beyond the limit to distinguish exact EOF from overflow.
        let allowed = (self.limit.saturating_sub(self.bytes) + 1).min(buf.len() as u64) as usize;
        let n = match self.inner.read(&mut buf[..allowed]) {
            Ok(n) => n,
            Err(error) => {
                self.last_error = Some(error.to_string());
                return Err(error);
            }
        };
        self.bytes += n as u64;
        if self.bytes > self.limit {
            self.last_error = Some("gamestate exceeds maxBytes".to_owned());
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "gamestate exceeds maxBytes",
            ));
        }
        self.hash.update(&buf[..n]);
        Ok(n)
    }
}

#[derive(Default)]
struct Frame {
    owner: Option<String>,
    pending: Option<String>,
    awaiting_value: bool,
    entries: u32,
}

#[derive(Default)]
struct Scan {
    metadata: Vec<MetadataField>,
    metadata_size: usize,
    sections: BTreeMap<String, (u32, u32)>,
    tokens: u64,
    depth: u32,
}

fn add_root(scan: &mut Scan, key: &str) -> Result<()> {
    if key.len() > 512 {
        return Err(fail("root key exceeds 512 bytes"));
    }
    if !scan.sections.contains_key(key) && scan.sections.len() >= MAX_ROOT_KEYS {
        return Err(fail("too many distinct root keys"));
    }
    scan.sections.entry(key.to_owned()).or_default().0 += 1;
    Ok(())
}

fn flush_bare(frame: &mut Frame, is_root: bool) -> Result<()> {
    if frame.awaiting_value {
        return Err(fail("missing value after operator"));
    }
    if frame.pending.take().is_some() {
        if is_root {
            return Err(fail("bare value at gamestate root"));
        }
        frame.entries += 1;
    }
    Ok(())
}

fn scan(reader: impl Read) -> Result<Scan> {
    let mut reader = TokenReader::from_reader_with_buf(reader, vec![0; TOKEN_BUFFER]);
    let mut stack = vec![Frame::default()];
    let mut result = Scan::default();
    while let Some(token) = reader.next().map_err(fail)? {
        result.tokens += 1;
        let depth = stack.len() - 1;
        let frame = stack.last_mut().expect("root frame exists");
        match token {
            Token::Quoted(s) | Token::Unquoted(s) => {
                let text = std::str::from_utf8(s.as_bytes()).map_err(fail)?;
                if frame.awaiting_value {
                    let key = frame.pending.take().expect("operator has a key");
                    frame.awaiting_value = false;
                    frame.entries += 1;
                    if depth == 0 {
                        add_root(&mut result, &key)?;
                    }
                    if depth == 1 && frame.owner.as_deref() == Some("meta_data") {
                        if result.metadata.len() >= MAX_ROOT_KEYS {
                            return Err(fail("too many metadata scalar fields"));
                        }
                        result.metadata_size += key.len() + text.len();
                        if result.metadata_size > 4 * 1024 * 1024 {
                            return Err(fail("metadata scalar result exceeds 4 MiB"));
                        }
                        result.metadata.push(MetadataField {
                            key,
                            value: text.to_owned(),
                        });
                    }
                } else {
                    flush_bare(frame, depth == 0)?;
                    // Only root and metadata keys need their actual bytes. Deeper keys
                    // are represented by an empty marker, bounding allocations.
                    let keep =
                        depth == 0 || (depth == 1 && frame.owner.as_deref() == Some("meta_data"));
                    frame.pending = Some(if keep { text.to_owned() } else { String::new() });
                }
            }
            Token::Operator(op) => {
                if op != Operator::Equal {
                    return Err(fail("unsupported non-equality save operator"));
                }
                if frame.pending.is_none() || frame.awaiting_value {
                    return Err(fail("operator without a key"));
                }
                frame.awaiting_value = true;
            }
            Token::Open => {
                let owner = if frame.awaiting_value {
                    frame.awaiting_value = false;
                    frame.pending.take()
                } else {
                    flush_bare(frame, depth == 0)?;
                    if depth == 0 {
                        return Err(fail("anonymous block at gamestate root"));
                    }
                    None
                };
                frame.entries += 1;
                if depth == 0 {
                    add_root(&mut result, owner.as_deref().expect("root key"))?;
                }
                if depth + 1 > MAX_DEPTH {
                    return Err(fail("nesting limit exceeded"));
                }
                result.depth = result.depth.max((depth + 1) as u32);
                stack.push(Frame {
                    owner,
                    ..Frame::default()
                });
            }
            Token::Close => {
                if depth == 0 {
                    return Err(fail("unexpected closing brace"));
                }
                flush_bare(frame, false)?;
                let closed = stack.pop().expect("non-root frame");
                if depth == 1
                    && let Some(key) = closed.owner
                {
                    result
                        .sections
                        .get_mut(&key)
                        .expect("registered root block")
                        .1 += closed.entries;
                }
            }
        }
    }
    if stack.len() != 1 {
        return Err(fail("unclosed block at end of gamestate"));
    }
    flush_bare(&mut stack[0], true)?;
    Ok(result)
}

fn inspect(path: &str, max_bytes: u32) -> Result<SaveInspection> {
    if max_bytes == 0 || max_bytes > 2_147_483_648 {
        return Err(fail("invalid maxBytes"));
    }
    let start = Instant::now();
    let file = File::open(path).map_err(fail)?;
    let before = file.metadata().map_err(fail)?;
    if !before.is_file() || before.len() > 2_147_483_648 {
        return Err(fail("expected a regular save file of at most 2 GiB"));
    }
    let observer = file.try_clone().map_err(fail)?;
    let envelope = JominiFile::from_file(file).map_err(fail)?;
    if !envelope.header().kind().is_text() {
        return Err(fail(
            "binary saves require a CK3 token resolver; not supported by this inspector",
        ));
    }
    if envelope.header().metadata_len() > u64::from(max_bytes) {
        return Err(fail("metadata exceeds maxBytes"));
    }
    let (reader, crc_verified): (Box<dyn Read + '_>, bool) = match envelope.kind() {
        JominiFileKind::Zip(zip) => {
            if zip.gamestate_uncompressed_hint() > u64::from(max_bytes) {
                return Err(fail("gamestate exceeds maxBytes"));
            }
            (Box::new(zip.gamestate_verified().map_err(fail)?), true)
        }
        JominiFileKind::Uncompressed(_) => (Box::new(envelope.gamestate().map_err(fail)?), false),
    };
    let mut meter = Meter {
        inner: reader,
        bytes: 0,
        limit: u64::from(max_bytes),
        hash: Sha256::new(),
        last_error: None,
    };
    let scanned = match scan(&mut meter) {
        Ok(scanned) => scanned,
        Err(error) => {
            return Err(match meter.last_error.take() {
                Some(detail) => fail(format!("{error}; {detail}")),
                None => error,
            });
        }
    };
    // The token reader was driven to EOF, including Jomini's ZIP CRC/size check.
    let after = observer.metadata().map_err(fail)?;
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(fail(
            "save changed during inspection; retry on a stable copy",
        ));
    }
    if !scanned.metadata.iter().any(|f| f.key == "version") {
        return Err(fail("missing CK3 meta_data/version"));
    }
    Ok(SaveInspection {
        format: format!("{:?}", envelope.header().kind()),
        file_bytes: before.len() as f64,
        gamestate_bytes: meter.bytes as f64,
        metadata_bytes: envelope.header().metadata_len() as f64,
        gamestate_sha256: format!("{:x}", meter.hash.finalize()),
        crc_verified,
        metadata: scanned.metadata,
        sections: scanned
            .sections
            .into_iter()
            .map(|(key, (occurrences, child_entries))| SectionSummary {
                key,
                occurrences,
                child_entries,
            })
            .collect(),
        token_count: scanned.tokens as f64,
        max_depth: scanned.depth,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

pub struct InspectTask {
    path: String,
    max_bytes: u32,
}

#[napi]
impl Task for InspectTask {
    type Output = SaveInspection;
    type JsValue = SaveInspection;
    fn compute(&mut self) -> Result<Self::Output> {
        inspect(&self.path, self.max_bytes)
    }
    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[napi]
pub fn inspect_save(path: String, max_bytes: u32) -> AsyncTask<InspectTask> {
    AsyncTask::new(InspectTask { path, max_bytes })
}
mod conversion;
