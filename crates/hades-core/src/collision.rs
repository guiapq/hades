//! Grid de colisão denso por bitset (1 bit por tile).
//!
//! Conforme definido no Potato Budget e SPEC-0001:
//! - 1 bit por célula: `1 = andável (walkable)`, `0 = bloqueado (wall/water)`
//! - Um mapa de 1024x1024 consome exatamente 128 KB de RAM.
//! - Consultas de colisão em tempo O(1) usando operações aritméticas e bitshifts.

use crate::types::Position;

/// Grid de colisão 2D compacto com representação em bitset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionGrid {
    pub width: u16,
    pub height: u16,
    /// Bitset linear contíguo: cada bit representa uma célula.
    data: Box<[u64]>,
}

impl CollisionGrid {
    /// Cria um novo grid de colisão inicializado com todas as células livres (walkable)
    /// ou bloqueadas dependendo do parâmetro `default_walkable`.
    pub fn new(width: u16, height: u16, default_walkable: bool) -> Self {
        let total_cells = (width as usize) * (height as usize);
        let num_words = total_cells.div_ceil(64);

        let initial_val = if default_walkable { u64::MAX } else { 0 };
        let data = vec![initial_val; num_words].into_boxed_slice();

        Self {
            width,
            height,
            data,
        }
    }

    /// Retorna o tamanho em bytes consumido pelo buffer de colisão em memória RAM.
    #[inline]
    pub fn memory_footprint_bytes(&self) -> usize {
        self.data.len() * std::mem::size_of::<u64>()
    }

    /// Verifica se a coordenada (x, y) está dentro dos limites e é andável.
    /// Operação O(1) em registradores de CPU sem alocações.
    #[inline]
    pub fn is_walkable(&self, pos: Position) -> bool {
        if pos.x >= self.width || pos.y >= self.height {
            return false;
        }

        let bit_index = (pos.y as usize) * (self.width as usize) + (pos.x as usize);
        let word_idx = bit_index / 64;
        let bit_offset = bit_index % 64;

        // Se o bit for 1, a célula é andável.
        (self.data[word_idx] & (1u64 << bit_offset)) != 0
    }

    /// Altera o estado de colisão de uma célula.
    #[inline]
    pub fn set_walkable(&mut self, pos: Position, walkable: bool) -> bool {
        if pos.x >= self.width || pos.y >= self.height {
            return false;
        }

        let bit_index = (pos.y as usize) * (self.width as usize) + (pos.x as usize);
        let word_idx = bit_index / 64;
        let bit_offset = bit_index % 64;

        if walkable {
            self.data[word_idx] |= 1u64 << bit_offset;
        } else {
            self.data[word_idx] &= !(1u64 << bit_offset);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collision_grid_memory_footprint() {
        // Mapa clássico de 1024x1024
        let grid = CollisionGrid::new(1024, 1024, true);

        // 1024 * 1024 bits = 1.048.576 bits = 131.072 bytes = 128 KB
        assert_eq!(grid.memory_footprint_bytes(), 128 * 1024);
    }

    #[test]
    fn test_collision_set_and_is_walkable() {
        let mut grid = CollisionGrid::new(100, 100, true);
        let pos = Position::new_unchecked(15, 30);

        assert!(
            grid.is_walkable(pos),
            "Célula deveria estar livre inicialmente"
        );

        // Bloqueia a célula (parede)
        grid.set_walkable(pos, false);
        assert!(
            !grid.is_walkable(pos),
            "Célula deveria estar bloqueada após set_walkable(false)"
        );

        // Libera novamente
        grid.set_walkable(pos, true);
        assert!(
            grid.is_walkable(pos),
            "Célula deveria estar livre novamente"
        );
    }

    #[test]
    fn test_out_of_bounds_is_not_walkable() {
        let grid = CollisionGrid::new(50, 50, true);

        // Fora dos limites da largura/altura do mapa
        assert!(!grid.is_walkable(Position::new_unchecked(50, 20)));
        assert!(!grid.is_walkable(Position::new_unchecked(20, 50)));
        assert!(!grid.is_walkable(Position::new_unchecked(4095, 4095)));
    }
}
