const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');
const { inspectSave, writeCandidate } = require('../index.cjs');
const profile = 'experimental-1.19.0.6-to-1.20.0.3';
const folder = fs.mkdtempSync(path.join((fs.mkdirSync(path.join(__dirname, '../test-output'), { recursive: true }), path.join(__dirname, '../test-output')), 'upgrade-'));
const old = `date=1283.8.10
played_character={ character=101 name="Barbara" }
religion={
 religions={ 3={ religion_type=christianity_religion faiths={ 11 20 21 } } }
 faiths={
  11={ faith_type=orthodox religion=3 tag="orthodox" color=rgb { 178 0 127 } fervor=73.5 religious_head=5 doctrine=tenet_communion doctrine=tenet_monasticism doctrine=doctrine_monogamy holy_sites={ 0 } changes={ 1283.1.1="campaign history" } variables={ flag=keep } }
  20={ faith_type=insular_celtic religion=3 tag="insular_celtic" religious_head=4294967295 doctrine=tenet_ritual_celebrations holy_sites={ 0 } }
  21={ faith_type="" religion=3 tag="custom_faith" name="Barbara's Faith" religious_head=4294967295 doctrine=tenet_communion holy_sites={ 1 } }
  4294967295=none
 }
 holy_sites={ 0={ holy_site_template=jerusalem is_active=yes } 1={ holy_site_type=mod_site } }
 great_holy_wars={ campaign=keep }
}
landed_titles={ landed_titles={
 5={ key=d_patriarch holder=100 history={ 1282.1.1={ holder=100 } } }
 7={ key=b_jerusalem holder=101 }
 9={ key=e_campaign holder=101 state_faith=11 title_name_data={ name="The Confederation" } }
} }
county_manager={ counties={ c_jerusalem={ culture=209 faith=11 development=42 variables={ faith=21 } } } }
provinces={ 1={ holding={ type=castle_holding levy=77 } } }
living={
 100={ first_name="Campaign Patriarch" faith=11 }
 101={ first_name="Bárbara" faith=11 secret_faith=20 culture=209 dna="unchanged" traits={ 0 1 } trait_xp_amounts={ 10 } family_data={ child={ 102 } } variables={ faith=21 } }
 102={ first_name="No recorded faith" }
}
dead_unprunable={ 103={ first_name="Ancestor" faith=20 traits={ 0 } } }
characters={ dead_prunable={ 104={ faith=11 } } unborn={ 105={ faith=21 } } }
holy_orders={ holy_orders={ 2={ faith=11 title=9 regiments={ 77 } founder=101 } } }
traits_lookup={ scholar poet brave }
triggered_event={ faith=11 }
triggered_event={ faith=20 }
`;
const reference = `date=1066.9.17
religion={ religions={ 8={ religion_type=christianity_religion } } holy_sites={ 80={ holy_site_type=jerusalem barony=700 inventory={ artifacts={ 999 } } } } }
faiths={ database={
 99={ faith_type=orthodox religion=8 main_rite=55 name="Orthodoxy" adjective="Orthodox" fervor=50 religious_head=500 variables={ flag=donor_only } }
} saints={ donor_saint=999 } }
rites={ database={
 55={ rite_type=byzantine_rite faith=99 head_of_rite=900 data={ name="Greek Rite" doctrine=donor_only } }
 56={ rite_type=insular_celtic faith=99 head_of_rite=901 data={ name="Insular" adjective="Insular" } }
} }
landed_titles={ landed_titles={ 700={ key=b_jerusalem holder=900 } 500={ key=d_patriarch holder=900 } } }
county_manager={ counties={ c_jerusalem={ rite=55 } } }
provinces={ 1={ holding={ type=castle_holding levy=888 } } }
traits_lookup={ erudite lifestyle_poet brave cleric }
organization_manager={ database={ 999={ faith=99 head=900 } } }
clerical_region_manager={ database={ 5={ title=500 } } }
house_relations={ database={ donor=99 } }
barter_missions={ database={} }
great_projects={ database={} }
situation_participant_group_manager={ database={ donor=900 } }
situation_sub_region_manager={ database={} }
theocracy_lease_manager={ donor=700 }
title_and_vassal_change_manager={ next_id=400 }
`;
let serial = 0;
const cleanupSource = old.replace('100={ first_name="Campaign Patriarch" faith=11 }', '100={ first_name="Campaign Patriarch" faith=11 landed_data={ government=clan_government } }')
  .replace('102={ first_name="No recorded faith" }', '102={ first_name="No recorded faith" landed_data={ government=administrative_government } }')
  .replace('dna="unchanged"', `landed_data={ government=feudal_government } alive_data={ perk={ pedagogy_perk scholar_perk } modifier={ modifier="" expiration_date=1284.1.1 } } playable_data={ active_accolades={ 5 } inactive_accolades={ 6 } } modifier={ modifier="" expiration_date=1284.1.1 } modifier={ modifier=valid_modifier expiration_date=1284.1.1 } dna="unchanged"`)
  + `
living_extra_for_test={ unused=yes }
opinions={ active_opinions={
 { owner=100 target=101 temporary_opinion={ modifier=impressed_opinion start_date=1272.11.12 expiration_date=1285.3.12 converging={ days=3921 opinion=15.75 } value=2 } scripted_relations={ friendship=yes } }
 { owner=101 target=100 temporary_opinion={ modifier=friend_had_good_time_at_feast_opinion start_date=1276.8.29 expiration_date=1286.8.29 converging={ total_days=3650 days=2536 opinion=15 } value=5 } temporary_opinion={ modifier=childhood_victim start_date=1250.12.25 expiration_date=9999.1.1 value=-5 used_punishments={ imprisonment_reason=yes } } }
 { owner=102 target=101 temporary_opinion={ modifier=voting_for_me_opinion start_date=1283.8.10 expiration_date=9999.1.1 converging={ opinion=25 } value=0 } }
} }
vassal_contracts={ database={
 1={ vassal=101 liege=100 date=1278.5.17 levels={ 11 0=2 1=3 9=1 10=1 } contract_group="" }
 2={ vassal=100 liege=101 date=1278.5.17 levels={ 7 5=1 6=1 } contract_group=clan_vassal }
 3={ vassal=102 liege=101 date=1278.5.17 levels={ 2 1=2 } contract_group=admin_vassal }
 4={ vassal=101 liege=100 date=1278.5.17 levels={ 5 0=2 1=2 } contract_group=tributary_settled }
} }
accolades={ database={
 5={ primary=disciplinarian_attribute secondary=thug_attribute glory=600 name="Barbara's Guard" owner=101 acclaimed=100 primary_icon=horn secondary_icon=star_1 history={ { acclaimed=102 date=1280.1.1 } } }
 6={ primary=stalwart_attribute secondary=idealist_attribute glory=60 name="Vacant Guard" owner=101 history={ { acclaimed=102 date=1280.1.1 } } }
} }
culture_manager={ cultures={ 209={ exposure_marker=innovation_bombard exposure_type=religion fascination_marker=innovation_bombard culture_marker=3 none="" culture_innovation={ { type=innovation_bombard progress=83.25 } { type=innovation_catapult progress=100 } } variables={ scholar_perk=keep innovation_bombard=keep exposure_type=keep } } } }
triggered_event={ on_action="bp2_parent_guardian_hostage_taker_pulse" scope={ root={ type=char identity=101 } } date=1283.8.11 }
triggered_event={ on_action=faith_fervor_events_pulse date=1283.8.12 }
triggered_event={ on_action=valid_pulse scope={ variables={ faith_fervor_events_pulse=keep } } date=1283.8.12 }
artifacts={ artifacts={ 8={ name="Sword" modifiers={ valid_modifier "" second_modifier } } } }
game_rules={ settings="" setting={ "" valid_rule } }
`;
function save(text, version='1.19.0.6', format=15) {
  const m=`meta_data={ version="${version}" save_game_version=${format} portraits_version=4 meta_player_name="Bárbara" meta_date=1283.8.10 }\n`;
  const file=path.join(folder,`input-${++serial}.ck3`);
  fs.writeFileSync(file,'SAV0100abcdef12'+Buffer.byteLength(m).toString(16).padStart(8,'0')+'\n'+m+text);return file;
}
function body(file) { const b=fs.readFileSync(file);return b.subarray(b.indexOf(10)+1); }
const sha = b => createHash('sha256').update(b).digest('hex');

test('cleanup transfers opinion adjustment and duration while preserving values, dates and relationships',async()=>{
  const input=save(cleanupSource),ref=save(reference,'1.20.0.3',17),out=path.join(folder,'cleanup.ck3');
  const before=fs.readFileSync(input),info=await inspectSave(input);
  const result=await writeCandidate(input,out,info.gamestateSha256,profile,ref);const text=body(out).toString();
  assert.ok(!text.includes('converging='));
  assert.match(text,/modifier=impressed_opinion start_date=1272\.11\.12 expiration_date=1285\.3\.12 modify=15\.75 value=2/);
  assert.match(text,/modify=15 days=3650 value=5/);assert.ok(!text.includes('days=2536')&&!text.includes('days=3921'));
  assert.match(text,/value=-5 used_punishments=\{ imprisonment_reason=yes \}\s+modify=-5/);
  assert.match(text,/modify=25 value=0/);assert.ok(text.includes('scripted_relations={ friendship=yes }'));
  assert.match(text,/levels=\{ 12 0=2 1=3 10=1 11=1 \} contract_group=feudal_vassal/);
  assert.match(text,/levels=\{ 8 6=1 7=1 \} contract_group=clan_vassal/);
  assert.match(text,/levels=\{ 3 1=2 \} contract_group=admin_vassal/);
  assert.ok(text.includes('levels={ 5 0=2 1=2 } contract_group=tributary_settled'));
  assert.match(text,/types=\{ \{ level=2 type="disciplinarian_attribute" \} \{ level=1 type="thug_attribute" \} \}/);
  assert.match(text,/types=\{ \{ level=1 type="stalwart_attribute" \} \{ level=1 type="idealist_attribute" \} \}/);
  assert.ok(text.includes('glory=600 name="Barbara\'s Guard" owner=101 acclaimed=100'));
  assert.ok(text.includes('history={ { acclaimed=102 date=1280.1.1 } }'));
  assert.ok(text.includes('playable_data={ accolades={ 5 6 }'));
  assert.ok(!text.includes('primary_icon=')&&!text.includes('inactive_accolades='));
  assert.ok(text.includes('perk={ pedagogy_perk erudite_perk }'));assert.ok(text.includes('dna="unchanged"'));
  assert.ok(text.includes('spread_marker=innovation_gunpowder'));assert.ok(text.includes('fascination_marker=innovation_gunpowder'));
  assert.ok(text.includes('type=innovation_gunpowder progress=83.25'));assert.ok(text.includes('culture_marker=3'));
  assert.ok(text.includes('variables={ scholar_perk=keep innovation_bombard=keep exposure_type=keep }'));
  assert.ok(!text.includes('on_action="bp2_parent_guardian_hostage_taker_pulse"')&&!text.includes('on_action=faith_fervor_events_pulse'));
  assert.ok(text.includes('on_action=valid_pulse scope={ variables={ faith_fervor_events_pulse=keep } } date=1283.8.12'));
  assert.ok(text.includes('modifiers={ valid_modifier  second_modifier }'));
  assert.ok(!text.includes('modifier=""'));assert.ok(text.includes('modifier=valid_modifier expiration_date=1284.1.1'));
  assert.ok(text.includes('game_rules={  setting={  valid_rule } }'));assert.deepEqual(fs.readFileSync(input),before);
  const counts={};for(const c of result.changes)counts[c.rule]=(counts[c.rule]||0)+1;
  assert.equal(counts['opinion-decay-schema'],3);assert.equal(counts['opinion-flat-adjustment'],1);
  assert.equal(counts['contract-obligation-map'],3);assert.equal(counts['obsolete-queued-on-action'],2);
  assert.equal(counts['empty-character-modifier'],2);
  // Verify every edit and all intervening bytes, including the large records.
  const src=body(input),dst=body(out);let si=0,di=0;
  for(const c of result.changes){const n=c.start-si;assert.deepEqual(dst.subarray(di,di+n),src.subarray(si,c.start));di+=n;assert.equal(sha(src.subarray(c.start,c.end)),c.beforeSha256);assert.equal(sha(dst.subarray(di,di+c.replacementBytes)),c.afterSha256);di+=c.replacementBytes;si=c.end;}
  assert.deepEqual(dst.subarray(di),src.subarray(si));
});

test('cleanup recovers malformed republic contracts with an explicit fixed-obligation fallback',async()=>{
  const source=cleanupSource.replace('government=feudal_government','government=republic_government').replace('11 0=2 1=3 9=1 10=1','5 0=2 1=2');
  const input=save(source),ref=save(reference,'1.20.0.3',17),out=path.join(folder,'republic.ck3'),info=await inspectSave(input);
  const result=await writeCandidate(input,out,info.gamestateSha256,profile,ref);
  assert.match(body(out).toString(),/1=\{ vassal=101 liege=100 date=1278\.5\.17 levels=\{ 1 \} contract_group=republic_vassal/);
  assert.equal(result.changes.filter(c=>c.rule==='contract-republic-default').length,1);
});

test('cleanup retains knight assignments on formerly inactive accolades',async()=>{
  const input=save(cleanupSource.replace('name="Vacant Guard" owner=101','name="Vacant Guard" owner=101 acclaimed=102'));
  const ref=save(reference,'1.20.0.3',17),out=path.join(folder,'inactive-assigned.ck3'),info=await inspectSave(input);
  await writeCandidate(input,out,info.gamestateSha256,profile,ref);
  const text=body(out).toString();assert.ok(text.includes('name="Vacant Guard" owner=101 acclaimed=102'));
  assert.ok(text.includes('playable_data={ accolades={ 5 6 }'));assert.ok(!text.includes('inactive_accolades='));
});

test('cleanup rejects ambiguous schemas, collisions, invalid obligations and dangling accolade lists',async()=>{
  const bad=[
    ['days=3921 opinion=15.75','days=3921 opinion=15.75 mystery=1',/unsupported converging/],
    ['converging={ days=3921 opinion=15.75 }','modify=15 converging={ days=3921 opinion=15.75 }',/mixed opinion/],
    ['11 0=2 1=3 9=1 10=1','11 0=2 11=1',/out-of-range/],
    ['11 0=2 1=3 9=1 10=1','11 0=2 0=3',/duplicate/],
    ['11 0=2 1=3 9=1 10=1','11 0=99',/invalid old obligation level/],
    ['contract_group=clan_vassal','contract_group=custom_unknown',/unknown contract group/],
    ['active_accolades={ 5 }','active_accolades={ 99 }',/dangling character accolade/],
    ['glory=600','glory=-1',/invalid accolade glory/],
    ['perk={ pedagogy_perk scholar_perk }','perk={ erudite_perk scholar_perk }',/perk alias collision/],
    ['type=innovation_catapult','type=innovation_gunpowder',/innovation alias collision/],
    ['exposure_marker=innovation_bombard','spread_marker=innovation_bombard exposure_marker=innovation_bombard',/mixed culture spread/],
    ['primary=disciplinarian_attribute','types={} primary=disciplinarian_attribute',/mixed accolade type/]
  ];
  const ref=save(reference,'1.20.0.3',17);
  for(const [a,b,error] of bad){const input=save(cleanupSource.replace(a,b)),out=path.join(folder,`cleanup-rejected-${++serial}.ck3`),info=await inspectSave(input);await assert.rejects(writeCandidate(input,out,info.gamestateSha256,profile,ref),error);assert.equal(fs.existsSync(out),false);}
});

test('1.20 migration preserves campaign identities and builds rites without reference political state',async()=>{
  const input=save(old), ref=save(reference,'1.20.0.3',17), output=path.join(folder,'candidate.ck3');
  const before=fs.readFileSync(input), rb=fs.readFileSync(ref), info=await inspectSave(input);
  const result=await writeCandidate(input,output,info.gamestateSha256,profile,ref);
  const b=body(output), text=b.toString();
  assert.match(text,/version="1\.20\.0\.3" save_game_version=17 portraits_version=4/);
  assert.match(text,/101=\{ first_name="Bárbara" rite=11 secret_faith=20 culture=209 dna="unchanged" traits=\{ 0 1 \} trait_xp_amounts=\{ 10 \}/);
  assert.ok(!text.includes('secret_rite'));
  assert.match(text,/103=\{ first_name="Ancestor" rite=20/);assert.match(text,/104=\{ rite=11 \}/);assert.match(text,/105=\{ rite=21 \}/);
  assert.ok(text.includes('2={ rite=11 title=9 regiments={ 77 } founder=101 }'));
  assert.ok(text.includes('102={ first_name="No recorded faith" }'));
  assert.ok(text.includes('c_jerusalem={ culture=209 rite=11 development=42 variables={ faith=21 } }'));
  assert.ok(text.includes('holder=101 state_rite=11 title_name_data={ name="The Confederation" }'));
  assert.ok(text.includes('faith=11\nfounder=4294967295\nhead_of_rite=100'));
  assert.ok(text.includes('holy_site_type="jerusalem"\nbarony=7\ninventory={ holy_site_owner=0 }\nfaiths={ 11 20 }'));
  assert.ok(text.includes('holy_site_type="mod_site"\nbarony=4294967295'));
  assert.ok(text.includes('faith_type="orthodox"\nreligion=3\nmain_rite=11\norganization=11'));
  assert.ok(!text.includes('faith_type="insular_celtic"'));assert.ok(text.includes('rite_type=insular_celtic'));
  assert.ok(text.includes('main_rite=21\norganization=21'));assert.ok(text.includes('name="Barbara\'s Faith"'));
  assert.ok(text.includes('fervor=73.5'));assert.ok(text.includes('religious_head=5'));
  assert.ok(text.includes('changes={ 1283.1.1="campaign history" }'));assert.ok(text.includes('variables={ flag=keep }'));
  assert.ok(text.includes('tenet=tenet_communion'));assert.ok(text.includes('status=core'));assert.ok(text.includes('doctrine=doctrine_monasticism_accepted'));
  assert.ok(!text.includes('tenet_monasticism'));assert.ok(text.includes('traits_lookup={ erudite lifestyle_poet brave }'));
  assert.ok(text.includes('organization_manager={ database={\n11={ faith=11 }'));
  assert.ok(text.includes('clerical_region_manager={ database={} }'));assert.ok(text.includes('saints={}'));
  for(const token of ['donor_only','donor_saint','head_of_rite=900','holder=900','artifacts={ 999 }','levy=888'])assert.ok(!text.includes(token),token);
  assert.ok(text.includes('levy=77'));assert.equal((text.match(/triggered_event=/g)||[]).length,2);
  assert.equal(result.referenceSha256,sha(rb));assert.equal(result.outputVerified,true);assert.equal(result.unchangedSpansVerified,true);
  assert.deepEqual(fs.readFileSync(input),before);assert.deepEqual(fs.readFileSync(ref),rb);
  // Independently verify journal offsets, hashes, and every retained byte range.
  let src=0,dst=0;const original=body(input);
  for(const c of result.changes){
    assert.equal(c.beforeSha256,sha(original.subarray(c.start,c.end)));
    assert.deepEqual(b.subarray(dst,dst+c.start-src),original.subarray(src,c.start));dst+=c.start-src;
    assert.equal(c.afterSha256,sha(b.subarray(dst,dst+c.replacementBytes)));dst+=c.replacementBytes;src=c.end;
  }
  assert.deepEqual(b.subarray(dst),original.subarray(src));
  const inspection=await inspectSave(output);
  assert.equal(inspection.sections.find(s=>s.key==='faiths').occurrences,1);
  assert.equal(inspection.sections.find(s=>s.key==='rites').occurrences,1);
  const second=path.join(folder,'second.ck3');await writeCandidate(input,second,info.gamestateSha256,profile,ref);
  assert.deepEqual(fs.readFileSync(output),fs.readFileSync(second));
});

test('1.20 profile rejects wrong versions, references and province maps before writing',async()=>{
  for(const [sourceText,sourceVersion,refText,refVersion,refFormat,error] of [
    [old,'1.18.4',reference,'1.20.0.3',17,/requires embedded version 1.19.0.6/],
    [old,'1.19.0.6',reference,'1.19.0.6',17,/reference must be version/],
    [old,'1.19.0.6',reference,'1.20.0.4',17,/reference must be version/],
    [old,'1.19.0.6',reference,'1.20.0.3',16,/save_game_version 17/],
    [old,'1.19.0.6',reference.replace('provinces={ 1=', 'provinces={ 2='),'1.20.0.3',17,/matching province IDs/],
  ]) {
    const input=save(sourceText,sourceVersion),ref=save(refText,refVersion,refFormat),info=await inspectSave(input),output=path.join(folder,`rejected-${++serial}.ck3`);
    await assert.rejects(writeCandidate(input,output,info.gamestateSha256,profile,ref),error);assert.ok(!fs.existsSync(output));
  }
  const input=save(old),info=await inspectSave(input),output=path.join(folder,'missing-reference.ck3');
  await assert.rejects(writeCandidate(input,output,info.gamestateSha256,profile),/reference save/);assert.ok(!fs.existsSync(output));
});

test('mixed schemas, duplicate identities and dangling affiliations fail before publication',async()=>{
  const ref=save(reference,'1.20.0.3',17);
  for(const [text,error] of [
    [old+'faiths={ database={} }',/mixed legacy and target schema/],
    [old.replace('faith=11 secret_faith=20','faith=11 rite=11 secret_faith=20'),/mixed legacy and target rite schema/],
    [old.replace('faith=11 secret_faith=20','faith=999 secret_faith=20'),/unresolved faith ID/],
    [old.replace('secret_faith=20','secret_faith=999'),/unresolved secret faith ID/],
    [old.replace('11={ faith_type=orthodox','11=none 11={ faith_type=orthodox'),/duplicate database ID/],
    [old.replace('faith_type=insular_celtic','faith_type=orthodox'),/duplicate stable identity/],
    [old.replace('traits_lookup={ scholar poet brave }','traits_lookup={ scholar erudite poet }'),/duplicate trait identity/],
    [old.replace('holy_site_template=jerusalem','holy_site_template=jerusalem holy_site_type=jerusalem'),/mixed holy-site identity/],
  ]) {
    const input=save(text),info=await inspectSave(input),output=path.join(folder,`ambiguous-${++serial}.ck3`);
    await assert.rejects(writeCandidate(input,output,info.gamestateSha256,profile,ref),error);assert.ok(!fs.existsSync(output));
  }
});

test('camp and estate owners are restored from campaign titles, with unowned slots tombstoned',async()=>{
  const titleText=old.replace('7={ key=b_jerusalem holder=101 }','7={ key=b_jerusalem holder=101 domicile=731 }')
    .replace('9={ key=e_campaign holder=101','9={ key=e_campaign holder=101 domicile=139');
  const records=`domiciles={ database={
  731={ domicile_type=camp owner_title=4294967295 provisions=4533 buildings={ camp_main_02 } province=1 }
  139={ domicile_type=estate owner_title=4294967295 herd=17 buildings={ estate_main_01 } province=1 }
  307={ domicile_type=camp owner_title=4294967295 provisions=44 province=1 }
  1605={ domicile_type=estate owner_title=4294967295 buildings={ estate_main_01 } province=1 }
  1=none
} }`;
  const input=save(titleText+records),ref=save(reference,'1.20.0.3',17),info=await inspectSave(input),output=path.join(folder,'domiciles.ck3');
  const result=await writeCandidate(input,output,info.gamestateSha256,profile,ref),text=body(output).toString();
  assert.ok(text.includes('731={ domicile_type=camp owner_title=7 provisions=4533 buildings={ camp_main_02 } province=1 }'));
  assert.ok(text.includes('139={ domicile_type=estate owner_title=9 herd=17 buildings={ estate_main_01 } province=1 }'));
  assert.ok(text.includes('307=none'));assert.ok(text.includes('1605=none'));assert.ok(text.includes('1=none'));
  assert.equal(result.changes.filter(c=>c.rule==='domicile-owner-title').length,2);
  assert.equal(result.changes.filter(c=>c.rule==='orphan-domicile-tombstone').length,2);
  // Correct reciprocal ownership and resource/building bytes need no repair.
  const valid=save(titleText+records.replace('owner_title=4294967295 provisions=4533','owner_title=7 provisions=4533'));
  const validInfo=await inspectSave(valid),validOutput=path.join(folder,'valid-domiciles.ck3');
  const validResult=await writeCandidate(valid,validOutput,validInfo.gamestateSha256,profile,ref);
  assert.equal(validResult.changes.filter(c=>c.rule==='domicile-owner-title').length,1);
  assert.ok(body(validOutput).toString().includes('731={ domicile_type=camp owner_title=7 provisions=4533 buildings={ camp_main_02 } province=1 }'));
});

test('ambiguous or dangling domicile ownership fails before publishing a save',async()=>{
  const records='domiciles={ database={ 731={ domicile_type=camp owner_title=4294967295 province=1 } } }';
  const owned=old.replace('7={ key=b_jerusalem holder=101 }','7={ key=b_jerusalem holder=101 domicile=731 }');
  const ref=save(reference,'1.20.0.3',17);
  for(const [text,error] of [
    [owned,/missing domicile database/],
    [owned+records.replace('731={','731=none 732={'),/missing domicile 731/],
    [owned.replace('9={ key=e_campaign holder=101','9={ key=e_campaign holder=101 domicile=731')+records,/multiple title owners/],
    [owned+records.replace('owner_title=4294967295','owner_title=9'),/conflicting owner title/],
    [old+records.replace('owner_title=4294967295','owner_title=7'),/no reciprocal title link/],
  ]) {
    const input=save(text),info=await inspectSave(input),output=path.join(folder,`domicile-rejected-${++serial}.ck3`);
    await assert.rejects(writeCandidate(input,output,info.gamestateSha256,profile,ref),error);assert.ok(!fs.existsSync(output));
  }
});
