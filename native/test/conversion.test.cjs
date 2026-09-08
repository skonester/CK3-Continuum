const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { deflateRawSync, inflateRawSync } = require('node:zlib');
const { createHash } = require('node:crypto');
const { inspectSave, writeCandidate } = require('../index.cjs');
const out = fs.mkdtempSync(path.join((fs.mkdirSync(path.join(__dirname, '../test-output'), { recursive: true }), path.join(__dirname, '../test-output')), 'convert-'));
const profile = 'experimental-1.16.1-to-1.19.0.6';
const meta = 'meta_data={ version="1.16.1" portraits_version=4 save_game_version=15 }\n';
const world = 'religion={ religions={ 0={ template=akom_religion } } faiths={ 11={ template=orthodox } } }\n' +
  'landed_titles={ landed_titles={ 1184={ key=e_byzantium name="Bárbara { # }" holder=16844662 adj="Latin" article="the " nested={ name="keep" } } 4294967295=none } }\n' +
  'living={ 16844662={ name="Bárbara" traits={ 149 197 } culture=209 family_data={ children={ 42 } } } }\n' +
  'traits_lookup={ intellect_good_3 poet }\nculture_manager={ 209={ name="Latin" template=keep } }\n' +
  'triggered_event={ x=1 }\ntriggered_event={ x=2 }\n# preserved } comment\nlevels={ 11 0=2 1=2 }\n';
function crc32(b) { let c=0xffffffff; for(const v of b){c^=v;for(let i=0;i<8;i++)c=(c>>>1)^((c&1)?0xedb88320:0);}return(c^0xffffffff)>>>0; }
function fixture(name, text=meta+world, compressed=false) {
  const filename=path.join(out,name);
  const head=Buffer.from('SAV01'+(compressed?'02':'00')+'abcdef12'+Buffer.byteLength(meta).toString(16).padStart(8,'0')+'\n');
  if(!compressed){fs.writeFileSync(filename,Buffer.concat([head,Buffer.from(text)]));return filename;}
  const raw=Buffer.from(text), packed=deflateRawSync(raw), prefix=Buffer.concat([head,Buffer.from(meta)]);
  const local=Buffer.alloc(30),central=Buffer.alloc(46),end=Buffer.alloc(22),entry=Buffer.from('gamestate');
  local.writeUInt32LE(0x04034b50);local.writeUInt16LE(20,4);local.writeUInt16LE(8,8);local.writeUInt32LE(crc32(raw),14);
  local.writeUInt32LE(packed.length,18);local.writeUInt32LE(raw.length,22);local.writeUInt16LE(9,26);
  central.writeUInt32LE(0x02014b50);central.writeUInt16LE(20,4);central.writeUInt16LE(20,6);central.writeUInt16LE(8,10);
  central.writeUInt32LE(crc32(raw),16);central.writeUInt32LE(packed.length,20);central.writeUInt32LE(raw.length,24);central.writeUInt16LE(9,28);central.writeUInt32LE(0,42);
  end.writeUInt32LE(0x06054b50);end.writeUInt16LE(1,8);end.writeUInt16LE(1,10);end.writeUInt32LE(55,12);end.writeUInt32LE(39+packed.length,16);
  fs.writeFileSync(filename,Buffer.concat([prefix,local,entry,packed,central,entry,end]));return filename;
}
function extract(filename) {
  const b=fs.readFileSync(filename), h=b.indexOf(10)+1, m=parseInt(b.subarray(15,23).toString(),16);
  const metadata=b.subarray(h,h+m);
  if(b.subarray(5,7).toString()==='00')return{game:b.subarray(h),metadata,header:b.subarray(0,h)};
  const local=h+m, size=b.readUInt32LE(local+18), start=local+30+b.readUInt16LE(local+26)+b.readUInt16LE(local+28);
  const game=inflateRawSync(b.subarray(start,start+size));
  assert.equal(crc32(game),b.readUInt32LE(local+14));
  return{game,metadata,header:b.subarray(0,h)};
}
const sha=b=>createHash('sha256').update(b).digest('hex');
test('plain and ZIP writer controls preserve all gamestate and metadata bytes',async()=>{
  for(const zip of [false,true]){
    const input=fixture('noop-'+zip+'.ck3',meta+world,zip), output=path.join(out,'noop-result-'+zip+'.ck3');
    const before=fs.readFileSync(input), info=await inspectSave(input), result=await writeCandidate(input,output,info.gamestateSha256,'roundtrip');
    assert.deepEqual(extract(output),extract(input));
    assert.equal(result.changes.length,0);
    assert.equal(result.outputGamestateSha256,info.gamestateSha256);
    assert.deepEqual(fs.readFileSync(input),before);
  }
});
test('scoped structural migration preserves IDs, traits, histories, duplicates, comments and Unicode',async()=>{
  const input=fixture('old.ck3',meta+world,true), output=path.join(out,'migrated.ck3'), info=await inspectSave(input);
  const result=await writeCandidate(input,output,info.gamestateSha256,profile);
  const {game,metadata}=extract(output), text=game.toString();
  assert.match(text,/religion_type=akom_religion/); assert.match(text,/faith_type=orthodox/);
  assert.ok(text.includes('title_name_data={\nname="Bárbara { # }"\nadj="Latin"\narticle="the "\n}'));
  assert.ok(text.includes('holder=16844662'));assert.ok(text.includes('nested={ name="keep" }'));
  assert.ok(text.includes(world.slice(world.indexOf('living='))));
  assert.equal((text.match(/triggered_event=/g)||[]).length,2);
  assert.ok(metadata.includes(Buffer.from('version="1.19.0.6"')));assert.deepEqual(game.subarray(0,metadata.length),metadata);
  assert.equal(result.outputVerified,true);assert.equal(result.unchangedSpansVerified,true);
  const original=extract(input).game;
  for(const change of result.changes)assert.equal(sha(original.subarray(change.start,change.end)),change.beforeSha256);
  assert.equal(result.changes.filter(c=>c.rule==='religion-faith-key').length,2);
  assert.equal(result.changes.filter(c=>c.rule==='title-name-block').length,3);
  const second=path.join(out,'migrated-again-from-source.ck3');
  await writeCandidate(input,second,info.gamestateSha256,profile);
  assert.deepEqual(fs.readFileSync(output),fs.readFileSync(second));
  const modern=await inspectSave(output);
  await assert.rejects(writeCandidate(output,path.join(out,'double.ck3'),modern.gamestateSha256,profile),/requires embedded version/);
});
test('stale inspection and output collision never replace files',async()=>{
  const input=fixture('stable.ck3'),output=path.join(out,'existing.ck3'),info=await inspectSave(input);
  fs.writeFileSync(output,'keep');
  await assert.rejects(writeCandidate(input,output,info.gamestateSha256,profile));
  assert.equal(fs.readFileSync(output,'utf8'),'keep');
  await assert.rejects(writeCandidate(input,path.join(out,'stale.ck3'),'0'.repeat(64),profile),/changed since inspection/);
  assert.equal(fs.existsSync(path.join(out,'stale.ck3')),false);
});
test('ambiguous target fields and duplicate migration keys fail closed',async()=>{
  for(const [name,text] of [
    ['religion-conflict',meta+world.replace('template=akom_religion','template=akom_religion religion_type=other')],
    ['name-conflict',meta+world.replace('key=e_byzantium','key=e_byzantium title_name_data={}')],
    ['duplicate',meta+world.replace('template=orthodox','template=orthodox template=other')]
  ]){
    const input=fixture(name+'.ck3',text),info=await inspectSave(input),output=path.join(out,name+'-output.ck3');
    await assert.rejects(writeCandidate(input,output,info.gamestateSha256,profile),/both|duplicate/);
    assert.equal(fs.existsSync(output),false);
  }
});
test('outer/inner mismatch and extra archive entries are rejected',async()=>{
  const input=fixture('mismatch.ck3',meta+world,true);
  let b=fs.readFileSync(input);b[24+meta.indexOf('1.16.1')]=50;fs.writeFileSync(input,b);
  const info=await inspectSave(input);
  await assert.rejects(writeCandidate(input,path.join(out,'mismatch-output.ck3'),info.gamestateSha256,'roundtrip'),/metadata differ/);
  const extra=fixture('extra.ck3',meta+world,true); b=fs.readFileSync(extra);b.writeUInt16LE(2,b.length-22+10);fs.writeFileSync(extra,b);
  await assert.rejects(writeCandidate(extra,path.join(out,'extra-output.ck3'),info.gamestateSha256,'roundtrip'));
});
test('whitespace and escaped quotes do not widen path selection',async()=>{
  const text=meta+world.replace('template=orthodox','template = orthodox').replace('name="Bárbara { # }"','name = "Bárbara \\"quoted\\" { # }"');
  const input=fixture('whitespace.ck3',text),info=await inspectSave(input),output=path.join(out,'whitespace-output.ck3');
  await writeCandidate(input,output,info.gamestateSha256,profile);
  const result=extract(output).game.toString();
  assert.ok(result.includes('faith_type = orthodox'));assert.ok(result.includes('name = "Bárbara \\"quoted\\" { # }"'));
});

// Read directory offsets strictly from the embedded ZIP origin. Unlike
// general ZIP libraries this deliberately performs no prepended-data repair.
function assertEmbeddedZipOffsets(filename) {
  const b = fs.readFileSync(filename);
  const origin = b.indexOf(10) + 1 + parseInt(b.subarray(15,23).toString(),16);
  const zip = b.subarray(origin);
  const eocd = zip.length - 22;
  assert.equal(zip.readUInt32LE(eocd), 0x06054b50);
  const directory = zip.readUInt32LE(eocd + 16);
  assert.ok(directory + 55 <= eocd, 'directory offset must be relative to ZIP, not SAV file');
  assert.equal(zip.readUInt32LE(directory), 0x02014b50);
  const local = zip.readUInt32LE(directory + 42);
  assert.equal(local, 0, 'first local header must be at embedded ZIP offset zero');
  assert.equal(zip.readUInt32LE(local), 0x04034b50);
  assert.equal(directory + zip.readUInt32LE(eocd + 12), eocd);
}
test('embedded ZIP directory offsets follow CK3 archive-relative addressing',async()=>{
  const input=fixture('relative-offset-source.ck3',meta+world,true);
  assertEmbeddedZipOffsets(input);
  const info=await inspectSave(input);
  for(const mode of ['roundtrip',profile]){
    const output=path.join(out,'relative-offset-'+mode+'.ck3');
    await writeCandidate(input,output,info.gamestateSha256,mode);
    assertEmbeddedZipOffsets(output);
  }
});
