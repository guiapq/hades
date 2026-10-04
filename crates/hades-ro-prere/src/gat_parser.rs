//! SPEC-0004: Parser de mapas binários .gat para CollisionGrid do Hades.
//!
//! Converte a matriz de células no formato clássico .gat diretamente para o bitset
//! compacto de 1 bit por tile do Hades.

use hades_core::collision::CollisionGrid;
use hades_core::types::Position;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GatParseError {
    #[error("Arquivo muito pequeno para conter o cabeçalho GAT (mínimo 16 bytes, recebido {0})")]
    BufferTooShort(usize),

    #[error("Cabeçalho mágico inválido: esperado 'GRAT', recebido {0:?}")]
    InvalidMagic([u8; 4]),

    #[error("Dimensões do mapa excedem o limite suportado pelo motor (máximo 4095x4095): {0}x{1}")]
    DimensionsTooLarge(u32, u32),

    #[error("Payload de células incompleto: esperado {0} bytes, recebido {1} bytes")]
    TruncatedCellData(usize, usize),
}

/// Identifica a natureza da célula no formato clássico .gat de 2.5D RPGs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GatCellType {
    WalkableSnipable = 0,
    NonWalkableNonSnipable = 1,
    NonWalkableSnipableWater = 2,
    WalkableSnipableWater = 3,
    NonWalkableSnipable = 4,
    NonWalkableSnipableCliff = 5,
    WalkableSnipableSpecial = 6,
    Unknown(u32),
}

impl GatCellType {
    #[inline]
    pub fn from_u32(val: u32) -> Self {
        match val {
            0 => Self::WalkableSnipable,
            1 => Self::NonWalkableNonSnipable,
            2 => Self::NonWalkableSnipableWater,
            3 => Self::WalkableSnipableWater,
            4 => Self::NonWalkableSnipable,
            5 => Self::NonWalkableSnipableCliff,
            6 => Self::WalkableSnipableSpecial,
            other => Self::Unknown(other),
        }
    }

    /// Retorna se a entidade pode andar sobre essa célula.
    #[inline]
    pub fn is_walkable(self) -> bool {
        matches!(
            self,
            Self::WalkableSnipable | Self::WalkableSnipableWater | Self::WalkableSnipableSpecial
        )
    }
}

pub const GAT_MAGIC: &[u8; 4] = b"GRAT";
pub const GAT_HEADER_SIZE: usize = 14;
pub const CELL_SIZE_BYTES: usize = 20;

/// Lê um buffer binário de arquivo `.gat` e converte diretamente para um `CollisionGrid` do Hades.
pub fn parse_gat(bytes: &[u8]) -> Result<CollisionGrid, GatParseError> {
    if bytes.len() < GAT_HEADER_SIZE {
        return Err(GatParseError::BufferTooShort(bytes.len()));
    }

    let magic: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
    if &magic != GAT_MAGIC {
        return Err(GatParseError::InvalidMagic(magic));
    }

    let width = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
    let height = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]);

    if width > Position::MAX_COORD as u32 || height > Position::MAX_COORD as u32 {
        return Err(GatParseError::DimensionsTooLarge(width, height));
    }

    let total_cells = (width as usize) * (height as usize);
    let expected_cell_bytes = total_cells * CELL_SIZE_BYTES;
    let available_cell_bytes = bytes.len() - GAT_HEADER_SIZE;

    if available_cell_bytes < expected_cell_bytes {
        return Err(GatParseError::TruncatedCellData(
            expected_cell_bytes,
            available_cell_bytes,
        ));
    }

    // Inicializa grid todo bloqueado (false) e ativa os bits andáveis
    let mut grid = CollisionGrid::new(width as u16, height as u16, false);
    let mut offset = GAT_HEADER_SIZE;

    for gat_y in 0..height {
        let game_y = (height - 1 - gat_y) as u16;
        for x in 0..width {
            // Os últimos 4 bytes da struct de 20 bytes de cada célula é o cell_type (u32 LE)
            let type_offset = offset + 16;
            let cell_type_raw = u32::from_le_bytes([
                bytes[type_offset],
                bytes[type_offset + 1],
                bytes[type_offset + 2],
                bytes[type_offset + 3],
            ]);

            let cell_type = GatCellType::from_u32(cell_type_raw);
            if cell_type.is_walkable() {
                grid.set_walkable(Position::new_unchecked(x as u16, game_y), true);
            }

            offset += CELL_SIZE_BYTES;
        }
    }

    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_too_short_or_invalid_magic() {
        assert_eq!(
            parse_gat(&[0u8; 10]),
            Err(GatParseError::BufferTooShort(10))
        );

        let mut invalid_magic = [0u8; 14];
        invalid_magic[0..4].copy_from_slice(b"ABCD");
        assert_eq!(
            parse_gat(&invalid_magic),
            Err(GatParseError::InvalidMagic(*b"ABCD"))
        );
    }

    #[test]
    fn test_parse_synthetic_gat_map() {
        let width = 2u32;
        let height = 2u32;
        let mut buffer = Vec::new();

        // 1. Header canônico (14 bytes)
        buffer.extend_from_slice(b"GRAT"); // Magic (4 bytes)
        buffer.push(1u8); // Major (1 byte)
        buffer.push(2u8); // Minor (1 byte)
        buffer.extend_from_slice(&width.to_le_bytes()); // Width (4 bytes)
        buffer.extend_from_slice(&height.to_le_bytes()); // Height (4 bytes)

        // 2. 4 Células (2x2):
        // (0,0): tipo 0 (walkable)
        // (1,0): tipo 1 (parede / bloqueado)
        // (0,1): tipo 3 (água andável)
        // (1,1): tipo 5 (abismo / bloqueado)
        let cell_types = [0u32, 1u32, 3u32, 5u32];

        for &ct in &cell_types {
            // 16 bytes de floats de altitude
            buffer.extend_from_slice(&[0u8; 16]);
            // 4 bytes de cell_type
            buffer.extend_from_slice(&ct.to_le_bytes());
        }

        let grid = parse_gat(&buffer).expect("Deve parsear mapa sintético com sucesso");
        assert_eq!(grid.width, 2);
        assert_eq!(grid.height, 2);

        // (0,0): walkable
        assert!(grid.is_walkable(Position::new_unchecked(0, 0)));
        // (1,0): blocked
        assert!(!grid.is_walkable(Position::new_unchecked(1, 0)));
        // (0,1): walkable
        assert!(grid.is_walkable(Position::new_unchecked(0, 1)));
        // (1,1): blocked
        assert!(!grid.is_walkable(Position::new_unchecked(1, 1)));
    }
}
