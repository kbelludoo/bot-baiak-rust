use crate::game::catalog::{get_all_hunts, get_hunt_facts, HuntInfo};

#[derive(Debug, Clone)]
pub struct HuntSimulation {
    pub id: String,
    pub name: String,
    pub ttk: f64,
    pub kills_per_hour: f64,
    pub gold_per_hour: f64,
    pub exp_per_hour: f64,
    pub can_tank: bool,
    pub balance_score: f64,
    pub min_level: u32,
}

pub fn estimate_dps(level: u32, party: bool) -> f64 {
    let lvl = if level < 1 { 1.0 } else { level as f64 };
    let n = if party { 1.25 } else { 1.0 };
    (0.5f64).max(lvl * (0.24 + 0.12 * 3.0 + 0.08 * 3.0) * n)
}

pub fn simulate_hunt(hunt_id: &str, level: u32) -> Option<HuntSimulation> {
    let facts = get_hunt_facts(hunt_id)?;
    if facts.avg_hp <= 0.0 {
        return None;
    }

    let dps = estimate_dps(level, true);
    let ttk = facts.avg_hp / dps.max(1.0);
    let mut spawn_s = facts.spawn_ms / 1000.0;
    if spawn_s < 0.2 {
        spawn_s = 0.2;
    }
    let alive = (facts.max_alive as f64).max(1.0);
    let kills_h = (alive / (ttk + spawn_s).max(spawn_s)) * 3600.0;

    let prior = if hunt_id == "refiner-cave" { 2.2 } else { 1.0 };
    let gold_kill = facts.gold_kill * prior;
    let gold_h = kills_h * gold_kill;
    let exp_h = kills_h * facts.avg_exp;

    let incoming = facts.avg_dmg * alive / ttk.max(0.25);
    let sustain = (level as f64) * 18.0;
    let can_tank = incoming < sustain * 5.0 || facts.avg_hp <= (level as f64) * 40.0;

    let score = exp_h + gold_h * 1.5;

    Some(HuntSimulation {
        id: facts.id.clone(),
        name: facts.name.clone(),
        ttk,
        kills_per_hour: kills_h,
        gold_per_hour: gold_h,
        exp_per_hour: exp_h,
        can_tank,
        balance_score: score,
        min_level: facts.min_level,
    })
}

pub fn find_optimal_hunt(level: u32, focus_gold: bool) -> Option<String> {
    let all = get_all_hunts();
    let mut valid = Vec::new();

    for h in all {
        if h.min_level > level {
            continue;
        }
        if let Some(sim) = simulate_hunt(&h.id, level) {
            if sim.can_tank && sim.ttk < 45.0 {
                valid.push(sim);
            }
        }
    }

    if valid.is_empty() {
        return None;
    }

    if focus_gold {
        valid.sort_by(|a, b| b.gold_per_hour.partial_cmp(&a.gold_per_hour).unwrap());
    } else {
        valid.sort_by(|a, b| b.balance_score.partial_cmp(&a.balance_score).unwrap());
    }

    valid.first().map(|s| s.id.clone())
}
