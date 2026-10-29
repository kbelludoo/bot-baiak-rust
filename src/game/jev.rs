use regex::Regex;
use std::sync::OnceLock;

static RE_SOUL: OnceLock<Regex> = OnceLock::new();
static RE_INFERNIARCH: OnceLock<Regex> = OnceLock::new();
static RE_FALCON: OnceLock<Regex> = OnceLock::new();
static RE_NAGA: OnceLock<Regex> = OnceLock::new();
static RE_GHOST_CHEST: OnceLock<Regex> = OnceLock::new();
static RE_ELDRITCH: OnceLock<Regex> = OnceLock::new();
static RE_COBRA: OnceLock<Regex> = OnceLock::new();
static RE_LION: OnceLock<Regex> = OnceLock::new();
static RE_GNOME: OnceLock<Regex> = OnceLock::new();
static RE_PLASMA: OnceLock<Regex> = OnceLock::new();
static RE_PRISMATIC: OnceLock<Regex> = OnceLock::new();

fn init_regexes() {
    let _ = RE_SOUL.get_or_init(|| Regex::new(r"(?i)(soulbleeder|soulhexer|soulshredder|soultainter|soulcutter|soulbiter|soulcrusher|soulpiercer|soulful\s+legs|ring\s+of\s+souls|\bsoul\b)").unwrap());
    let _ = RE_INFERNIARCH.get_or_init(|| Regex::new(r"(?i)inferniarch").unwrap());
    let _ = RE_FALCON.get_or_init(|| Regex::new(r"(?i)falcon").unwrap());
    let _ = RE_NAGA.get_or_init(|| Regex::new(r"(?i)naga").unwrap());
    let _ = RE_GHOST_CHEST.get_or_init(|| Regex::new(r"(?i)ghost\s+chestplate").unwrap());
    let _ = RE_ELDRITCH.get_or_init(|| Regex::new(r"(?i)(eldritch|fabulous\s+legs)").unwrap());
    let _ = RE_COBRA.get_or_init(|| Regex::new(r"(?i)(cobra|death\s+oyoroi|toga\s+mortis)").unwrap());
    let _ = RE_LION.get_or_init(|| Regex::new(r"(?i)(lion|dark\s+whispers)").unwrap());
    let _ = RE_GNOME.get_or_init(|| Regex::new(r"(?i)gnome").unwrap());
    let _ = RE_PLASMA.get_or_init(|| Regex::new(r"(?i)(collar\s+of\s+(?:blue|red)\s+plasma|ring\s+of\s+(?:blue|red)\s+plasma)").unwrap());
    let _ = RE_PRISMATIC.get_or_init(|| Regex::new(r"(?i)(prismatic|ornate|depth|yalahari|depth\s+galea|alicorn)").unwrap());
}

#[derive(Debug, Clone, Default)]
pub struct ItemAttribute {
    pub id: i64,
    pub level: i64,
}

pub fn estimate_item_market_value_coins(name: &str, tier: i64, up_level: i64, attrs: &[ItemAttribute]) -> i64 {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return 0;
    }
    init_regexes();

    let mut base_value = 40;

    if RE_SOUL.get().unwrap().is_match(trimmed) {
        base_value = 350;
    } else if RE_INFERNIARCH.get().unwrap().is_match(trimmed) {
        base_value = 250;
    } else if RE_FALCON.get().unwrap().is_match(trimmed) {
        base_value = 220;
    } else if RE_NAGA.get().unwrap().is_match(trimmed) {
        base_value = 180;
    } else if RE_GHOST_CHEST.get().unwrap().is_match(trimmed) {
        base_value = 160;
    } else if RE_ELDRITCH.get().unwrap().is_match(trimmed) {
        base_value = 150;
    } else if RE_COBRA.get().unwrap().is_match(trimmed) {
        base_value = 140;
    } else if RE_LION.get().unwrap().is_match(trimmed) {
        base_value = 130;
    } else if RE_GNOME.get().unwrap().is_match(trimmed) {
        base_value = 120;
    } else if RE_PLASMA.get().unwrap().is_match(trimmed) {
        base_value = 100;
    } else if RE_PRISMATIC.get().unwrap().is_match(trimmed) {
        base_value = 70;
    }

    let tier_bonus = match tier {
        1 => 40,
        2 => 120,
        3 => 300,
        4 => 700,
        t if t >= 5 => 1500 + (t - 5) * 1000,
        _ => 0,
    };

    let mut attr_bonus = 0;
    for a in attrs {
        if a.level >= 10 {
            attr_bonus += 100;
        } else if a.level >= 8 {
            attr_bonus += 50;
        } else if a.level >= 6 {
            attr_bonus += 20;
        }
    }

    let up_bonus = match up_level {
        lvl if lvl >= 11 => 500,
        lvl if lvl >= 9 => 200,
        lvl if lvl >= 7 => 80,
        lvl if lvl >= 5 => 30,
        lvl if lvl >= 3 => 10,
        _ => 0,
    };

    base_value + tier_bonus + attr_bonus + up_bonus
}
