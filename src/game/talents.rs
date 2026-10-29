use crate::game::colyseus::ColyseusRoom;
use crate::game::types::CharacterInfo;
use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing::info;

pub fn get_priority_talents(voc: &str) -> &'static [&'static str] {
    match voc {
        "knight" => &[
            "k_fury", "k_iron_skin", "k_vitality", "k_blade_mastery", "k_shield_mastery",
            "k_berserk", "k_endurance", "k_bloodlust", "k_juggernaut",
        ],
        "paladin" => &[
            "p_accuracy", "p_divine_power", "p_swiftness", "p_holy_aura", "p_sharpshooter",
            "p_divine_fury", "p_vitality", "p_blessed_steel",
        ],
        "monk" => &[
            "m_fist_mastery", "m_inner_peace", "m_iron_body", "m_zenith", "m_flurry",
            "m_vitality", "m_spirit_surge",
        ],
        "sorcerer" => &[
            "s_magic_surge", "s_arcane_wisdom", "s_elemental_mastery", "s_inferno", "s_spark",
            "s_vitality", "s_mana_shield",
        ],
        "druid" => &[
            "d_earth_mastery", "d_frost_mastery", "d_nature_call", "d_wild_growth", "d_blizzard",
            "d_vitality", "d_mana_shield",
        ],
        _ => &[
            "k_fury", "k_iron_skin", "k_vitality", "k_blade_mastery", "k_shield_mastery",
        ],
    }
}

pub async fn spend_talents_for_party(room: &ColyseusRoom, chars: &[CharacterInfo]) {
    for (slot_idx, c) in chars.iter().enumerate() {
        if slot_idx > 3 {
            break;
        }
        let voc = c.vocation.to_lowercase();
        let nodes = get_priority_talents(&voc);

        let mut points_to_spend = c.level;
        if points_to_spend > 50 {
            points_to_spend = 50;
        }

        let mut spent = 0;
        for &node_id in nodes {
            for _rank in 0..5 {
                if spent >= points_to_spend {
                    break;
                }
                let _ = room.send_action("tree", json!({
                    "slot": slot_idx,
                    "action": "spend",
                    "nodeId": node_id,
                })).await;
                spent += 1;
                sleep(Duration::from_millis(15)).await;
            }
            if spent >= points_to_spend {
                break;
            }
        }

        info!("🌳 [Tree] Slot {} ({} Lv {} {}): Aplicados pontos de talentos nos nós prioritários ({} envios)",
            slot_idx, c.name, c.level, c.vocation, spent);
    }
}
