//! SPEC-0016: Mensagens do protocolo de rede do World Server.

use serde::{Deserialize, Serialize};

/// Mensagens enviadas do Cliente para o World Server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ClientWorldMsg {
    /// Solicitação de entrada no mapa mundial após o handshake.
    #[serde(rename = "enter_world")]
    EnterWorld {
        aid: u32,
        gid: u32,
        #[serde(default)]
        auth_code: i32,
        #[serde(default)]
        name: Option<String>,
    },

    /// Solicitação de caminhada/movimento para coordenadas de destino.
    #[serde(rename = "move_request")]
    MoveRequest {
        to_x: u16,
        to_y: u16,
    },

    /// Ping / Keepalive de aplicação (opcional, com timestamp em ms para RTT).
    #[serde(rename = "ping")]
    Ping {
        #[serde(default)]
        timestamp: u64,
    },
}

/// Mensagens enviadas do World Server para o Cliente.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ServerWorldMsg {
    /// Resposta de sucesso na entrada do mapa. Mapeia para ACCEPT_ENTER no cliente.
    #[serde(rename = "enter_world_ok")]
    EnterWorldOk {
        aid: u32,
        gid: u32,
        name: String,
        map_name: String,
        pos_x: u16,
        pos_y: u16,
        dir: u8,
        sex: u8,
        speed: u16,
    },

    /// Confirmação de movimento validado para o próprio jogador (NOTIFY_PLAYERMOVE).
    #[serde(rename = "player_move")]
    PlayerMove {
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
        start_time: u64,
        end_time: u64,
    },

    /// Notificação de entidade que surgiu ou já estava no raio AoI 3x3 (NOTIFY_STANDENTRY / NEWENTRY).
    #[serde(rename = "entity_spawn")]
    EntitySpawn {
        id: u16,
        name: String,
        job: u16,
        pos_x: u16,
        pos_y: u16,
        dir: u8,
        speed: u16,
    },

    /// Notificação de entidade remota se movendo no raio AoI 3x3 (NOTIFY_MOVE).
    #[serde(rename = "entity_move")]
    EntityMove {
        id: u16,
        from_x: u16,
        from_y: u16,
        to_x: u16,
        to_y: u16,
        start_time: u64,
        end_time: u64,
    },

    /// Notificação de entidade saindo do raio AoI ou desconectando (NOTIFY_VANISH).
    #[serde(rename = "entity_despawn")]
    EntityDespawn {
        id: u16,
    },

    /// Resposta de Pong/Heartbeat com eco do timestamp para medição precisa de RTT.
    #[serde(rename = "pong")]
    Pong {
        #[serde(default)]
        timestamp: u64,
    },

    /// Mensagem de erro / recusa.
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

impl ServerWorldMsg {
    /// Serializa para linha JSON com quebra de linha `\n`.
    pub fn to_json_line(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(self).unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_world_roundtrip() {
        let json = r#"{"type":"enter_world","aid":1,"gid":1,"auth_code":12345}"#;
        let msg: ClientWorldMsg = serde_json::from_str(json).expect("deserializar enter_world");
        assert_eq!(
            msg,
            ClientWorldMsg::EnterWorld {
                aid: 1,
                gid: 1,
                auth_code: 12345,
                name: None,
            }
        );
    }

    #[test]
    fn test_move_request_roundtrip() {
        let json = r#"{"type":"move_request","to_x":160,"to_y":182}"#;
        let msg: ClientWorldMsg = serde_json::from_str(json).expect("deserializar move_request");
        assert_eq!(msg, ClientWorldMsg::MoveRequest { to_x: 160, to_y: 182 });
    }

    #[test]
    fn test_enter_world_ok_serialize() {
        let msg = ServerWorldMsg::EnterWorldOk {
            aid: 1,
            gid: 1,
            name: "Berenice".to_string(),
            map_name: "prontera.gat".to_string(),
            pos_x: 156,
            pos_y: 180,
            dir: 0,
            sex: 0,
            speed: 150,
        };
        let line = msg.to_json_line();
        assert!(line.ends_with(b"\n"));
        let parsed: ServerWorldMsg =
            serde_json::from_slice(&line[..line.len() - 1]).expect("parse ServerWorldMsg");
        assert_eq!(parsed, msg);
    }
}
