use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

pub const OP_HANDSHAKE: u8 = 9;
pub const OP_JOIN_ROOM: u8 = 10;
pub const OP_ERROR: u8 = 11;
pub const OP_LEAVE_ROOM: u8 = 12;
pub const OP_ROOM_DATA: u8 = 13;
pub const OP_ROOM_STATE: u8 = 14;
pub const OP_ROOM_STATE_PATCH: u8 = 15;
pub const OP_PING: u8 = 18;
pub const OP_PONG: u8 = 19;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seat {
    #[serde(default)]
    pub room_id: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub reconnection_token: String,
    #[serde(default)]
    pub process_id: String,
    #[serde(default)]
    pub public_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchmakeResponse {
    pub room: Option<Seat>,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RoomMessage {
    pub msg_type: String,
    pub payload: Value,
}

pub struct ColyseusRoom {
    pub seat: Seat,
    pub is_joined: Arc<AtomicBool>,
    pub is_closed: Arc<AtomicBool>,
    ws_sender: Arc<Mutex<Option<futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>>>>,
    pub event_bus: broadcast::Sender<RoomMessage>,
}

impl ColyseusRoom {
    pub async fn join_or_create(origin: &str, room_name: &str, options: Value) -> Result<Self, String> {
        let http_url = format!("{}/rt/matchmake/joinOrCreate/{}", origin.trim_end_matches('/'), room_name);
        let client = reqwest::Client::new();

        let resp = client.post(&http_url)
            .json(&options)
            .send()
            .await
            .map_err(|e| format!("Matchmake HTTP request failed: {e}"))?;

        let status = resp.status();
        let matchmake: MatchmakeResponse = resp.json()
            .await
            .map_err(|e| format!("Erro ao decodificar Matchmake: {e}"))?;

        if !status.is_success() {
            return Err(format!("Matchmake status {}: {:?}", status, matchmake.error));
        }

        let mut seat = matchmake.room.ok_or_else(|| "Nenhum seat retornado pelo Matchmake".to_string())?;
        if !matchmake.session_id.is_empty() {
            seat.session_id = matchmake.session_id;
        }

        let ws_url = if !seat.public_address.is_empty() {
            format!("wss://{}/{}/{}?sessionId={}", seat.public_address, seat.process_id, seat.room_id, urlencoding::encode(&seat.session_id))
        } else {
            let ws_scheme = if origin.starts_with("https") { "wss" } else { "ws" };
            let host = origin.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/');
            format!("{}://{}/rt/{}/{}?sessionId={}", ws_scheme, host, seat.process_id, seat.room_id, urlencoding::encode(&seat.session_id))
        };

        info!("[Colyseus] Conectando ao WebSocket da Sala: {}", ws_url);
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .map_err(|e| format!("Erro ao conectar WebSocket Colyseus: {e}"))?;

        let (mut write, mut read) = ws_stream.split();

        // Enviar Join Room ACK com Opcode 10
        let join_ack = vec![OP_JOIN_ROOM];
        write.send(Message::Binary(join_ack.into())).await.map_err(|e| format!("Falha no JoinRoom ACK: {e}"))?;

        let is_joined = Arc::new(AtomicBool::new(true));
        let is_closed = Arc::new(AtomicBool::new(false));
        let sender = Arc::new(Mutex::new(Some(write)));
        let (tx, _) = broadcast::channel::<RoomMessage>(256);
        let event_tx = tx.clone();

        // Spawn loop leitor de frames
        let closed_clone = is_closed.clone();
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(bin)) => {
                        if bin.is_empty() {
                            continue;
                        }
                        let opcode = bin[0];
                        match opcode {
                            OP_ROOM_DATA => {
                                // Decodificar msgpack [msg_type, payload]
                                if bin.len() > 1 {
                                    let slice = &bin[1..];
                                    let mut de = rmp_serde::Deserializer::new(slice);
                                    if let Ok(msg_type) = String::deserialize(&mut de) {
                                        let payload: Value = Value::deserialize(&mut de).unwrap_or(Value::Null);
                                        debug!("[Colyseus] Evento recebido: {}", msg_type);
                                        let _ = event_tx.send(RoomMessage { msg_type, payload });
                                    }
                                }
                            }
                            OP_PING => {
                                debug!("[Colyseus] Ping recebido");
                            }
                            OP_JOIN_ROOM => {
                                debug!("[Colyseus] Handshake confirmado pelo servidor");
                            }
                            _ => {
                                debug!("[Colyseus] Opcode ignorado: {}", opcode);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        warn!("[Colyseus] Sala fechada pelo servidor.");
                        closed_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                    Err(e) => {
                        error!("[Colyseus] Erro no stream WS: {}", e);
                        closed_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(Self {
            seat,
            is_joined,
            is_closed,
            ws_sender: sender,
            event_bus: tx,
        })
    }

    pub async fn send_action(&self, action: &str, data: Value) -> Result<(), String> {
        let mut sender = self.ws_sender.lock().await;
        if let Some(s) = sender.as_mut() {
            // Encode msgpack: [action, data]
            let mut buf = Vec::with_capacity(128);
            buf.push(OP_ROOM_DATA);
            if let Err(e) = rmp_serde::encode::write(&mut buf, &action) {
                return Err(format!("Erro ao codificar action msgpack: {e}"));
            }
            if let Err(e) = rmp_serde::encode::write(&mut buf, &data) {
                return Err(format!("Erro ao codificar payload msgpack: {e}"));
            }
            s.send(Message::Binary(buf.into())).await.map_err(|e| format!("Falha ao enviar ação Colyseus: {e}"))?;
            Ok(())
        } else {
            Err("WebSocket desconectado".to_string())
        }
    }
}
