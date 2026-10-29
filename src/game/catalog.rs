use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntInfo {
    pub id: String,
    pub name: String,
    pub min_level: u32,
    #[serde(default)]
    pub monsters: Vec<String>,
    #[serde(default)]
    pub max_alive: u32,
    #[serde(default)]
    pub spawn_ms: f64,
    #[serde(default)]
    pub avg_hp: f64,
    #[serde(default)]
    pub avg_exp: f64,
    #[serde(default)]
    pub gold_kill: f64,
    #[serde(default)]
    pub avg_dmg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineData {
    pub hunts: Vec<HuntInfo>,
    #[serde(default)]
    pub priors: HashMap<String, f64>,
}

static ENGINE_DATA: OnceLock<EngineData> = OnceLock::new();
static HUNTS_BY_ID: OnceLock<HashMap<String, HuntInfo>> = OnceLock::new();

pub static PREFERRED_HUNT_TABLE: &[(u32, &str)] = &[
    (1, "troll-cave"), (8, "elf-lair"), (9, "amazon-camp"), (10, "minotaur"), (15, "cyclopolis"),
    (30, "refiner-cave"), (50, "giant-spider"), (60, "glooth-cave"), (80, "dragon-lair"),
    (90, "werebadge-cave"), (100, "hydra-cave"), (130, "wyrm-cave"), (140, "werehyaena-cave"),
    (150, "asura-lair"), (170, "wereliones-cave"), (180, "draken-lair"), (190, "cobra-cave"),
    (200, "undeadragon-lair"), (210, "falcon"), (220, "grimeleech-cave"), (240, "choking-cave"),
    (250, "guzzlemaw-cave"), (270, "lionknight-cave"), (290, "megadragon-cave"), (300, "naga-lair"),
    (310, "moonstone-crater"), (350, "gazer-lair"), (370, "forbidden-gardens"), (420, "asura-citadel"),
    (450, "inferniarch-lair"), (500, "livraria-cave"), (510, "thalassara-surroundings"), (610, "lavafungos-cave"),
    (700, "crypt-cave"), (800, "bonyseadevil-cave"), (1500, "draklightsource-cave"), (2000, "radiant-skyhold"),
    (2500, "radiant-skyhold-devoted"),
];

fn get_engine() -> &'static EngineData {
    ENGINE_DATA.get_or_init(|| {
        let raw = include_str!("engine_hunts.json");
        serde_json::from_str(raw).expect("Falha ao parsear engine_hunts.json")
    })
}

fn get_hunts_map() -> &'static HashMap<String, HuntInfo> {
    HUNTS_BY_ID.get_or_init(|| {
        let eng = get_engine();
        let mut map = HashMap::with_capacity(eng.hunts.len());
        for h in &eng.hunts {
            map.insert(h.id.clone(), h.clone());
        }
        map
    })
}

pub fn get_hunt_facts(id: &str) -> Option<&'static HuntInfo> {
    get_hunts_map().get(id)
}

pub fn get_all_hunts() -> &'static [HuntInfo] {
    &get_engine().hunts
}

#[derive(Clone, Default)]
pub struct Catalog;

impl Catalog {
    pub fn select_best_hunt(&self, level: u32) -> String {
        let mut best = "troll-cave";
        for (min_lvl, hunt_id) in PREFERRED_HUNT_TABLE {
            if level >= *min_lvl {
                best = *hunt_id;
            } else {
                break;
            }
        }
        best.to_string()
    }

    pub fn has_route(hunt_id: &str) -> bool {
        get_hunts_map().contains_key(hunt_id)
    }
}
