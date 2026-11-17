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

module.exports = { old, reference };
