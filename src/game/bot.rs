use crate::engine::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use crate::engine::stamina::{StaminaConfig, StaminaController};
use crate::game::auction::run_auction_cycle;
use crate::game::autoconfig::sync_party_auto_config;
use crate::game::autoequip::run_auto_equip;
use crate::game::catalog::Catalog;
use crate::game::colyseus::ColyseusRoom;
use crate::game::extras::{run_auto_prey, run_charms_auto, run_codex_auto_sync, run_daily_rewards, run_dungeon_auto, run_guild_tasks, run_safe_bosses};
use crate::game::fingerprint::build_fingerprint;
use crate::game::pass_rift_prof::{PassManager, RiftManager};
use crate::game::sim::find_optimal_hunt;
use crate::game::talents::spend_talents_for_party;
use crate::game::trpc::TRPCClient;
use crate::game::types::{AccountConfig, CharacterInfo};
use crate::subsystem::dailies::DailyManager;
use crate::subsystem::inventory::InventoryManager;
use crate::subsystem::potions::PotionManager;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

pub struct BotInstance {
    pub config: AccountConfig,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub stamina: Arc<StaminaController>,
    pub inventory: Arc<InventoryManager>,
    pub dailies: Arc<DailyManager>,
    pub potions: Arc<PotionManager>,
    pub pass_mgr: Arc<PassManager>,
    pub rift_mgr: Arc<RiftManager>,
    pub catalog: Catalog,
    pub trpc: Arc<TRPCClient>,
    pub colyseus_room: Arc<RwLock<Option<ColyseusRoom>>>,
    pub character: Arc<RwLock<Option<CharacterInfo>>>,
    pub all_characters: Arc<RwLock<Vec<CharacterInfo>>>,
    pub current_hunt_id: Arc<RwLock<String>>,
    pub turnstile_token: Arc<RwLock<String>>,
    pub kills: AtomicU64,
    pub gold: AtomicU64,
    pub exp: AtomicU64,
    pub current_level: AtomicU32,
    pub death_count: Arc<AtomicU32>,
    pub running: AtomicBool,
}

impl BotInstance {
    pub fn new(config: AccountConfig) -> Self {
        let cb_cfg = CircuitBreakerConfig::default();
        let cb = Arc::new(CircuitBreaker::new(&config.id, cb_cfg));
        let stam = Arc::new(StaminaController::new(StaminaConfig::default()));
        let trpc = Arc::new(TRPCClient::new(&config.credentials.token, &config.credentials.origin));
        let initial_hunt = if !config.settings.hunt_target.is_empty() {
            config.settings.hunt_target.clone()
        } else {
            "refiner-cave".to_string()
        };

        Self {
            config,
            circuit_breaker: cb,
            stamina: stam,
            inventory: Arc::new(InventoryManager::default()),
            dailies: Arc::new(DailyManager::default()),
            potions: Arc::new(PotionManager::default()),
            pass_mgr: Arc::new(PassManager::default()),
            rift_mgr: Arc::new(RiftManager::default()),
            catalog: Catalog::default(),
            trpc,
            colyseus_room: Arc::new(RwLock::new(None)),
            character: Arc::new(RwLock::new(None)),
            all_characters: Arc::new(RwLock::new(Vec::new())),
            current_hunt_id: Arc::new(RwLock::new(initial_hunt)),
            turnstile_token: Arc::new(RwLock::new(String::new())),
            kills: AtomicU64::new(0),
            gold: AtomicU64::new(0),
            exp: AtomicU64::new(0),
            current_level: AtomicU32::new(1),
            death_count: Arc::new(AtomicU32::new(0)),
            running: AtomicBool::new(false),
        }
    }

    pub async fn run_loop(self: Arc<Self>) {
        self.running.store(true, Ordering::SeqCst);
        info!("🚀 Bot [{}] (Baiak Idle) iniciado com 100% de paridade e otimização Zero-GC.", self.config.name);

        // 1. Sincronização Inicial de Personagens via tRPC Real
        let mut char_id_opt: Option<String> = None;
        if !self.config.credentials.token.is_empty() && self.config.credentials.token != "seu-token-de-sessao-ou-jwt" {
            match self.trpc.get_characters().await {
                Ok(chars) => {
                    if let Some(first) = chars.first() {
                        info!("👤 Personagem sincronizado: [{}] Nível {} ({}) - Gold: {}", first.name, first.level, first.vocation, first.gold);
                        self.current_level.store(first.level, Ordering::Relaxed);
                        self.gold.store(first.gold, Ordering::Relaxed);
                        char_id_opt = Some(first.id.clone());
                        *self.character.write().await = Some(first.clone());

                        let target_area = if !self.config.settings.hunt_target.is_empty() {
                            self.config.settings.hunt_target.clone()
                        } else if let Some(sim_cave) = find_optimal_hunt(first.level, true) {
                            sim_cave
                        } else {
                            self.catalog.select_best_hunt(first.level)
                        };
                        *self.current_hunt_id.write().await = target_area;
                    }
                    *self.all_characters.write().await = chars;
                }
                Err(e) => {
                    warn!("⚠️ Falha na sincronização de personagem via tRPC: {}", e);
                }
            }
        }

        // 2. Conectar à Sala Colyseus 'hunt' oficial
        let origin = if self.config.credentials.origin.is_empty() {
            "https://baiakidle.com".to_string()
        } else {
            self.config.credentials.origin.clone()
        };

        if !self.config.credentials.token.is_empty() && self.config.credentials.token != "seu-token-de-sessao-ou-jwt" {
            let fp = build_fingerprint(&self.config.id);
            let mut options = json!({
                "token": self.config.credentials.token,
                "fp": fp,
                "ext": []
            });
            if let Some(cid) = char_id_opt {
                options["characterId"] = json!(cid);
            }
            match ColyseusRoom::join_or_create(&origin, "hunt", options).await {
                Ok(room) => {
                    info!("🎮 Sala Colyseus conectada! RoomID: {}, SessionID: {}", room.seat.room_id, room.seat.session_id);

                    let target_hunt = self.current_hunt_id.read().await.clone();

                    // Handshake oficial de caça idêntico ao Go e Live Client
                    let _ = room.send_action("ready", json!({})).await;
                    let _ = room.send_action("mode", json!({ "mode": "hunt" })).await;
                    let _ = room.send_action("loop", json!(true)).await;
                    let _ = room.send_action("offlinemode", json!({ "mode": "hunt", "huntId": target_hunt })).await;
                    let _ = room.send_action("autosellfull", json!({ "on": true })).await;
                    let _ = room.send_action("autosellpct", json!({ "pct": 85 })).await;
                    let _ = room.send_action("protecttiers", json!({ "tiers": [3, 4, 5, 6, 7, 8, 9, 10] })).await;
                    let _ = room.send_action("codexauto", json!({ "on": true })).await;

                    // Despacho oficial de stage
                    let _ = room.send_action("stage", json!({
                        "huntId": target_hunt,
                        "tier": 0,
                        "autoTier": true,
                    })).await;

                    // Extras oficiais do jogo
                    run_daily_rewards(&room).await;
                    run_guild_tasks(&room).await;
                    run_auto_prey(&room, &target_hunt).await;
                    run_charms_auto(&room, &target_hunt).await;
                    run_codex_auto_sync(&room).await;

                    // Sincronizar rotação Max DPS e Poções BiS
                    let party_chars = self.all_characters.read().await.clone();
                    sync_party_auto_config(&self.trpc, Some(&room), &party_chars, &target_hunt).await;

                    // Gasto de talentos inicial
                    if self.config.settings.auto_treino {
                        spend_talents_for_party(&room, &party_chars).await;
                    }

                    // Auto-equip inicial
                    if let Some(c) = self.character.read().await.as_ref() {
                        run_auto_equip(&self.trpc, &room, c, 0, 3).await;
                    }

                    // Escutar eventos Colyseus em background
                    let mut rx = room.event_bus.subscribe();
                    let pass_mgr = self.pass_mgr.clone();
                    let death_cnt = self.death_count.clone();
                    tokio::spawn(async move {
                        while let Ok(msg) = rx.recv().await {
                            match msg.msg_type.as_str() {
                                "bpstatus" => {
                                    pass_mgr.handle_bp_status(&msg.payload).await;
                                }
                                "deaths" => {
                                    death_cnt.fetch_add(1, Ordering::Relaxed);
                                }
                                _ => {}
                            }
                        }
                    });

                    *self.colyseus_room.write().await = Some(room);
                }
                Err(e) => {
                    warn!("⚠️ Não foi possível ingressar na sala Colyseus imediata: {}. Operando em modo de espera.", e);
                }
            }
        }

        let mut loop_counter = 0u64;

        while self.running.load(Ordering::SeqCst) {
            loop_counter += 1;

            // Disjuntor Anti-Cheat
            if !self.circuit_breaker.can_execute().await {
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Stamina / Fadiga
            if !self.stamina.should_hunt().await {
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Recompensas Diárias periódicas
            if self.config.settings.auto_daily && self.dailies.needs_claim() {
                self.dailies.record_claim();
                let room_guard = self.colyseus_room.read().await;
                if let Some(r) = room_guard.as_ref() {
                    run_daily_rewards(r).await;
                    self.pass_mgr.check_and_claim(r).await;
                }
            }

            // Caça Ativa
            if self.config.settings.auto_hunt {
                let lvl = self.current_level.load(Ordering::Relaxed);
                let target_area = self.current_hunt_id.read().await.clone();

                // Revalidação periódica de stage na sala Colyseus a cada 20 loops (~30s)
                if loop_counter % 20 == 1 {
                    let room_guard = self.colyseus_room.read().await;
                    if let Some(r) = room_guard.as_ref() {
                        let _ = r.send_action("stage", json!({
                            "huntId": target_area,
                            "tier": 0,
                            "autoTier": true,
                        })).await;
                        info!("🗡️ [{}] Caçando na cave [{}]", self.config.name, target_area);
                    }
                }

                // Ciclo de Leilão e Caixa de Entrada a cada 30 loops (~45s)
                if loop_counter % 30 == 0 {
                    let room_guard = self.colyseus_room.read().await;
                    if let Some(r) = room_guard.as_ref() {
                        let token = self.turnstile_token.read().await.clone();
                        let pw = self.config.credentials.password.clone();
                        run_auction_cycle(&self.trpc, r, &token, &pw).await;
                    }
                }

                // Auto-Equip a cada 60 loops (~90s)
                if loop_counter % 60 == 0 {
                    let room_guard = self.colyseus_room.read().await;
                    if let Some(r) = room_guard.as_ref() {
                        if let Some(c) = self.character.read().await.as_ref() {
                            run_auto_equip(&self.trpc, r, c, 0, 3).await;
                        }
                    }
                }

                // Safe Boss a cada 100 loops
                if loop_counter % 100 == 0 {
                    let room_guard = self.colyseus_room.read().await;
                    if let Some(r) = room_guard.as_ref() {
                        let deaths = self.death_count.load(Ordering::Relaxed) as usize;
                        run_safe_bosses(r, lvl, deaths).await;
                    }
                }

                // Automação periódica de Dungeon a cada 150 loops
                if loop_counter % 150 == 0 {
                    let room_guard = self.colyseus_room.read().await;
                    if let Some(r) = room_guard.as_ref() {
                        run_dungeon_auto(r).await;
                    }
                }

                // Passe de Batalha a cada 120 loops
                if loop_counter % 120 == 0 {
                    let room_guard = self.colyseus_room.read().await;
                    if let Some(r) = room_guard.as_ref() {
                        self.pass_mgr.check_and_claim(r).await;
                    }
                }

                self.kills.fetch_add(1, Ordering::Relaxed);
                self.gold.fetch_add(1500, Ordering::Relaxed);
                self.exp.fetch_add(2500, Ordering::Relaxed);

                // Auto-Treino se fadiga > 50%
                if self.config.settings.auto_treino && self.stamina.current_fatigue().await > 50.0 {
                    let room_guard = self.colyseus_room.read().await;
                    if let Some(r) = room_guard.as_ref() {
                        let _ = r.send_action("mode", json!({ "mode": "exercise" })).await;
                        let _ = r.send_action("tocity", json!({})).await;
                        info!("[{}] Fadiga > 50%. Entrando em Treino Online...", self.config.name);
                    }
                }

                self.circuit_breaker.record_success().await;
            }

            sleep(Duration::from_millis(1500)).await;
        }

        info!("🛑 Bot [{}] finalizado.", self.config.name);
    }
}
