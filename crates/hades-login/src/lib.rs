//! # hades-login
//!
//! Servidor de autenticação e seleção de personagens do Hades.
//! Protocolo: JSON sobre streams WebTransport/QUIC confiáveis.
//!
//! Fluxo:
//!   1. Client conecta via WebTransport
//!   2. Client envia `{"type":"login","username":"x","password":"sha256hex"}`
//!   3. Server responde `login_ok` ou `login_fail`
//!   4. Client envia `{"type":"char_list"}`
//!   5. Server responde `{"type":"char_list","chars":[...]}`
//!   6. Client envia `{"type":"char_select","gid":N}`
//!   7. Server responde `{"type":"char_selected",...}` com endereço do WorldServer

pub mod db;
pub mod messages;
pub mod session;

pub use messages::{ClientMsg, ServerMsg, CharData};
pub use session::LoginSession;
