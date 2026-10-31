use crate::game::catalog::get_hunt_facts;
use crate::game::colyseus::ColyseusRoom;
use serde_json::json;
use tracing::info;

pub struct SafeBoss {
    pub id: &'static str,
    pub name: &'static str,
    pub min_level: u32,
}

pub static SAFE_BOSS_CATALOG: &[SafeBoss] = &[
    SafeBoss { id: "shadowpelt", name: "Shadowpelt", min_level: 5 },
    SafeBoss { id: "darkfang", name: "Darkfang", min_level: 15 },
    SafeBoss { id: "bloodback", name: "Bloodback", min_level: 15 },
    SafeBoss { id: "black_vixen", name: "Black Vixen", min_level: 20 },
    SafeBoss { id: "sharpclaw", name: "Sharpclaw", min_level: 20 },
    SafeBoss { id: "utua_stone_sting", name: "Utua Stone Sting", min_level: 35 },
    SafeBoss { id: "amenef_the_burning", name: "Amenef the Burning", min_level: 50 },
    SafeBoss { id: "brokul", name: "Brokul", min_level: 50 },
    SafeBoss { id: "scarlett", name: "Scarlett Etzel", min_level: 80 },
    SafeBoss { id: "ahau", name: "Ahau", min_level: 60 },
    SafeBoss { id: "kusuma", name: "Kusuma", min_level: 60 },
    SafeBoss { id: "brain_head", name: "Brain Head", min_level: 70 },
    SafeBoss { id: "neferi_the_spy", name: "Neferi the Spy", min_level: 70 },
    SafeBoss { id: "drume", name: "Drume", min_level: 90 },
    SafeBoss { id: "oberon", name: "Grand Master Oberon", min_level: 100 },
];

pub async fn run_safe_bosses(room: &ColyseusRoom, player_level: u32, recent_deaths: usize) -> bool {
    if recent_deaths > 0 || player_level < 120 {
        return false;
    }

    for b in SAFE_BOSS_CATALOG {
        if player_level >= b.min_level * 2 && player_level >= b.min_level + 50 {
            let _ = room.send_action("boss", json!({
                "bossId": b.id,
                "fromCity": false,
            })).await;
            info!("⚔️ [Auto-Boss] Iniciando combate com boss seguro: {} (Lv {})", b.name, b.min_level);
            return true;
        }
    }
    false
}

pub async fn run_daily_rewards(room: &ColyseusRoom) {
    let _ = room.send_action("dailyclaim", json!({})).await;
    tokio::time::sleep(std::time::Duration::from_millis(300 + (rand::random::<u64>() % 300))).await;
    let _ = room.send_action("bpclaim", json!({})).await;
    tokio::time::sleep(std::time::Duration::from_millis(300 + (rand::random::<u64>() % 300))).await;
    let _ = room.send_action("reward", json!({ "action": "collectall" })).await;
    tokio::time::sleep(std::time::Duration::from_millis(300 + (rand::random::<u64>() % 300))).await;
    let _ = room.send_action("sellreward", json!({})).await;
    info!("🎁 [Daily] Resgatadas recompensas diárias, missões e baús com pacing humano");
}

pub async fn run_auto_prey(room: &ColyseusRoom, current_hunt_id: &str) {
    let facts = match get_hunt_facts(current_hunt_id) {
        Some(f) => f,
        None => return,
    };
    if facts.monsters.is_empty() {
        return;
    }
    let target_mob = &facts.monsters[0];
    if target_mob.is_empty() {
        return;
    }

    let _ = room.send_action("prey", json!({
        "slot": 0,
        "action": "pick",
        "monsterKey": target_mob,
    })).await;
    let _ = room.send_action("prey", json!({
        "slot": 0,
        "action": "lock",
    })).await;
    info!("🎯 [Prey] Presa vinculada ao monstro '{}' (Hunt: {})", target_mob, facts.name);
}

pub async fn run_guild_tasks(room: &ColyseusRoom) {
    let _ = room.send_action("guildtaskflush", json!({})).await;
    info!("🛡️ [Guild] Tarefas sincronizadas com a guilda");
}

pub async fn run_charms_auto(room: &ColyseusRoom, current_hunt_id: &str) {
    let facts = match get_hunt_facts(current_hunt_id) {
        Some(f) => f,
        None => return,
    };
    for (i, mob) in facts.monsters.iter().take(3).enumerate() {
        let _ = room.send_action("charmassign", json!({
            "charmId": i,
            "monsterKey": mob,
        })).await;
    }
}

pub async fn run_dungeon_auto(room: &ColyseusRoom) {
    let _ = room.send_action("dgAuto", json!({})).await;
    info!("🏰 [Dungeon] Automação periódica de dungeon despachada");
}

pub async fn run_codex_auto_sync(room: &ColyseusRoom) {
    let _ = room.send_action("codexauto", json!({ "on": true })).await;
}
