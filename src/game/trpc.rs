use crate::game::types::{CharacterInfo, PartyConfig};
use crate::network::client::build_http_client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

pub const DEFAULT_TRPC_BASE: &str = "https://baiakidle.com/api/trpc";
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

pub struct TRPCClient {
    base_url: String,
    token: String,
    client: Client,
}

impl TRPCClient {
    pub fn new(token: &str, origin: &str, bind_ip: &str) -> Self {
        let base = if origin.is_empty() {
            DEFAULT_TRPC_BASE.to_string()
        } else {
            format!("{}/api/trpc", origin.trim_end_matches('/'))
        };

        let client = build_http_client(bind_ip, 12);

        Self {
            base_url: base,
            token: token.to_string(),
            client,
        }
    }

    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
        headers.insert("accept", HeaderValue::from_static("application/json"));
        headers.insert("sec-ch-ua", HeaderValue::from_static(
            r#""Chromium";v="128", "Not;A=Brand";v="24", "Google Chrome";v="128""#
        ));
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        headers.insert("sec-ch-ua-platform", HeaderValue::from_static(r#""Windows""#));

        if !self.token.is_empty() {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", self.token)) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        headers
    }

    fn unwrap_response(&self, body: &str) -> Result<Value, String> {
        let parsed: Value = serde_json::from_str(body).map_err(|e| format!("Erro no JSON tRPC: {e}"))?;

        let first = if let Some(arr) = parsed.as_array() {
            arr.first().cloned().unwrap_or(Value::Null)
        } else {
            parsed
        };

        if let Some(err_obj) = first.get("error") {
            if !err_obj.is_null() {
                return Err(format!("tRPC error: {err_obj}"));
            }
        }

        if let Some(res) = first.get("result") {
            if let Some(data) = res.get("data") {
                if let Some(json_field) = data.get("json") {
                    return Ok(json_field.clone());
                }
                return Ok(data.clone());
            }
        }

        Ok(first)
    }

    pub async fn query(&self, path: &str, input: Option<Value>) -> Result<Value, String> {
        let input_val = input.unwrap_or(Value::Null);
        let wrapped = json!({ "0": input_val });
        let input_str = wrapped.to_string();
        let encoded_input = urlencoding::encode(&input_str);

        let url = format!("{}/{}?batch=1&input={}", self.base_url, path, encoded_input);

        let resp = self.client
            .get(&url)
            .headers(self.default_headers())
            .send()
            .await
            .map_err(|e| format!("Falha de conexão tRPC GET {}: {}", path, e))?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("Falha ao ler corpo: {e}"))?;

        if !status.is_success() {
            return Err(format!("tRPC GET {} status {}: {}", path, status, body));
        }

        self.unwrap_response(&body)
    }

    pub async fn mutate(&self, path: &str, input: Value) -> Result<Value, String> {
        let wrapped = json!({ "0": input });
        let url = format!("{}/{}?batch=1", self.base_url, path);

        let mut headers = self.default_headers();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let resp = self.client
            .post(&url)
            .headers(headers)
            .json(&wrapped)
            .send()
            .await
            .map_err(|e| format!("Falha de conexão tRPC POST {}: {}", path, e))?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("Falha ao ler corpo: {e}"))?;

        if !status.is_success() {
            return Err(format!("tRPC POST {} status {}: {}", path, status, body));
        }

        self.unwrap_response(&body)
    }

    pub async fn get_characters(&self) -> Result<Vec<CharacterInfo>, String> {
        let val = self.query("characters.list", None).await?;
        let mut list = Vec::new();

        if let Some(arr) = val.as_array() {
            for item in arr {
                let id = item.get("id")
                    .map(|v| {
                        if let Some(s) = v.as_str() {
                            s.to_string()
                        } else if let Some(n) = v.as_i64() {
                            n.to_string()
                        } else {
                            v.to_string()
                        }
                    })
                    .unwrap_or_default();
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let vocation = item.get("vocation").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let level = item.get("level").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let stamina = item.get("stamina")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(42.0) as u32;
                let gold = item.get("gold").and_then(|v| v.as_u64()).unwrap_or(0);

                list.push(CharacterInfo {
                    id,
                    name,
                    vocation,
                    level,
                    stamina,
                    gold,
                });
            }
        }
        Ok(list)
    }

    pub async fn get_party_config(&self) -> Result<PartyConfig, String> {
        let val = self.query("characters.partyConfig", None).await?;
        serde_json::from_value(val).map_err(|e| format!("Erro ao decodificar partyConfig: {e}"))
    }

    pub async fn enter_hunt(&self, hunt_id: &str) -> Result<Value, String> {
        self.mutate("hunts.enter", json!({ "huntId": hunt_id })).await
    }

    pub async fn buy_vip(&self) -> Result<Value, String> {
        self.mutate("shop.buyVip", json!({})).await
    }
}
