use crate::game::colyseus::ColyseusRoom;
use crate::game::hunt_elements::get_hunt_element_profile;
use crate::game::trpc::TRPCClient;
use crate::game::types::CharacterInfo;
use serde_json::json;
use tracing::info;

pub fn get_recommended_rotation(voc: &str, level: u32, hunt_id: &str) -> Vec<String> {
    let v = voc.to_lowercase();
    let profile = get_hunt_element_profile(hunt_id);

    match v.as_str() {
        "knight" => {
            if level >= 90 {
                vec!["exori gran".into(), "exori".into(), "exori min".into(), "exori mas".into()]
            } else if level >= 70 {
                vec!["exori".into(), "exori min".into(), "exori mas".into(), "exori scu".into()]
            } else if level >= 35 {
                vec!["exori".into(), "exori mas".into(), "exori scu".into(), "exori ico".into()]
            } else {
                vec!["exori ico".into(), "exori ico scu".into(), "exori hur".into(), "exori".into()]
            }
        }
        "monk" => {
            if level >= 90 {
                vec!["exori gran mas pug".into(), "exori mas pug".into(), "exori mas nia".into(), "exori med pug".into()]
            } else if level >= 60 {
                vec!["exori mas pug".into(), "exori mas nia".into(), "exori amp pug".into(), "exori pug".into()]
            } else {
                vec!["exori pug".into(), "exori nia".into(), "exori amp pug".into(), "exori mas pug".into()]
            }
        }
        "paladin" => {
            if profile.has_resistance("holy") {
                if level >= 70 {
                    vec!["exori gran con".into(), "exori dir moe".into(), "exori con".into(), "exori".into()]
                } else {
                    vec!["exori con".into(), "exori".into(), "exori hur".into(), "exori min".into()]
                }
            } else if level >= 70 {
                vec!["exevo mas san".into(), "exori dir san".into(), "exori dir moe".into(), "exori gran con".into()]
            } else if level >= 50 {
                vec!["exevo mas san".into(), "exori san".into(), "exori con".into(), "utori san".into()]
            } else {
                vec!["exori con".into(), "exori san".into(), "utori san".into(), "exori".into()]
            }
        }
        "sorcerer" => {
            let use_energy = profile.has_resistance("fire") || (profile.has_weakness("energy") && !profile.has_weakness("fire"));
            if use_energy {
                if level >= 90 {
                    vec!["exevo gran mas vis".into(), "exori max vis".into(), "exori gran vis".into(), "exevo vis hur".into()]
                } else {
                    vec!["exevo vis hur".into(), "exori vis".into(), "exori mort".into(), "exori flam".into()]
                }
            } else if level >= 90 {
                vec!["exevo gran mas flam".into(), "exori max flam".into(), "exori gran flam".into(), "exevo vis hur".into()]
            } else {
                vec!["exevo flam hur".into(), "exori flam".into(), "exori vis".into(), "exori mort".into()]
            }
        }
        "druid" => {
            let use_earth = profile.has_resistance("ice") || (profile.has_weakness("earth") && !profile.has_weakness("ice"));
            if use_earth {
                if level >= 90 {
                    vec!["exevo gran mas tera".into(), "exori max tera".into(), "exori gran tera".into(), "exevo tera hur".into()]
                } else {
                    vec!["exevo tera hur".into(), "exori tera".into(), "exori infir tera".into(), "exori frigo".into()]
                }
            } else if level >= 90 {
                vec!["exevo gran mas frigo".into(), "exori max frigo".into(), "exori gran frigo".into(), "exevo frigo hur".into()]
            } else {
                vec!["exevo frigo hur".into(), "exori frigo".into(), "exori tera".into(), "exori infir tera".into()]
            }
        }
        _ => vec![],
    }
}

pub fn get_recommended_heal(voc: &str, level: u32) -> &'static str {
    let v = voc.to_lowercase();
    match v.as_str() {
        "knight" => {
            if level >= 80 { "exura gran ico" } else if level >= 40 { "exura med ico" } else { "exura ico" }
        }
        "paladin" => {
            if level >= 70 { "exura gran san" } else if level >= 35 { "exura san" } else { "exura" }
        }
        "sorcerer" => {
            if level >= 80 { "exura vita" } else if level >= 35 { "exura gran" } else { "exura" }
        }
        "druid" => {
            if level >= 80 { "exura vita" } else if level >= 50 { "exura gran mas res" } else { "exura gran" }
        }
        "monk" => {
            if level >= 70 { "exura med pug" } else { "exura pug" }
        }
        _ => "exura",
    }
}

pub fn get_recommended_potions(level: u32) -> (&'static str, &'static str) {
    let hp = if level >= 130 {
        "supreme health potion"
    } else if level >= 80 {
        "ultimate health potion"
    } else if level >= 50 {
        "great health potion"
    } else {
        "health potion"
    };

    let mp = if level >= 130 {
        "distilled ultimate mana potion"
    } else if level >= 80 {
        "ultimate mana potion"
    } else if level >= 50 {
        "great mana potion"
    } else {
        "mana potion"
    };

    (hp, mp)
}

pub async fn sync_party_auto_config(
    trpc: &TRPCClient,
    room: Option<&ColyseusRoom>,
    chars: &[CharacterInfo],
    hunt_id: &str,
) {
    for (idx, c) in chars.iter().enumerate() {
        if idx > 3 {
            break;
        }
        let rot = get_recommended_rotation(&c.vocation, c.level, hunt_id);
        let heal = get_recommended_heal(&c.vocation, c.level);
        let (hp_pot, mp_pot) = get_recommended_potions(c.level);

        // Via Colyseus se sala ativa
        if let Some(r) = room {
            let _ = r.send_action("rotation", json!({
                "slot": idx,
                "rotation": rot,
            })).await;

            let _ = r.send_action("healspell", json!({
                "slot": idx,
                "spell": heal,
            })).await;

            let _ = r.send_action("potions", json!({
                "slot": idx,
                "hpPotion": hp_pot,
                "manaPotion": mp_pot,
            })).await;
        }

        // Também garantir persistência via tRPC
        let payload = json!({
            "characterId": c.id,
            "rotation": rot,
            "healSpell": heal,
            "hpPotion": hp_pot,
            "manaPotion": mp_pot,
        });
        let _ = trpc.mutate("characters.autoConfig", payload).await;
        info!("🧙 [AutoConfig] Slot {} ({}, Lv {}): Rotação Max DPS & Cura BiS aplicadas para cave [{}]", idx, c.name, c.level, hunt_id);
    }
}
