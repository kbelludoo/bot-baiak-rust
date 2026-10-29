use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    pub credentials: GameCredentials,
    #[serde(default)]
    pub settings: GameSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCredentials {
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameSettings {
    #[serde(default, alias = "hunt_target", alias = "huntTarget")]
    pub hunt_target: String,
    #[serde(default)]
    pub priority: String,
    #[serde(default = "default_true", alias = "auto_hunt", alias = "autoHunt")]
    pub auto_hunt: bool,
    #[serde(default = "default_true", alias = "auto_sell", alias = "autoSell")]
    pub auto_sell: bool,
    #[serde(default = "default_true", alias = "auto_heal", alias = "autoHeal")]
    pub auto_heal: bool,
    #[serde(default = "default_true", alias = "auto_daily", alias = "autoDaily", alias = "autoRewards", alias = "auto_rewards")]
    pub auto_daily: bool,
    #[serde(default = "default_true", alias = "auto_treino", alias = "autoTreino")]
    pub auto_treino: bool,
    #[serde(default = "default_true", alias = "auto_buy_vip", alias = "autoBuyVip")]
    pub auto_buy_vip: bool,
    #[serde(default = "default_true", alias = "auto_auction", alias = "autoAuction")]
    pub auto_auction: bool,
    #[serde(default, alias = "bind_ip", alias = "bindIP")]
    pub bind_ip: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub id: String,
    pub name: String,
    pub vocation: String,
    pub level: u32,
    #[serde(default)]
    pub stamina: u32,
    #[serde(default)]
    pub gold: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartyConfig {
    pub leader: Option<Value>,
    #[serde(default)]
    pub disabled: Vec<Value>,
}
