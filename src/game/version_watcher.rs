use reqwest::Client;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct VersionChecker {
    client: Client,
    pub current_build_hash: Arc<RwLock<String>>,
    pub update_detected: Arc<AtomicBool>,
}

impl Default for VersionChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionChecker {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            current_build_hash: Arc::new(RwLock::new(String::new())),
            update_detected: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Captura o nome/hash atual do script principal em https://baiakidle.com/jogar/
    pub async fn fetch_latest_build_hash(&self) -> Result<String, String> {
        let url = "https://baiakidle.com/jogar/";
        let resp = self.client.get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| format!("Falha ao conectar no portal: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Status HTTP: {}", resp.status()));
        }

        let body = resp.text().await.map_err(|e| format!("Falha ao ler HTML: {e}"))?;

        // Procura por href="/jogar/assets/game-XXXXX.js" ou src="/jogar/assets/main-XXXXX.js"
        if let Some(pos) = body.find("/jogar/assets/game-") {
            let slice = &body[pos + 14..];
            if let Some(end) = slice.find(".js") {
                return Ok(slice[..end].to_string());
            }
        }
        if let Some(pos) = body.find("/jogar/assets/main-") {
            let slice = &body[pos + 14..];
            if let Some(end) = slice.find(".js") {
                return Ok(slice[..end].to_string());
            }
        }

        Err("Não foi possível localizar o hash do bundle de jogo no HTML".to_string())
    }

    /// Executa uma verificação periódica. Retorna true se houver nova atualização.
    pub async fn check_for_updates(&self) -> bool {
        match self.fetch_latest_build_hash().await {
            Ok(hash) => {
                let mut current = self.current_build_hash.write().await;
                if current.is_empty() {
                    info!("🛡️ [Version Watcher] Build inicial do jogo registrada: [{}]", hash);
                    *current = hash;
                    return false;
                }

                if *current != hash {
                    warn!("🚨 [ALERTA DE ATUALIZAÇÃO] O Baiak Idle foi atualizado no servidor!");
                    warn!("   Build anterior: [{}] -> Nova Build: [{}]", *current, hash);
                    *current = hash;
                    self.update_detected.store(true, Ordering::SeqCst);
                    return true;
                }
            }
            Err(e) => {
                warn!("⚠️ [Version Watcher] Falha ao verificar versão do jogo: {}", e);
            }
        }
        false
    }
}
