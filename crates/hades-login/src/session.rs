//! Sessão de login: lida com uma conexão WebTransport de um cliente.

use std::sync::Arc;
use tracing::{info, warn, error};

use crate::db::{Db, DbError};
use crate::messages::{ClientMsg, ServerMsg, ServerEntry};

/// Endereço do WorldServer a ser retornado após seleção de char.
pub struct WorldServerConfig {
    pub ip:   String,
    pub port: u16,
    pub name: String,
}

/// Gerencia uma sessão completa de login para um cliente.
pub struct LoginSession {
    db:     Arc<Db>,
    world:  WorldServerConfig,
}

impl LoginSession {
    pub fn new(db: Arc<Db>, world: WorldServerConfig) -> Self {
        Self { db, world }
    }

    /// Processa o stream bidirecional de um cliente WebTransport.
    /// Lê linhas JSON, despacha para o handler correto, escreve respostas.
    pub async fn run(&self, conn: quinn::Connection) {
        let peer = conn.remote_address();
        info!("Nova conexão de {peer}");

        // Aceita o stream bidirecional de login/char
        let Ok((mut send, mut recv)) = conn.accept_bi().await else {
            warn!("Falha ao aceitar stream bidirecional de {peer}");
            return;
        };

        let mut account_id: Option<u32> = None;
        let mut sex_session: u8 = 0;
        let mut buf = Vec::new();

        loop {
            // Lê bytes do stream até achar '\n'
            let mut tmp = [0u8; 4096];
            let n = match recv.read(&mut tmp).await {
                Ok(Some(n)) => n,
                Ok(None) => break, // EOF
                Err(e) => { error!("Erro de leitura de {peer}: {e}"); break; }
            };

            buf.extend_from_slice(&tmp[..n]);

            // Processa todas as linhas completas no buffer
            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line_bytes = buf.drain(..=pos).collect::<Vec<_>>();
                let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len()-1]).to_string();
                if line.trim().is_empty() { continue; }

                info!("← {peer}: {line}");

                let response = match serde_json::from_str::<ClientMsg>(&line) {
                    Err(e) => {
                        warn!("JSON inválido de {peer}: {e}");
                        ServerMsg::Error { message: format!("JSON inválido: {e}") }
                    }
                    Ok(msg) => self.handle(msg, &mut account_id, &mut sex_session).await,
                };

                let bytes = response.to_json_line();
                info!("→ {peer}: {}", std::str::from_utf8(&bytes).unwrap_or("?").trim());

                if let Err(e) = send.write_all(&bytes).await {
                    error!("Erro ao enviar resposta para {peer}: {e}");
                    return;
                }
            }
        }

        info!("Conexão encerrada: {peer}");
    }

    /// Processa uma conexão WebSocket transparente (usada por clientes web no browser).
    pub async fn run_ws(&self, mut ws: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, peer: std::net::SocketAddr) {
        use futures_util::{StreamExt, SinkExt};
        use tokio_tungstenite::tungstenite::Message;

        info!("Nova conexão WebSocket de {peer}");

        let mut account_id: Option<u32> = None;
        let mut sex_session: u8 = 0;

        while let Some(msg_res) = ws.next().await {
            let msg = match msg_res {
                Ok(Message::Text(text)) => text.to_string(),
                Ok(Message::Binary(bin)) => match String::from_utf8(bin.to_vec()) {
                    Ok(s) => s,
                    Err(_) => continue,
                },
                Ok(Message::Ping(p)) => {
                    let _ = ws.send(Message::Pong(p)).await;
                    continue;
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => continue,
            };

            for line in msg.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }

                info!("← [WS {peer}]: {trimmed}");

                let response = match serde_json::from_str::<ClientMsg>(trimmed) {
                    Err(e) => {
                        warn!("JSON inválido de [WS {peer}]: {e}");
                        ServerMsg::Error { message: format!("JSON inválido: {e}") }
                    }
                    Ok(client_msg) => self.handle(client_msg, &mut account_id, &mut sex_session).await,
                };

                let response_str = serde_json::to_string(&response).unwrap_or_default();
                info!("→ [WS {peer}]: {response_str}");

                if let Err(e) = ws.send(Message::Text(response_str.into())).await {
                    error!("Erro ao enviar resposta WebSocket para {peer}: {e}");
                    return;
                }
            }
        }

        info!("Conexão WebSocket encerrada: {peer}");
    }

    async fn handle(&self, msg: ClientMsg, account_id: &mut Option<u32>, sex: &mut u8) -> ServerMsg {
        match msg {
            ClientMsg::Login { username, password } => {
                match self.db.authenticate(&username, &password).await {
                    Ok((aid, s, user_level)) => {
                        *account_id = Some(aid);
                        *sex = s;
                        info!("Login OK: {username} (AID={aid})");
                        ServerMsg::LoginOk {
                            auth_code:  (aid as i32).wrapping_mul(7) ^ 0x1BADB002, // pseudo auth_code
                            aid,
                            user_level,
                            sex:        s,
                            servers:    vec![ServerEntry {
                                ip:        self.world.ip.clone(),
                                port:      self.world.port,
                                name:      self.world.name.clone(),
                                usercount: 0,
                                state:     0,
                                property:  0,
                            }],
                        }
                    }
                    Err(DbError::InvalidCredentials) => {
                        warn!("Login falhou: {username}");
                        ServerMsg::LoginFail { error_code: 0 }
                    }
                    Err(e) => {
                        error!("Erro de DB no login: {e}");
                        ServerMsg::Error { message: "Erro interno".into() }
                    }
                }
            }

            ClientMsg::CharList { aid: msg_aid, auth_code: _ } => {
                if let Some(a) = msg_aid {
                    *account_id = Some(a);
                }
                let aid = match account_id {
                    Some(a) => *a,
                    None => return ServerMsg::Error { message: "Não autenticado".into() },
                };
                match self.db.list_chars(aid).await {
                    Ok(chars) => ServerMsg::CharList { chars },
                    Err(e) => ServerMsg::Error { message: e.to_string() },
                }
            }

            ClientMsg::CharSelect { gid } => {
                let aid = match account_id {
                    Some(a) => *a,
                    None => return ServerMsg::Error { message: "Não autenticado".into() },
                };
                match self.db.get_char(aid, gid).await {
                    Ok(c) => ServerMsg::CharSelected {
                        gid:        c.gid,
                        map_name:   format!("{}.gat", c.map),
                        world_ip:   self.world.ip.clone(),
                        world_port: self.world.port,
                    },
                    Err(e) => ServerMsg::Error { message: e.to_string() },
                }
            }

            ClientMsg::CharCreate { name, str, agi, vit, int_stat, dex, luk, hair_style, hair_color } => {
                let aid = match account_id {
                    Some(a) => *a,
                    None => return ServerMsg::Error { message: "Não autenticado".into() },
                };
                match self.db.create_char(aid, &name, str, agi, vit, int_stat, dex, luk, hair_style, hair_color, *sex).await {
                    Ok(char) => ServerMsg::CharCreated { char },
                    Err(DbError::DuplicateName) => ServerMsg::CharCreateFail { error_code: 1 },
                    Err(DbError::SlotsFull)     => ServerMsg::CharCreateFail { error_code: 2 },
                    Err(e) => ServerMsg::Error { message: e.to_string() },
                }
            }

            ClientMsg::CharDelete { gid } => {
                let aid = match account_id {
                    Some(a) => *a,
                    None => return ServerMsg::Error { message: "Não autenticado".into() },
                };
                match self.db.delete_char(aid, gid).await {
                    Ok(()) => ServerMsg::CharDeleted,
                    Err(e) => ServerMsg::Error { message: e.to_string() },
                }
            }
        }
    }
}
