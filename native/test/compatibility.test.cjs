const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');
const { inspectSave, lintSave, writeCandidate } = require('../index.cjs');
const { old, reference } = require('./fixtures/upgrade.cjs');
const base = path.join(__dirname, '../test-output');
fs.mkdirSync(base, {recursive:true});
const folder = fs.mkdtempSync(path.join(base, 'compatibility-'));
const upgrade = 'experimental-1.19.0.6-to-1.20.0.3';
const legacy = 'experimental-1.16.1-to-1.19.0.6';
const sha = b => createHash('sha256').update(b).digest('hex');
let serial = 0;
function save(body, version='1.20.0.3', options={}) {
  const meta=`meta_data={ version="${version}" save_game_version=${version==='1.20.0.3'?17:15} portraits_version=${options.portraits??4} ${options.mods?'mods={ "mod/ugc_123.mod" }':''} }\n`;
  const file=path.join(folder, `input-${++serial}.ck3`);
  fs.writeFileSync(file, `SAV0100abcdef12${Buffer.byteLength(meta).toString(16).padStart(8,'0')}\n${meta}${body}`);
  return file;
}
function body(file) { const data=fs.readFileSync(file);return data.subarray(data.indexOf(10)+1); }
async function lint(text, version='1.20.0.3') {
  const file=save(text,version);const before=fs.readFileSync(file);
  const info=await inspectSave(file);const report=await lintSave(file,info.gamestateSha256);
  assert.deepEqual(fs.readFileSync(file),before);
  assert.equal(report.gamestateSha256,info.gamestateSha256);assert.equal(report.engineTested,false);
  return report;
}
async function convert(source, sourceVersion, referenceBody, targetVersion, mode, options={}) {
  const input=save(source,sourceVersion,options), ref=save(referenceBody,targetVersion,{portraits:5,mods:options.referenceMods});
  const before=[fs.readFileSync(input),fs.readFileSync(ref)];
  const out=path.join(folder,`output-${++serial}.ck3`);const info=await inspectSave(input);
  const result=await writeCandidate(input,out,info.gamestateSha256,mode,ref);
  assert.deepEqual(fs.readFileSync(input),before[0]);assert.deepEqual(fs.readFileSync(ref),before[1]);
  // Verify journal and untouched spans independently of the native writer.
  const a=body(input), b=body(out);let src=0,dst=0;
  for(const change of result.changes){
    assert.equal(change.beforeSha256,sha(a.subarray(change.start,change.end)));
    assert.deepEqual(b.subarray(dst,dst+change.start-src),a.subarray(src,change.start));dst+=change.start-src;
    assert.equal(change.afterSha256,sha(b.subarray(dst,dst+change.replacementBytes)));dst+=change.replacementBytes;src=change.end;
  }
  assert.deepEqual(b.subarray(dst),a.subarray(src));
  return {result,text:b.toString(),input,ref,out};
}

test('compatibility lint detects namespace collisions including generations and cross-section duplicates',async()=>{
  const report=await lint('living={ 1={} 16777217={} } dead_unprunable={ 1={} }');
  assert.equal(report.findings.find(f=>f.code==='duplicate-slot').count,1);
  assert.equal(report.findings.find(f=>f.code==='duplicate-id').count,1);
  const repeat=await lint('dead_unprunable={ 1={ first_name="Same" } 1={ first_name="Same" } }');
  assert.equal(repeat.errorCount,0);assert.equal(repeat.warningCount,0);
  const implicit=await lint('living={ 1={ first_name="Implicit faith is valid in 1.19" } }','1.19.0.6');
  assert.equal(implicit.warningCount,0);
});

test('compatibility lint resolves scoped links and title cycles without flagging boolean capital-barony markers',async()=>{
  const report=await lint(`living={ 1={ rite=8 family_data={ spouse=999 } variables={ rite=999 } } }
    religion={ religions={ 3={} } } faiths={ database={ 7={ religion=3 main_rite=8 } } }
    rites={ database={ 8={ faith=7 } } }
    landed_titles={ landed_titles={ 10={ key=c_one de_facto_liege=11 capital_barony=yes holder=1 } 11={ key=d_two de_facto_liege=10 } } }
    vassal_contracts={ database={ 4={ vassal=1 } } }
    # rite=1000 is a comment
    description="rite=1000 title={ }"`);
  assert.equal(report.findings.find(f=>f.code==='title-liege-cycle').count,2);
  assert.equal(report.findings.find(f=>f.code==='missing-contract-party').count,1);
  assert.equal(report.findings.find(f=>f.code==='family-reference').count,1);
  assert.ok(!report.findings.some(f=>f.code==='dangling-reference'));
});

test('compatibility lint reports unresolved faith and rite handles, bounds examples, and rejects stale inspection',async()=>{
  const report=await lint(`living={ ${Array.from({length:12},(_,i)=>`${i+1}={ rite=900 }`).join(' ')} } rites={ database={} }`);
  const missing=report.findings.find(f=>f.code==='dangling-reference');assert.equal(missing.count,12);assert.equal(missing.examples.length,8);
  const file=save('living={}');const info=await inspectSave(file);fs.appendFileSync(file,'other={}');
  await assert.rejects(lintSave(file,info.gamestateSha256),/source changed/);
});

const content=`
armies={ regiments={ 1={ type=teutonic_knights strength=120 history={ type=teutonic_knights } } 2={ type=chu_ko_nu strength=90 } 3={ type=mod_teutonic_knights } } army_regiments={ 4={ type="teutonic_knights" size=300 source=hired } } }
artifacts={ artifacts={ 8={ type=medium_animal_hide description="teutonic_knights" modifiers={ "artifact_befriend_scheme_power_add_1_modifier" artifact_befriend_scheme_phase_duration_add_1_modifier valid_modifier } variables={ type=medium_animal_hide } } } }
`;
test('verified content successors preserve units and artifact records, remove list duplicates, and avoid lookalike fields',async()=>{
  const source=old.replace('7={ key=b_jerusalem holder=101 }','7={ key=b_jerusalem holder=101 modifier={ modifier=rice_fields_modifier expiration_date=1285.1.1 } history={ modifier=rice_fields_modifier } }');
  const {result,text,out}=await convert(source+content,'1.19.0.6',reference,'1.20.0.3',upgrade);
  assert.equal(result.contentMigration.status,'checked');assert.equal(result.contentMigration.mapped,5);assert.equal(result.contentMigration.duplicatesRemoved,1);
  assert.match(text,/1=\{ type="order_serjeants" strength=120 history=\{ type=teutonic_knights \}/);
  assert.match(text,/2=\{ type="shenbigong" strength=90/);assert.match(text,/3=\{ type=mod_teutonic_knights/);
  assert.match(text,/4=\{ type="order_serjeants" size=300 source=hired/);
  assert.match(text,/modifier=\{ modifier="terraced_hills_modifier" expiration_date=1285.1.1 \} history=\{ modifier=rice_fields_modifier \}/);
  assert.match(text,/type="large_animal_hide" description="teutonic_knights"/);
  assert.match(text,/variables=\{ type=medium_animal_hide \}/);
  assert.equal((text.match(/artifact_befriend_scheme_phase_duration_add_1_modifier/g)||[]).length,1);
  assert.ok(!text.includes('"artifact_befriend_scheme_power_add_1_modifier"'));
  assert.equal(result.compatibilityAfter.findings.filter(f=>f.code==='removed-content').length,0);
  const info=await inspectSave(out);const audit=await lintSave(out,info.gamestateSha256);assert.equal(audit.errorCount,result.compatibilityAfter.errorCount);
});

test('modded source or reference prevents vanilla content and portrait assumptions',async()=>{
  for(const options of [{mods:true},{referenceMods:true}]) {
    const source=old.replace('dna="unchanged"',`dna="${Buffer.alloc(416).toString('base64')}"`);
    const {result,text}=await convert(source+content,'1.19.0.6',reference,'1.20.0.3',upgrade,{...options,portraits:5});
    assert.equal(result.contentMigration.status,'skipped');assert.equal(result.portraitMigration.status,'skipped');
    assert.ok(result.portraitMigration.notes.some(n=>n.includes('mods')));
    assert.ok(text.includes('type=teutonic_knights strength=120'));assert.ok(text.includes('portraits_version=5'));
    assert.ok(!result.changes.some(c=>c.rule.startsWith('removed-content')||c.rule.startsWith('portrait-')));
  }
});

function dna() {
  const raw=Buffer.alloc(104*4);
  for(let i=0;i<104;i++){raw[i*4+1]=(i+57)%256;raw[i*4+3]=(i+143)%256;}
  raw[0]=125;raw[2]=245; // color bytes are not template indices
  raw[90*4]=7;raw[90*4+2]=8; // 1.16 complexion beauty/ugly
  raw[102*4]=1; // 1.16 no_teeth; 1.19 places it at index 2
  return raw;
}
test('portrait migration carries actual versioned complexion and teeth templates and preserves allele/color bytes',async()=>{
  const original=dna();const source=old.replace('dna="unchanged"',`dna="${original.toString('base64')}"`);
  const {result,text,input,ref}=await convert(source,'1.16.1',old,'1.19.0.6',legacy);
  const output=Buffer.from(/dna="([^"]+)"/.exec(text)[1],'base64');
  const expected=Buffer.from(original);expected[90*4]=8;expected[90*4+2]=9;expected[102*4]=2;
  assert.deepEqual(output,expected);assert.equal(result.portraitMigration.changed,1);assert.equal(result.portraitMigration.status,'checked');
  assert.equal(result.portraitMigration.sourceVersion,'1.16.1');assert.equal(result.portraitMigration.targetVersion,'1.19.0.6');
  assert.ok(text.includes('portraits_version=5'));assert.ok(result.changes.some(c=>c.rule==='portrait-schema-version'));
  // Repeating from the immutable source produces the same result, not a second remap.
  const output2=path.join(folder,`repeat-${++serial}.ck3`);const info=await inspectSave(input);await writeCandidate(input,output2,info.gamestateSha256,legacy,ref);
  assert.deepEqual(body(output2).toString(),text);
});

test('a single unrecognized face defers every portrait edit and the global portrait version',async()=>{
  const good=dna().toString('base64');
  const source=old.replace('dna="unchanged"',`dna="${good}"`).replace('first_name="Campaign Patriarch"','dna="bad!" first_name="Campaign Patriarch"');
  const {result,text}=await convert(source,'1.16.1',old,'1.19.0.6',legacy);
  assert.equal(result.portraitMigration.status,'skipped');assert.equal(result.portraitMigration.examined,2);assert.equal(result.portraitMigration.preserved,2);
  assert.ok(text.includes(`dna="${good}"`));assert.ok(text.includes('dna="bad!"'));assert.ok(text.includes('portraits_version=4'));
  assert.ok(!result.changes.some(c=>c.rule.startsWith('portrait-')));
});

test('unrecognized exact game versions preserve DNA while supported structural migration remains available',async()=>{
  const good=dna().toString('base64');
  const {result,text}=await convert(old.replace('dna="unchanged"',`dna="${good}"`),'1.18.99',old,'1.19.0.6',legacy);
  assert.equal(result.portraitMigration.status,'skipped');assert.ok(text.includes(`dna="${good}"`));assert.ok(text.includes('portraits_version=4'));
});

test('a previously relabeled 1.19 save with older portrait schema never reinterprets its DNA by length',async()=>{
  const good=dna().toString('base64');
  const {result,text}=await convert(old.replace('dna="unchanged"',`dna="${good}"`),'1.19.0.6',reference,'1.20.0.3',upgrade);
  assert.equal(result.portraitMigration.status,'skipped');assert.ok(result.portraitMigration.notes.some(n=>n.includes('predate')));
  assert.ok(text.includes(`dna="${good}"`));assert.ok(text.includes('portraits_version=4'));
});

test('conversion refuses new namespace errors before publishing, while writer control retains the original data',async()=>{
  const source=old.replace('  11={ faith_type=orthodox', ' 16777227={ faith_type=custom_other religion=3 tag="other" religious_head=4294967295 }\n  11={ faith_type=orthodox');
  const input=save(source,'1.19.0.6'), ref=save(reference,'1.20.0.3',{portraits:5});
  const info=await inspectSave(input), before=fs.readFileSync(input), output=path.join(folder,`refused-${++serial}.ck3`);
  await assert.rejects(writeCandidate(input,output,info.gamestateSha256,upgrade,ref),/introduced a compatibility error \(duplicate-slot\)/);
  assert.equal(fs.existsSync(output),false);assert.deepEqual(fs.readFileSync(input),before);
  const control=path.join(folder,`control-${++serial}.ck3`);const result=await writeCandidate(input,control,info.gamestateSha256,'roundtrip');
  assert.ok(result.compatibilityBefore.errorCount>0);assert.equal(result.compatibilityAfter.errorCount,result.compatibilityBefore.errorCount);
  assert.deepEqual(fs.readFileSync(control),before);
});
