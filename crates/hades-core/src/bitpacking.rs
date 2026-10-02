//! Serialização e Bitpacking compacto para o protocolo de rede do Hades.
//!
//! Conforme definido na SPEC-0001, o `MovementDelta` é comprimido em exatos 6 bytes:
//!
//! ```text
//! [ Byte 0..1 ] Entity ID (u16, Little Endian)
//! [ Byte 2    ] PosX [0..7] (8 bits mais baixos de X)
//! [ Byte 3    ] PosX [8..11] (4 bits altos) | PosY [0..3] (4 bits baixos)
//! [ Byte 4    ] PosY [4..11] (8 bits mais altos de Y)
//! [ Byte 5    ] Direction (3 bits) | Action Flags (5 bits)
//! ```

use crate::types::{Direction, EntityId, Position, TypeError};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BitpackError {
    #[error("Erro nos tipos ao decodificar pacote: {0}")]
    TypeError(#[from] TypeError),

    #[error("Coordenada decodificada excede 12 bits: x={0}, y={1}")]
    InvalidCoordinates(u16, u16),
}

/// Representa a menor unidade atômica de atualização de movimento de uma entidade.
/// Comprimido em 6 bytes para transmissão ultrarrápida via datagramas WebTransport/QUIC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementDelta {
    pub entity_id: EntityId,
    pub position: Position,
    pub direction: Direction,
    pub action_flags: u8,
}

impl MovementDelta {
    pub const PACKET_SIZE: usize = 6;
    pub const MAX_ACTION_FLAGS: u8 = 0b0001_1111; // 5 bits

    /// Cria uma nova instância de `MovementDelta`.
    #[inline]
    pub fn new(
        entity_id: EntityId,
        position: Position,
        direction: Direction,
        action_flags: u8,
    ) -> Self {
        Self {
            entity_id,
            position,
            direction,
            action_flags: action_flags & Self::MAX_ACTION_FLAGS,
        }
    }

    /// Codifica a estrutura em um array fixo de 6 bytes.
    /// Operação determinística, livre de alocações na heap.
    #[inline]
    pub fn encode(&self) -> [u8; Self::PACKET_SIZE] {
        let mut buf = [0u8; Self::PACKET_SIZE];

        // Byte 0..1: Entity ID (Little Endian)
        let id_bytes = self.entity_id.0.to_le_bytes();
        buf[0] = id_bytes[0];
        buf[1] = id_bytes[1];

        // Byte 2: 8 bits mais baixos de PosX
        buf[2] = (self.position.x & 0x00FF) as u8;

        // Byte 3: 4 bits altos de PosX (bits 0..3) e 4 bits baixos de PosY (bits 4..7)
        let x_high = ((self.position.x >> 8) & 0x0F) as u8;
        let y_low = ((self.position.y & 0x0F) as u8) << 4;
        buf[3] = x_high | y_low;

        // Byte 4: 8 bits altos de PosY (bits 4..11)
        buf[4] = ((self.position.y >> 4) & 0x00FF) as u8;

        // Byte 5: Direction (3 bits) e Action Flags (5 bits)
        let dir_bits = self.direction.to_u8() & 0x07;
        let action_bits = (self.action_flags & Self::MAX_ACTION_FLAGS) << 3;
        buf[5] = dir_bits | action_bits;

        buf
    }

    /// Decodifica um array de 6 bytes brutos para um `MovementDelta`.
    /// Valida se as coordenadas obedecem ao limite estrito de 12 bits (< 4096).
    #[inline]
    pub fn decode(buf: &[u8; Self::PACKET_SIZE]) -> Result<Self, BitpackError> {
        let entity_id = EntityId(u16::from_le_bytes([buf[0], buf[1]]));

        // Reconstrução de PosX (12 bits)
        let x_low = buf[2] as u16;
        let x_high = (buf[3] & 0x0F) as u16;
        let x = x_low | (x_high << 8);

        // Reconstrução de PosY (12 bits)
        let y_low = ((buf[3] >> 4) & 0x0F) as u16;
        let y_high = (buf[4] as u16) << 4;
        let y = y_low | y_high;

        if x > Position::MAX_COORD || y > Position::MAX_COORD {
            return Err(BitpackError::InvalidCoordinates(x, y));
        }

        let direction = Direction::from_u8(buf[5] & 0x07)?;
        let action_flags = (buf[5] >> 3) & Self::MAX_ACTION_FLAGS;

        Ok(Self {
            entity_id,
            position: Position::new_unchecked(x, y),
            direction,
            action_flags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_size_is_strictly_6_bytes() {
        assert_eq!(MovementDelta::PACKET_SIZE, 6);
    }

    #[test]
    fn test_encode_decode_roundtrip_happy_path() {
        let original = MovementDelta::new(
            EntityId::new(1337),
            Position::new(512, 1024).unwrap(),
            Direction::SouthEast,
            15, // 5-bit action
        );

        let encoded = original.encode();
        assert_eq!(encoded.len(), 6);

        let decoded = MovementDelta::decode(&encoded).expect("Decodificação deve ter sucesso");
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_decode_boundary_extremes() {
        // Testando valores mínimos
        let min_delta = MovementDelta::new(
            EntityId::new(0),
            Position::new(0, 0).unwrap(),
            Direction::North,
            0,
        );
        let min_encoded = min_delta.encode();
        assert_eq!(MovementDelta::decode(&min_encoded).unwrap(), min_delta);

        // Testando valores máximos de 12 bits e 16 bits
        let max_delta = MovementDelta::new(
            EntityId::new(u16::MAX),
            Position::new(4095, 4095).unwrap(),
            Direction::NorthWest,
            MovementDelta::MAX_ACTION_FLAGS,
        );
        let max_encoded = max_delta.encode();
        assert_eq!(MovementDelta::decode(&max_encoded).unwrap(), max_delta);
    }

    #[test]
    fn test_lossless_roundtrip_random_sampling() {
        // Validação determinística de múltiplos valores
        for id in [1, 255, 1024, 65534] {
            for x in [0, 1, 63, 511, 2048, 4095] {
                for y in [0, 1, 127, 1023, 3000, 4095] {
                    for dir_raw in 0..8 {
                        let delta = MovementDelta::new(
                            EntityId::new(id),
                            Position::new(x, y).unwrap(),
                            Direction::from_u8(dir_raw).unwrap(),
                            7,
                        );
                        let bytes = delta.encode();
                        let recovered = MovementDelta::decode(&bytes).unwrap();
                        assert_eq!(delta, recovered, "Falha de fidelidade na coordenada ({x}, {y})");
                    }
                }
            }
        }
    }
}
