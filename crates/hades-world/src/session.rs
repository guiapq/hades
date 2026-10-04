//! SPEC-0016: Sessão de rede do jogador no World Server (WebSocket e QUIC).

use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use hades_core::types::{EntityId, Position};

use crate::messages::{ClientWorldMsg, ServerWorldMsg};
use crate::world::WorldManager;

pub struct WorldSessionHandler {
    world: Arc<WorldManager>,
    default_pos: Position,
    verbose: bool,
}

impl WorldSessionHandler {
    pub fn new(world: Arc<WorldManager>, default_pos: Position) -> Self {
        Self {
            world,
            default_pos,
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Processa uma conexão WebSocket (usada por clientes web no navegador).
    pub async fn run_ws(
        &self,
        ws: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        peer: SocketAddr,
    ) {
        info!("Nova conexão World WebSocket de {peer}");

        let (tx, mut rx) = mpsc::unbounded_channel::<ServerWorldMsg>();
        let mut entity_id: Option<EntityId> = None;

        // Loop de escrita para o WebSocket
        let (mut ws_sink, mut ws_stream) = ws.split();

        let verbose = self.verbose;
        let write_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = serde_json::to_string(&msg).unwrap_or_default();
                if verbose {
                    info!("📤 [WS TX → {peer}]: {json}");
                } else {
                    debug!("→ [World WS {peer}]: {json}");
                }
                if ws_sink.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        });

        // Loop de leitura do WebSocket
        while let Some(msg_res) = ws_stream.next().await {
            let text = match msg_res {
                Ok(Message::Text(t)) => t.to_string(),
                Ok(Message::Binary(bin)) => match String::from_utf8(bin.to_vec()) {
                    Ok(s) => s,
                    Err(_) => continue,
                },
                Ok(Message::Ping(_p)) => {
                    let _ = tx.send(ServerWorldMsg::Error {
                        message: "pong".into(),
                    });
                    continue;
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => continue,
            };

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if self.verbose {
                    info!("📥 [WS RX ← {peer}]: {trimmed}");
                } else {
                    debug!("← [World WS {peer}]: {trimmed}");
                }

                let client_msg: ClientWorldMsg = match serde_json::from_str(trimmed) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("JSON inválido de [World WS {peer}]: {e} ({trimmed})");
                        continue;
                    }
                };

                match client_msg {
                    ClientWorldMsg::EnterWorld { aid, gid, name, .. } => {
                        if entity_id.is_none() {
                            let char_name = name.unwrap_or_else(|| format!("Player_{gid}"));
                            let (eid, _welcome) = self
                                .world
                                .register_player(aid, gid, char_name, self.default_pos, tx.clone())
                                .await;
                            entity_id = Some(eid);
                        }
                    }
                    ClientWorldMsg::Ping { timestamp } => {
                        let _ = tx.send(ServerWorldMsg::Pong { timestamp });
                    }
                    other => {
                        if let Some(eid) = entity_id {
                            self.world.handle_client_message(eid, other).await;
                        } else {
                            warn!("Mensagem recebida antes de enter_world de {peer}");
                        }
                    }
                }
            }
        }

        // Limpeza na desconexão
        if let Some(eid) = entity_id {
            self.world.unregister_player(eid).await;
        }

        write_task.abort();
        info!("Conexão World WebSocket encerrada: {peer}");
    }

    /// Processa uma conexão QUIC TLS 1.3 nativa (stream bidirecional confiável).
    pub async fn run_quic(&self, conn: quinn::Connection) {
        let peer = conn.remote_address();
        info!("Nova conexão World QUIC de {peer}");

        let Ok((mut send, mut recv)) = conn.accept_bi().await else {
            warn!("Falha ao aceitar stream bidirecional World de {peer}");
            return;
        };

        let (tx, mut rx) = mpsc::unbounded_channel::<ServerWorldMsg>();
        let mut entity_id: Option<EntityId> = None;

        // Task de escrita
        let verbose = self.verbose;
        let write_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if verbose {
                    let json = serde_json::to_string(&msg).unwrap_or_default();
                    info!("📤 [QUIC TX → {peer}]: {json}");
                } else {
                    debug!("📤 [World QUIC TX → {peer}]: {:?}", msg);
                }
                let bytes = msg.to_json_line();
                if send.write_all(&bytes).await.is_err() {
                    break;
                }
            }
        });

        // Loop de leitura
        let mut buf = Vec::new();
        loop {
            let mut tmp = [0u8; 4096];
            let n = match recv.read(&mut tmp).await {
                Ok(Some(n)) => n,
                Ok(None) => break,
                Err(e) => {
                    error!("Erro de leitura QUIC World de {peer}: {e}");
                    break;
                }
            };
            buf.extend_from_slice(&tmp[..n]);

            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line_bytes = buf.drain(..=pos).collect::<Vec<_>>();
                let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]).to_string();
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                if self.verbose {
                    info!("📥 [QUIC RX ← {peer}]: {trimmed}");
                } else {
                    debug!("📥 [World QUIC RX ← {peer}]: {trimmed}");
                }

                let client_msg: ClientWorldMsg = match serde_json::from_str(trimmed) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("JSON inválido QUIC World de {peer}: {e}");
                        continue;
                    }
                };

                match client_msg {
                    ClientWorldMsg::EnterWorld { aid, gid, name, .. } => {
                        if entity_id.is_none() {
                            let char_name = name.unwrap_or_else(|| format!("Player_{gid}"));
                            let (eid, _welcome) = self
                                .world
                                .register_player(aid, gid, char_name, self.default_pos, tx.clone())
                                .await;
                            entity_id = Some(eid);
                        }
                    }
                    ClientWorldMsg::Ping { timestamp } => {
                        let _ = tx.send(ServerWorldMsg::Pong { timestamp });
                    }
                    other => {
                        if let Some(eid) = entity_id {
                            self.world.handle_client_message(eid, other).await;
                        }
                    }
                }
            }
        }

        if let Some(eid) = entity_id {
            self.world.unregister_player(eid).await;
        }

        write_task.abort();
        info!("Conexão World QUIC encerrada: {peer}");
    }
}
