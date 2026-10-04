//! Definição de pacotes e serialização binária para o protocolo de rede do Hades.

use bytes::{Buf, BufMut, BytesMut};
use hades_core::{BitpackError, MovementDelta};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("Tamanho do buffer insuficiente: esperado {0}, disponível {1}")]
    BufferUnderflow(usize, usize),

    #[error("Tipo de pacote desconhecido: 0x{0:02X}")]
    UnknownPacketType(u8),

    #[error("String UTF-8 inválida no pacote")]
    InvalidUtf8,

    #[error("Erro de bitpacking no datagrama: {0}")]
    Bitpack(#[from] BitpackError),
}

/// Mensagens enviadas através dos streams multiplexados confiáveis (TCP-like, mas sem HoL blocking global).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReliablePacket {
    Ping,
    Pong,
    AuthRequest { account_id: u32, token: String },
    AuthResponse { success: bool, char_id: u32 },
    ChatMessage { channel: u8, text: String },
}

impl ReliablePacket {
    pub const TYPE_PING: u8 = 0x00;
    pub const TYPE_PONG: u8 = 0x01;
    pub const TYPE_AUTH_REQ: u8 = 0x02;
    pub const TYPE_AUTH_RESP: u8 = 0x03;
    pub const TYPE_CHAT: u8 = 0x04;

    /// Codifica o pacote confiável em bytes contíguos com prefixo de tamanho (u16 LE) e tipo (u8).
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        match self {
            Self::Ping => {
                buf.put_u8(Self::TYPE_PING);
            }
            Self::Pong => {
                buf.put_u8(Self::TYPE_PONG);
            }
            Self::AuthRequest { account_id, token } => {
                buf.put_u8(Self::TYPE_AUTH_REQ);
                buf.put_u32_le(*account_id);
                let bytes = token.as_bytes();
                buf.put_u16_le(bytes.len() as u16);
                buf.put_slice(bytes);
            }
            Self::AuthResponse { success, char_id } => {
                buf.put_u8(Self::TYPE_AUTH_RESP);
                buf.put_u8(if *success { 1 } else { 0 });
                buf.put_u32_le(*char_id);
            }
            Self::ChatMessage { channel, text } => {
                buf.put_u8(Self::TYPE_CHAT);
                buf.put_u8(*channel);
                let bytes = text.as_bytes();
                buf.put_u16_le(bytes.len() as u16);
                buf.put_slice(bytes);
            }
        }
        buf.to_vec()
    }

    /// Decodifica um pacote a partir de um buffer de bytes brutos.
    pub fn decode(mut bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.is_empty() {
            return Err(ProtocolError::BufferUnderflow(1, 0));
        }

        let packet_type = bytes.get_u8();
        match packet_type {
            Self::TYPE_PING => Ok(Self::Ping),
            Self::TYPE_PONG => Ok(Self::Pong),
            Self::TYPE_AUTH_REQ => {
                if bytes.remaining() < 6 {
                    return Err(ProtocolError::BufferUnderflow(6, bytes.remaining()));
                }
                let account_id = bytes.get_u32_le();
                let len = bytes.get_u16_le() as usize;
                if bytes.remaining() < len {
                    return Err(ProtocolError::BufferUnderflow(len, bytes.remaining()));
                }
                let str_bytes = &bytes[..len];
                let token = std::str::from_utf8(str_bytes)
                    .map_err(|_| ProtocolError::InvalidUtf8)?
                    .to_string();
                Ok(Self::AuthRequest { account_id, token })
            }
            Self::TYPE_AUTH_RESP => {
                if bytes.remaining() < 5 {
                    return Err(ProtocolError::BufferUnderflow(5, bytes.remaining()));
                }
                let success = bytes.get_u8() != 0;
                let char_id = bytes.get_u32_le();
                Ok(Self::AuthResponse { success, char_id })
            }
            Self::TYPE_CHAT => {
                if bytes.remaining() < 3 {
                    return Err(ProtocolError::BufferUnderflow(3, bytes.remaining()));
                }
                let channel = bytes.get_u8();
                let len = bytes.get_u16_le() as usize;
                if bytes.remaining() < len {
                    return Err(ProtocolError::BufferUnderflow(len, bytes.remaining()));
                }
                let str_bytes = &bytes[..len];
                let text = std::str::from_utf8(str_bytes)
                    .map_err(|_| ProtocolError::InvalidUtf8)?
                    .to_string();
                Ok(Self::ChatMessage { channel, text })
            }
            other => Err(ProtocolError::UnknownPacketType(other)),
        }
    }
}

/// Codifica um MovementDelta de 6 bytes diretamente para envio como datagrama QUIC.
#[inline(always)]
pub fn encode_movement_datagram(delta: &MovementDelta) -> [u8; MovementDelta::PACKET_SIZE] {
    delta.encode()
}

/// Decodifica um datagrama QUIC de 6 bytes para um MovementDelta.
#[inline(always)]
pub fn decode_movement_datagram(bytes: &[u8]) -> Result<MovementDelta, ProtocolError> {
    if bytes.len() != MovementDelta::PACKET_SIZE {
        return Err(ProtocolError::BufferUnderflow(
            MovementDelta::PACKET_SIZE,
            bytes.len(),
        ));
    }
    let mut fixed = [0u8; MovementDelta::PACKET_SIZE];
    fixed.copy_from_slice(bytes);
    MovementDelta::decode(&fixed).map_err(ProtocolError::Bitpack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hades_core::{Direction, EntityId, Position};

    #[test]
    fn test_reliable_packet_roundtrip_ping_pong() {
        let ping = ReliablePacket::Ping;
        assert_eq!(ReliablePacket::decode(&ping.encode()).unwrap(), ping);

        let pong = ReliablePacket::Pong;
        assert_eq!(ReliablePacket::decode(&pong.encode()).unwrap(), pong);
    }

    #[test]
    fn test_reliable_packet_roundtrip_auth_and_chat() {
        let auth = ReliablePacket::AuthRequest {
            account_id: 2000001,
            token: "secret_token_12345".to_string(),
        };
        assert_eq!(ReliablePacket::decode(&auth.encode()).unwrap(), auth);

        let chat = ReliablePacket::ChatMessage {
            channel: 1, // Canal Geral
            text: "Olá Prontera!".to_string(),
        };
        assert_eq!(ReliablePacket::decode(&chat.encode()).unwrap(), chat);
    }

    #[test]
    fn test_datagram_movement_delta_roundtrip() {
        let delta = MovementDelta::new(
            EntityId::new(42),
            Position::new_unchecked(150, 280),
            Direction::SouthEast,
            3,
        );

        let encoded = encode_movement_datagram(&delta);
        assert_eq!(encoded.len(), 6);

        let decoded = decode_movement_datagram(&encoded).unwrap();
        assert_eq!(delta, decoded);
    }
}
