use crate::game::colyseus::ColyseusRoom;
use crate::game::jev::{estimate_item_market_value_coins, ItemAttribute};
use crate::game::trpc::TRPCClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CoinBalances {
    #[serde(default)]
    pub coins: i64,
    #[serde(default)]
    pub market_coins: i64,
    #[serde(default)]
    pub event_coins: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItemSnippet {
    #[serde(default)]
    pub id: Option<Value>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tier: i64,
    #[serde(default)]
    pub up_level: i64,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionBrowseItem {
    #[serde(default)]
    pub id: i64,
    #[serde(rename = "type", default)]
    pub listing_type: String, // "item" | "gold"
    #[serde(default)]
    pub item: Option<InventoryItemSnippet>,
    #[serde(default)]
    pub gold_amount: f64,
    #[serde(default)]
    pub start_price: f64,
    #[serde(default)]
    pub current_price: f64,
    #[serde(default)]
    pub bid_count: i64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub ends_at: f64,
    #[serde(default)]
    pub is_own: bool,
    #[serde(default)]
    pub is_leading: bool,
}

pub async fn run_auction_cycle(
    trpc: &TRPCClient,
    room: &ColyseusRoom,
    captcha_token: &str,
    password: &str,
) {
    // 1. Resgatar compras e vendas encerradas da caixa de entrada
    let _ = room.send_action("inboxtakeall", json!({})).await;
    let _ = room.send_action("goldinboxclaim", json!({})).await;

    // 2. Atualizar saldos de moedas
    let mut bal = CoinBalances::default();
    if let Ok(b_val) = trpc.query("coin.balances", None).await {
        if let Ok(parsed) = serde_json::from_value::<CoinBalances>(b_val) {
            bal = parsed;
        }
    }

    // 3. Executar o Sniper de Leilão
    run_auction_sniper(trpc, &bal).await;

    // 4. Consultar anúncios ativos
    let mut active_count = 0;
    if let Ok(mine) = trpc.query("auction.mine", None).await {
        if let Some(arr) = mine.as_array() {
            active_count = arr.len();
            if active_count > 0 {
                info!("🛒 [Leilão] {} anúncios ativos monitorados no mercado.", active_count);
            }
        }
    }

    // Se temos menos de 10 anúncios e temos captcha_token / password, avalia mochila para vender excedente
    if active_count < 10 {
        if let Ok(summary_val) = trpc.query("items.summary", None).await {
            if let Some(items_arr) = summary_val.get("items").and_then(|v| v.as_array()) {
                for it_val in items_arr {
                    if active_count >= 10 {
                        break;
                    }
                    if let Ok(item) = serde_json::from_value::<InventoryItemSnippet>(it_val.clone()) {
                        if item.location == "equipped" || item.hash.is_empty() {
                            continue;
                        }
                        let est_val = estimate_item_market_value_coins(&item.name, item.tier, item.up_level, &[]);
                        if est_val >= 100 && !captcha_token.is_empty() {
                            let target_price = (est_val as f64 * 0.85).max(25.0) as i64;
                            let payload = json!({
                                "itemHash": item.hash,
                                "startPrice": target_price,
                                "durationHours": 24,
                                "password": password,
                                "confirmText": item.name,
                                "captchaToken": captcha_token,
                            });
                            if let Ok(_) = trpc.mutate("auction.create", payload).await {
                                info!("✔ [Leilão] Anúncio criado para {} (+{} Tier {}) por {} coins", item.name, item.up_level, item.tier, target_price);
                                active_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

pub async fn run_auction_sniper(trpc: &TRPCClient, bal: &CoinBalances) {
    let total_coins = bal.coins + bal.market_coins;
    if total_coins < 25 {
        return;
    }

    let browse_raw = match trpc.query("auction.browse", None).await {
        Ok(v) => v,
        Err(_) => return,
    };

    let rows: Vec<AuctionBrowseItem> = browse_raw
        .get("rows")
        .and_then(|r| serde_json::from_value(r.clone()).ok())
        .unwrap_or_default();

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64;

    for l in rows {
        if l.status != "active" || l.is_own || l.is_leading {
            continue;
        }

        let min_bid = if l.bid_count <= 0 {
            l.start_price.max(25.0)
        } else {
            let in_soft_close = (l.ends_at - now_ms) <= 60000.0;
            let step = if in_soft_close {
                (l.current_price * 0.10).ceil().max(1.0)
            } else {
                1.0
            };
            (l.current_price + step).max(25.0)
        };

        if min_bid as i64 > total_coins {
            continue;
        }

        let mins_left = (l.ends_at - now_ms) / 60000.0;

        // Caso Gold barato (> 8kk por coin)
        if l.listing_type == "gold" && l.gold_amount > 0.0 {
            let gold_per_coin = l.gold_amount / min_bid;
            if gold_per_coin >= 8_000_000.0 && (mins_left <= 5.0 || l.bid_count == 0) {
                let curr = if bal.market_coins >= min_bid as i64 { "market" } else { "normal" };
                let payload = json!({
                    "listingId": l.id,
                    "maxAmount": min_bid as i64,
                    "currency": curr,
                });
                if let Ok(_) = trpc.mutate("auction.bid", payload).await {
                    info!("🎯 [SNIPER] Lance dado em {}kk gold por {} coins!", (l.gold_amount / 1_000_000.0) as i64, min_bid as i64);
                    return;
                }
            }
        }

        // Caso Item valioso com margem >= 35 coins
        if l.listing_type == "item" {
            if let Some(it) = &l.item {
                let est_val = estimate_item_market_value_coins(&it.name, it.tier, it.up_level, &[]);
                if est_val >= (min_bid as i64 + 35) && (mins_left <= 5.0 || l.bid_count == 0) {
                    let curr = if bal.market_coins >= min_bid as i64 { "market" } else { "normal" };
                    let payload = json!({
                        "listingId": l.id,
                        "maxAmount": min_bid as i64,
                        "currency": curr,
                    });
                    if let Ok(_) = trpc.mutate("auction.bid", payload).await {
                        info!("🎯 [SNIPER] Lance dado no item {} por {} coins (Valor de mercado: ~{}c)!", it.name, min_bid as i64, est_val);
                        return;
                    }
                }
            }
        }
    }
}
