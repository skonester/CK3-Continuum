const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const {inspectSave,writeCandidate}=require('../index.cjs');
const profile='experimental-random-regions-1.16.1-to-1.19.0.6';
const folder=fs.mkdtempSync(path.join(__dirname,'../test-output/world-'));
const names='brave calm diligent gregarious just temperate patient ambitious humble generous honest education_stewardship_2';
const sourceCulture='culture_template="old" name="Old" name_list=old ethnicities={ 100=western } color=rgb { 12 34 56 }';
const newCulture='culture_template="new" name="New" name_list=new ethnicities={ 100=asian } color=rgb { 33 66 99 }';
const originalPeople='10={ first_name="Original" birth=1200.1.1 culture=0 family_data={ primary_spouse=20 spouse=20 } landed_data={ domain={ 90 91 92 } } } 20={ first_name="Original spouse" birth=1202.1.1 female=yes culture=0 family_data={ primary_spouse=10 spouse=10 } }';
const old=`date=1243.5.10
religion={ religions={ 0={ template=old_religion faiths={ 0 } } } faiths={ 0={ template=old_faith religion=0 } } holy_sites={ 0={ holy_site_type=old_site } } }
culture_manager={ cultures={ 0={ ${sourceCulture} } 7={ name="Hybrid" heritage=heritage_test } } template_cultures={ 0 } }
landed_titles={ landed_titles={ 90={ key=e_hre holder=10 name="Holy Roman Empire" de_jure_vassals={ 91 } } 91={ key=c_old holder=10 name="Old county" de_jure_liege=90 } 92={ key=b_old holder=10 name="Old barony" de_jure_liege=91 } } }
county_manager={ counties={ c_old={ culture=7 faith=0 development=45 } } }
provinces={ 1={ holding={ type=castle_holding buildings={ { type=keep_01 } } } } }
armies={ regiments={ 0={ origin=1 max=777 } 16777220=none } }
living={ ${originalPeople} }
dead_unprunable={ 30={ first_name="Old dead" } }
characters={ dead_prunable={ 40=none } }
deleted_characters={ 16777266 }
character_lookup={ historical_1=10 historical_deleted=60 }
dynasties={ dynasty_house={ 0=none } dynasties={ 0=none } }
vassal_contracts={ database={ 0=none } }
traits_lookup={ ${names} }
triggered_event={ x=1 }
triggered_event={ x=2 }
`;
const title=(id,key,extra='')=>`${id}={ key=${key} title_name_data={ name="${key}" } ${extra} }`;
const reference=`date=1066.9.15
religion={ religions={ 0={ religion_type=old_religion faiths={ 0 } } 1={ religion_type=new_religion tag="new_religion" family=rf_pagan faiths={ 1 } } } faiths={ 0={ faith_type=old_faith religion=0 } 1={ faith_type=new_faith tag="new_faith" color=rgb { 1 2 3 } religion=1 doctrine=test_a doctrine=test_b holy_sites={ 1 } } } holy_sites={ 0={ holy_site_type=old_site } 1={ holy_site_type=new_site } } }
culture_manager={ cultures={ 0={ ${sourceCulture} } 1={ ${newCulture} head=900 parents={ 0 } acceptance={ 0=99 } variables={ dangerous_reference=900 } } } template_cultures={ 1 0 } }
landed_titles={ landed_titles={ ${title(100,'e_hre','holder=900')} ${title(101,'c_old','holder=900 de_jure_liege=100')} ${title(102,'b_old','holder=900 de_jure_liege=101')}
${title(1,'h_china')} ${title(2,'e_japan')} ${title(3,'k_chrysanthemum_throne')} ${title(4,'k_yongson_throne')} ${title(90,'k_region','holder=900 capital=6')}
${title(6,'c_one','holder=900 de_jure_liege=90 capital=6 history={ 1066.1.1=900 }')} ${title(7,'b_one','holder=900 de_jure_liege=6 capital_barony=yes')}
${title(8,'c_two','holder=901 de_jure_liege=90 capital=8')} ${title(9,'b_two','holder=901 de_jure_liege=8 capital_barony=yes')}
${title(103,'c_mercenary_reference_only','holder=900 mercenary=yes landless=yes')}
${title(104,'c_nf_reference_only','holder=900 domicile=900')}
} }
county_manager={ counties={ c_old={ culture=0 faith=0 development=3 } c_one={ culture=1 faith=1 development=10 } c_two={ culture=1 faith=1 development=12 } } }
provinces={ 1={ holding={ type=castle_holding } } 2={ holding={ type=castle_holding buildings={ { type=keep_01 } } levy=0 garrison=1 income=3 } variables={ owner=900 } } }
armies={ regiments={ 0={ origin=2 max=100 chunks={ { max=100 current=99 army_regiment=999 } } } 1={ origin=2 max=200 source=garrison chunks={ { max=200 current=0 } } } } }
living={ 900={ first_name="Li" birth=1000.1.1 culture=1 } 901={ first_name="Wang" birth=1001.1.1 culture=1 } 902={ first_name="Mei" birth=1002.1.1 female=yes culture=1 } }
dead_unprunable={}
characters={ dead_prunable={} }
`;
function save(name,game,version='1.16.1') {
 const meta=`meta_data={ version="${version}" portraits_version=4 }\n`;
 const header='SAV0100abcdef12'+Buffer.byteLength(meta).toString(16).padStart(8,'0')+'\n';
 const file=path.join(folder,name);fs.writeFileSync(file,header+meta+game);return file;
}
// Independent parser for these controlled fixtures; preserve repeated keys as arrays.
function parse(text) {
 const tokens=text.match(/"(?:\\.|[^"\\])*"|[{}=]|[^\s{}=]+/g)||[];let i=0;
 function value(){const t=tokens[i++];if(t!=='{'){
  if(['rgb','hsv','hsv360'].includes(t)&&tokens[i]==='{')return {tag:t,values:value()};
  return t?.replace(/^"|"$/g,'');
 }const result={};while(tokens[i]!=='}'){
  if(tokens[i+1]==='='){const k=tokens[i];i+=2;(result[k]??=[]).push(value());}else{(result.$list??=[]).push(value());}
  if(i>=tokens.length)throw Error('truncated fixture');
 }i++;return result;}
 const out={};while(i<tokens.length){const k=tokens[i++];assert.equal(tokens[i++],'=');(out[k]??=[]).push(value());}return out;
}
function body(file){const b=fs.readFileSync(file,'utf8');return b.slice(b.indexOf('\n')+1);}
function one(object,key){assert.ok(object[key],`missing ${key}`);return object[key][0];}
test('new regions have fresh kingdoms, reciprocal vassal contracts and preserved original people',async()=>{
 const input=save('old.ck3',old),ref=save('reference.ck3',reference,'1.19.0.6'),output=path.join(folder,'result.ck3');
 const info=await inspectSave(input);const result=await writeCandidate(input,output,info.gamestateSha256,profile,ref);
 const text=body(output);assert.ok(text.includes(originalPeople));assert.equal((text.match(/triggered_event=/g)||[]).length,2);
 assert.ok(text.includes('color=rgb { 33 66 99 }'));assert.ok(text.includes('doctrine=test_a\ndoctrine=test_b'));
 assert.ok(!text.includes('c_mercenary_reference_only'));assert.ok(!text.includes('c_nf_reference_only'));assert.ok(!text.includes('dangerous_reference'));assert.ok(!text.includes('birth=1000.1.1'));
 const game=parse(text),titles=one(one(game,'landed_titles'),'landed_titles'),people=one(game,'living');
 const byKey=Object.fromEntries(Object.entries(titles).map(([id,[t]])=>[one(t,'key'),{id,t}]));
 for(const key of ['h_china','e_japan','k_chrysanthemum_throne','k_yongson_throne'])assert.ok(byKey[key]);
 assert.equal(one(byKey.e_hre.t,'holder'),'10');assert.equal(byKey.e_hre.id,'90');
 const k=byKey.k_region,c1=byKey.c_one,c2=byKey.c_two;const king=one(k.t,'holder');assert.ok(Number(king)>60);
 assert.equal(one(c1.t,'holder'),king);assert.notEqual(one(c2.t,'holder'),king);assert.equal(one(c2.t,'de_facto_liege'),k.id);
 const vassal=one(c2.t,'holder');const contracts=one(one(game,'vassal_contracts'),'database');const contract=Object.entries(contracts).find(([,v])=>typeof v[0]==='object');
 assert.ok(contract);assert.equal(one(contract[1][0],'contract_group'),'feudal_vassal');assert.equal(one(contract[1][0],'liege'),king);assert.equal(one(contract[1][0],'vassal'),vassal);
 assert.ok(one(one(one(people,king),'landed_data'),'vassal_contracts').$list.includes(contract[0]));
 for(const [, [t]] of Object.entries(titles)){if(!t.holder||one(t,'holder')==='10')continue;const h=one(t,'holder');assert.ok(people[h]);assert.ok(Number(h)>60);}
 const culture=one(game,'culture_manager');assert.deepEqual(one(culture,'template_cultures').$list,['8','0']);assert.equal(one(one(one(culture,'cultures'),'7'),'name'),'Hybrid');
 const regiments=one(one(game,'armies'),'regiments');assert.equal(one(one(regiments,'0'),'origin'),'1');assert.equal(one(one(regiments,'0'),'max'),'777');
 const holding=one(one(one(game,'provinces'),'2'),'holding');
 for(const kind of ['levy','garrison']){const id=one(holding,kind);assert.ok(Number(id)>4);assert.equal(one(one(regiments,id),'origin'),'2');}
 assert.equal(one(one(regiments,one(holding,'garrison')),'source'),'garrison');assert.ok(!text.includes('army_regiment=999'));
 assert.equal(result.referenceSha256.length,64);assert.ok(result.changes.some(c=>c.rule==='new-region-characters'));
 const second=path.join(folder,'second.ck3');await writeCandidate(input,second,info.gamestateSha256,profile,ref);assert.deepEqual(fs.readFileSync(output),fs.readFileSync(second));
});
test('missing/wrong reference and ambiguous source identities fail before output publication',async()=>{
 const input=save('invalid-reference-source.ck3',old),info=await inspectSave(input);
 await assert.rejects(writeCandidate(input,path.join(folder,'no-ref.ck3'),info.gamestateSha256,profile),/reference/);
 await assert.rejects(writeCandidate(input,path.join(folder,'old-ref.ck3'),info.gamestateSha256,profile,input),/reference must be version/);
 const dup=save('duplicate-source.ck3',old.replace('7={ name="Hybrid"','7={ culture_template="old" name="Hybrid"'));
 const ref=save('valid-ref.ck3',reference,'1.19.0.6');const di=await inspectSave(dup);
 await assert.rejects(writeCandidate(dup,path.join(folder,'dup-output.ck3'),di.gamestateSha256,profile,ref),/duplicate stable identity/);
 for(const name of ['no-ref.ck3','old-ref.ck3','dup-output.ck3'])assert.ok(!fs.existsSync(path.join(folder,name)));
});

test('reference regiments must originate in the importing province',async()=>{
 const input=save('regiment-source.ck3',old),ref=save('bad-regiment-reference.ck3',reference.replace('origin=2 max=100','origin=999 max=100'),'1.19.0.6');
 const info=await inspectSave(input),output=path.join(folder,'bad-regiment-output.ck3');
 await assert.rejects(writeCandidate(input,output,info.gamestateSha256,profile,ref),/regiment.*origin/);
 assert.ok(!fs.existsSync(output));
});
