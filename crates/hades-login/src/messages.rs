//! Mensagens JSON do protocolo Hades (SPEC-0015, derivado da engenharia reversa do roBrowser).
//! Cada campo é nomeado para mapear diretamente ao que o HadesProtocol.js espera.

use serde::{Deserialize, Serialize};

// ── Client → Server ──────────────────────────────────────────────────────────

/// Todas as mensagens que o cliente pode enviar no stream de login.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    /// Login com SHA-256 hex da senha.
    Login {
        username: String,
        password: String, // SHA-256 hex
    },
    /// Solicita lista de personagens.
    CharList {
        #[serde(default)]
        aid: Option<u32>,
        #[serde(default)]
        auth_code: Option<i32>,
    },
    /// Seleciona um personagem pelo GID.
    CharSelect { gid: u32 },
    /// Cria um personagem (Novice, stats distribuídos).
    CharCreate {
        name:       String,
        str:        u8,
        agi:        u8,
        vit:        u8,
        #[serde(rename = "int")]
        int_stat:   u8,
        dex:        u8,
        luk:        u8,
        hair_style: u16,
        hair_color: u16,
    },
    /// Deleta um personagem.
    CharDelete { gid: u32 },
}

// ── Server → Client ──────────────────────────────────────────────────────────

/// Dados de um personagem enviados ao client (mapeados para charInfo do roBrowser).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CharData {
    pub gid:          u32,
    pub name:         String,
    pub job:          u16,       // JobID: 0=Novice
    pub level:        u16,
    pub job_level:    u16,
    pub exp:          u64,
    pub job_exp:      u64,
    pub zeny:         u32,
    pub hp:           u32,
    pub max_hp:       u32,
    pub sp:           u32,
    pub max_sp:       u32,
    pub speed:        u16,       // walk delay ms (150 default)
    pub str:          u8,
    pub agi:          u8,
    pub vit:          u8,
    #[serde(rename = "int")]
    pub int_stat:     u8,
    pub dex:          u8,
    pub luk:          u8,
    pub skill_points: u16,
    pub job_points:   u16,
    pub hair_style:   u16,       // → head
    pub hair_color:   u16,       // → headpalette
    pub body:         u16,
    pub body_palette: u16,       // → bodypalette
    pub weapon:       u16,
    pub shield:       u16,
    pub accessory:    u16,
    pub accessory2:   u16,
    pub accessory3:   u16,
    pub map:          String,    // → lastMap (ex: "prontera")
    pub pos_x:        u16,
    pub pos_y:        u16,
    pub sex:          u8,        // 0=M, 1=F
    pub slot:         u8,        // → CharNum (índice 0..8)
}

impl CharData {
    /// Cria um Novice recém-criado com valores padrão.
    pub fn new_novice(gid: u32, name: String, str: u8, agi: u8, vit: u8, int_stat: u8, dex: u8, luk: u8, hair_style: u16, hair_color: u16, sex: u8, slot: u8) -> Self {
        Self {
            gid, name, job: 0, level: 1, job_level: 1,
            exp: 0, job_exp: 0, zeny: 0,
            hp: 40, max_hp: 40, sp: 11, max_sp: 11,
            speed: 150,
            str, agi, vit, int_stat, dex, luk,
            skill_points: 0, job_points: 0,
            hair_style, hair_color,
            body: 0, body_palette: 0, weapon: 0, shield: 0,
            accessory: 0, accessory2: 0, accessory3: 0,
            map: "prontera".into(), pos_x: 156, pos_y: 144,
            sex, slot,
        }
    }
}

/// Servidor Hades disponível (lista exibida após login).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerEntry {
    pub ip:        String,
    pub port:      u16,
    pub name:      String,
    pub usercount: u32,
    pub state:     u16,   // 0=normal, 1=manutenção, 2=cheio
    pub property:  u16,   // 0=normal, 1=pré-renewal, etc.
}

/// Todas as mensagens que o servidor pode enviar.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    /// Login bem-sucedido.
    LoginOk {
        auth_code:  i32,
        aid:        u32,
        user_level: u32,
        sex:        u8,
        servers:    Vec<ServerEntry>,
    },
    /// Login recusado.
    LoginFail {
        error_code: u8, // 0=senha errada, 1=ban, 2=servidor cheio
    },
    /// Lista de personagens da conta.
    CharList {
        chars: Vec<CharData>,
    },
    /// Personagem selecionado — redireciona para WorldServer.
    CharSelected {
        gid:        u32,
        map_name:   String, // ex: "prontera.gat"
        world_ip:   String,
        world_port: u16,
    },
    /// Personagem criado com sucesso.
    CharCreated {
        char: CharData,
    },
    /// Criação de personagem falhou.
    CharCreateFail {
        error_code: u8, // 1=nome duplicado, 2=slots cheios
    },
    /// Personagem deletado.
    CharDeleted,
    /// Erro genérico.
    Error {
        message: String,
    },
}

impl ServerMsg {
    /// Serializa para JSON + newline (framing do stream).
    pub fn to_json_line(&self) -> Vec<u8> {
        let mut s = serde_json::to_string(self).expect("serialização JSON não deve falhar");
        s.push('\n');
        s.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_msg_login_deserialize() {
        let raw = r#"{"type":"login","username":"luiz","password":"aabbcc"}"#;
        let msg: ClientMsg = serde_json::from_str(raw).unwrap();
        assert!(matches!(msg, ClientMsg::Login { .. }));
    }

    #[test]
    fn test_client_msg_char_list_deserialize() {
        let raw = r#"{"type":"char_list"}"#;
        let msg: ClientMsg = serde_json::from_str(raw).unwrap();
        assert!(matches!(msg, ClientMsg::CharList { .. }));
    }

    #[test]
    fn test_server_msg_login_ok_serialize() {
        let msg = ServerMsg::LoginOk {
            auth_code: 12345, aid: 2000001, user_level: 0, sex: 0,
            servers: vec![ServerEntry {
                ip: "127.0.0.1".into(), port: 6900,
                name: "Hades".into(), usercount: 0, state: 0, property: 0,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("login_ok"));
        assert!(json.contains("auth_code"));
        assert!(json.contains("Hades"));
    }

    #[test]
    fn test_char_data_roundtrip() {
        let c = CharData::new_novice(1001, "Ragnar".into(), 5,3,4,1,3,2, 0, 0, 0, 0);
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("Ragnar"));
        assert!(json.contains("prontera"));
    }
}
