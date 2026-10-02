//! Primitivas fundamentais de simulação para o Hades.

use thiserror::Error;

/// Erros decorrentes de validação de coordenadas e limites.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TypeError {
    #[error("Coordenada fora do limite de 12 bits (máximo 4095): x={0}, y={1}")]
    PositionOutOfBounds(u16, u16),

    #[error("Direção inválida: valor numérico {0} não mapeia para nenhuma direção de 8 vias")]
    InvalidDirection(u8),
}

/// Identificador único de uma entidade ativa em um mapa/instância.
/// Representado em 16 bits para comportar até 65.535 entidades simultâneas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EntityId(pub u16);

impl EntityId {
    pub const NULL: Self = Self(0);

    #[inline(always)]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    #[inline(always)]
    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

/// Posição discreta 2D dentro do mundo.
/// Cada eixo é limitado a 12 bits (0 a 4095), garantindo mapas de até 4096 x 4096 tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub const MAX_COORD: u16 = 4095;

    /// Cria uma nova posição validando os limites de 12 bits.
    #[inline]
    pub fn new(x: u16, y: u16) -> Result<Self, TypeError> {
        if x > Self::MAX_COORD || y > Self::MAX_COORD {
            return Err(TypeError::PositionOutOfBounds(x, y));
        }
        Ok(Self { x, y })
    }

    /// Cria uma posição sem checagem de limites (unchecked) para loops críticos internos.
    ///
    /// # Safety
    /// O chamador deve garantir que x <= 4095 e y <= 4095.
    #[inline(always)]
    pub const fn new_unchecked(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    /// Distância de Manhattan entre duas coordenadas (|x1 - x2| + |y1 - y2|).
    /// Operação puramente aritmética em registradores de CPU.
    #[inline(always)]
    pub fn manhattan_distance(self, other: Self) -> u16 {
        self.x.abs_diff(other.x) + self.y.abs_diff(other.y)
    }

    /// Distância de Chebyshev (passos em grid de 8 direções: max(|dx|, |dy|)).
    #[inline(always)]
    pub fn chebyshev_distance(self, other: Self) -> u16 {
        self.x.abs_diff(other.x).max(self.y.abs_diff(other.y))
    }
}

/// 8 Direções cardeais e colaterais padrão para jogos 2D/2.5D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    North = 0,
    NorthEast = 1,
    East = 2,
    SouthEast = 3,
    South = 4,
    SouthWest = 5,
    West = 6,
    NorthWest = 7,
}

impl Direction {
    /// Converte a direção para seu valor ordinal (0 a 7).
    #[inline(always)]
    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    /// Converte um valor de 3 bits (0 a 7) para Direction.
    #[inline]
    pub fn from_u8(val: u8) -> Result<Self, TypeError> {
        match val & 0b111 {
            0 => Ok(Self::North),
            1 => Ok(Self::NorthEast),
            2 => Ok(Self::East),
            3 => Ok(Self::SouthEast),
            4 => Ok(Self::South),
            5 => Ok(Self::SouthWest),
            6 => Ok(Self::West),
            7 => Ok(Self::NorthWest),
            _ => unreachable!(),
        }
    }

    /// Retorna o delta discreto (dx, dy) para avanço de 1 passo na grade.
    #[inline(always)]
    pub const fn delta(self) -> (i16, i16) {
        match self {
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::East => (1, 0),
            Self::SouthEast => (1, 1),
            Self::South => (0, 1),
            Self::SouthWest => (-1, 1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, -1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_valid_and_bounds() {
        assert!(Position::new(0, 0).is_ok());
        assert!(Position::new(4095, 4095).is_ok());
        assert_eq!(
            Position::new(4096, 100),
            Err(TypeError::PositionOutOfBounds(4096, 100))
        );
        assert_eq!(
            Position::new(100, 4096),
            Err(TypeError::PositionOutOfBounds(100, 4096))
        );
    }

    #[test]
    fn test_distances() {
        let p1 = Position::new_unchecked(10, 10);
        let p2 = Position::new_unchecked(15, 22);

        assert_eq!(p1.manhattan_distance(p2), 5 + 12);
        assert_eq!(p1.chebyshev_distance(p2), 12);
    }

    #[test]
    fn test_direction_roundtrip() {
        for i in 0..8 {
            let dir = Direction::from_u8(i).unwrap();
            assert_eq!(dir.to_u8(), i);
        }
    }
}
