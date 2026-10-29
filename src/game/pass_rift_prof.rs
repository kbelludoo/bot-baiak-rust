use crate::game::colyseus::ColyseusRoom;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub struct PassManager {
    pub pending_tiers: Arc<RwLock<Vec<i64>>>,
    pub open: AtomicBool,
    pub premium: AtomicBool,
}

impl Default for PassManager {
    fn default() -> Self {
        Self {
            pending_tiers: Arc::new(RwLock::new(Vec::new())),
            open: AtomicBool::new(false),
            premium: AtomicBool::new(false),
        }
    }
}

impl PassManager {
    pub async fn handle_bp_status(&self, payload: &Value) {
        if let Some(open) = payload.get("open").and_then(|v| v.as_bool()) {
            self.open.store(open, Ordering::Relaxed);
        }
        if let Some(prem) = payload.get("premium").and_then(|v| v.as_bool()) {
            self.premium.store(prem, Ordering::Relaxed);
        }

        let mut tiers = Vec::new();
        if let Some(arr) = payload.get("pendingTiers").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(n) = v.as_i64() {
                    tiers.push(n);
                }
            }
        }
        if self.premium.load(Ordering::Relaxed) {
            if let Some(arr) = payload.get("pendingPremium").and_then(|v| v.as_array()) {
                for v in arr {
                    if let Some(n) = v.as_i64() {
                        tiers.push(n);
                    }
                }
            }
        }

        tiers.sort_unstable();
        tiers.dedup();

        if !tiers.is_empty() {
            info!("🎫 [Passe de Temporada] {} degraus pendentes para resgate: {:?}", tiers.len(), tiers);
        }
        *self.pending_tiers.write().await = tiers;
    }

    pub async fn check_and_claim(&self, room: &ColyseusRoom) {
        let _ = room.send_action("dailystatus", json!({})).await;
        let _ = room.send_action("dailyclaim", json!({})).await;
        let _ = room.send_action("bpstatus", json!({ "fresh": true })).await;
        let _ = room.send_action("bpdeliver", json!({})).await;

        let mut lock = self.pending_tiers.write().await;
        let tiers_to_claim = lock.clone();
        lock.clear();

        let empty_tiers: Vec<i64> = Vec::new();
        let _ = room.send_action("bpclaim", json!({ "tiers": empty_tiers })).await;
        if !tiers_to_claim.is_empty() {
            let _ = room.send_action("bpclaim", json!({ "tiers": tiers_to_claim })).await;
            info!("🎁 [Passe de Temporada] Reivindicadas recompensas dos degraus: {:?}", tiers_to_claim);
        }
    }
}

pub struct RiftManager {
    pub nightmare_unlocked: AtomicBool,
    pub hell_unlocked: AtomicBool,
}

impl Default for RiftManager {
    fn default() -> Self {
        Self {
            nightmare_unlocked: AtomicBool::new(false),
            hell_unlocked: AtomicBool::new(false),
        }
    }
}

impl RiftManager {
    pub fn handle_progression(&self, payload: &Value) -> bool {
        let mut new_unlock = false;
        let points = payload.get("points").and_then(|v| v.as_i64()).unwrap_or(0);
        let nm_points = payload.get("nightmarePoints").and_then(|v| v.as_i64()).unwrap_or(0);

        if let Some(unl) = payload.get("unlocked") {
            if unl.get("nm").and_then(|v| v.as_bool()).unwrap_or(false) && !self.nightmare_unlocked.load(Ordering::Relaxed) {
                self.nightmare_unlocked.store(true, Ordering::SeqCst);
                new_unlock = true;
                info!("⚡ [Progression] TIER NIGHTMARE DESBLOQUEADO!");
            }
            if unl.get("hl").and_then(|v| v.as_bool()).unwrap_or(false) && !self.hell_unlocked.load(Ordering::Relaxed) {
                self.hell_unlocked.store(true, Ordering::SeqCst);
                new_unlock = true;
                info!("🔥 [Progression] TIER HELL DESBLOQUEADO!");
            }
        }

        if points >= 7500 && !self.nightmare_unlocked.load(Ordering::Relaxed) {
            self.nightmare_unlocked.store(true, Ordering::SeqCst);
            new_unlock = true;
            info!("⚡ [Progression] {} pontos alcançados! TIER NIGHTMARE DESBLOQUEADO!", points);
        }

        if (nm_points >= 7500 || points >= 15000) && !self.hell_unlocked.load(Ordering::Relaxed) {
            self.hell_unlocked.store(true, Ordering::SeqCst);
            new_unlock = true;
            info!("🔥 [Progression] TIER HELL DESBLOQUEADO!");
        }

        new_unlock
    }

    pub fn get_tier(&self) -> &'static str {
        if self.hell_unlocked.load(Ordering::Relaxed) {
            "hl"
        } else if self.nightmare_unlocked.load(Ordering::Relaxed) {
            "nm"
        } else {
            ""
        }
    }
}
