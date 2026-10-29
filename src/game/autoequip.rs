use crate::game::colyseus::ColyseusRoom;
use crate::game::trpc::TRPCClient;
use crate::game::types::CharacterInfo;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tier: f64,
    #[serde(default)]
    pub ftier: f64,
    #[serde(default)]
    pub up_level: f64,
    pub uid: Option<Value>,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub location: String,
    pub char_id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub atk: f64,
    #[serde(default)]
    pub element_atk: f64,
    #[serde(default)]
    pub def: f64,
    #[serde(default)]
    pub arm: f64,
    #[serde(default)]
    pub level: f64,
    #[serde(default)]
    pub two_handed: bool,
    #[serde(default)]
    pub vocs: Vec<String>,
}

fn calc_item_score(it: &InventoryItem, cat: &CatalogItem) -> f64 {
    let mut base = cat.atk + cat.element_atk;
    if cat.def > base {
        base = cat.def;
    }
    if cat.arm > base {
        base = cat.arm;
    }
    base * (1.0 + it.tier * 0.5 + it.up_level * 0.1) + it.ftier * 5.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ItemsSummaryResponse {
    #[serde(default)]
    items: Vec<InventoryItem>,
    #[serde(default)]
    catalog: HashMap<String, CatalogItem>,
}

pub async fn run_auto_equip(
    trpc: &TRPCClient,
    room: &ColyseusRoom,
    char: &CharacterInfo,
    char_slot: usize,
    min_tier: u32,
) -> usize {
    let summary_val = match trpc.query("items.summary", None).await {
        Ok(v) => v,
        Err(_) => return 0,
    };

    let summary: ItemsSummaryResponse = serde_json::from_value(summary_val).unwrap_or_default();
    let char_id_str = char.id.clone();

    let mut equipped_map: HashMap<String, f64> = HashMap::new();
    let mut equipped_cat_map: HashMap<String, CatalogItem> = HashMap::new();

    for it in &summary.items {
        if it.location == "equipped" {
            if let Some(cid) = &it.char_id {
                let s = if let Some(cs) = cid.as_str() {
                    cs.to_string()
                } else if let Some(n) = cid.as_i64() {
                    n.to_string()
                } else {
                    cid.to_string()
                };
                if s != char_id_str {
                    continue;
                }
            }
            if let Some(cat) = summary.catalog.get(&it.name) {
                if !cat.slot.is_empty() {
                    equipped_map.insert(cat.slot.clone(), calc_item_score(it, cat));
                    equipped_cat_map.insert(cat.slot.clone(), cat.clone());
                }
            }
        }
    }

    struct UpgradeCandidate {
        item: InventoryItem,
        cat: CatalogItem,
        score: f64,
    }

    let mut best_by_slot: HashMap<String, UpgradeCandidate> = HashMap::new();

    for it in &summary.items {
        if it.location == "equipped" || it.hash.is_empty() {
            continue;
        }
        if let Some(cid) = &it.char_id {
            let s = if let Some(cs) = cid.as_str() {
                cs.to_string()
            } else if let Some(n) = cid.as_i64() {
                n.to_string()
            } else {
                cid.to_string()
            };
            if !s.is_empty() && s != "null" && s != char_id_str {
                continue;
            }
        }
        if (it.tier as u32) < min_tier {
            continue;
        }

        let cat = match summary.catalog.get(&it.name) {
            Some(c) if !c.slot.is_empty() => c,
            _ => continue,
        };

        if cat.level > char.level as f64 {
            continue;
        }

        if !cat.vocs.is_empty() && !cat.vocs.iter().any(|v| v.eq_ignore_ascii_case(&char.vocation)) {
            continue;
        }

        let score = calc_item_score(it, cat);
        let cur_score = equipped_map.get(&cat.slot).copied().unwrap_or(0.0);

        if score > cur_score {
            let replace = match best_by_slot.get(&cat.slot) {
                Some(existing) => score > existing.score,
                None => true,
            };
            if replace {
                best_by_slot.insert(cat.slot.clone(), UpgradeCandidate {
                    item: it.clone(),
                    cat: cat.clone(),
                    score,
                });
            }
        }
    }

    // Regra de Duas Mãos
    if let Some(w) = best_by_slot.get("weapon") {
        if w.cat.two_handed {
            best_by_slot.remove("shield");
        }
    } else if let Some(cur_w) = equipped_cat_map.get("weapon") {
        if cur_w.two_handed {
            best_by_slot.remove("shield");
        }
    }

    let mut slots_to_equip = Vec::new();
    if best_by_slot.contains_key("weapon") {
        slots_to_equip.push("weapon".to_string());
    }
    for slot in best_by_slot.keys() {
        if slot != "weapon" {
            slots_to_equip.push(slot.clone());
        }
    }

    let mut equipped_count = 0;
    for slot in slots_to_equip {
        if let Some(cand) = best_by_slot.get(&slot) {
            let cur_score = equipped_map.get(&slot).copied().unwrap_or(0.0);
            let _ = room.send_action("equip", json!({
                "uid": cand.item.uid,
                "to": char_slot,
                "slot": slot,
                "hash": cand.item.hash,
            })).await;

            info!("🛡️ [Auto-Equip] Slot {} ({}): Equipado {} (Tier {}) no slot '{}' (Score {:.0} > {:.0})",
                char_slot, char.name, cand.item.name, cand.item.tier as u32, slot, cand.score, cur_score);
            equipped_count += 1;
        }
    }

    equipped_count
}
