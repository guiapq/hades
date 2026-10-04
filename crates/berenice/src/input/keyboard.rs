//! SPEC-0009: Mapeamento de Teclado e Mouse no estilo Tree of Savior.

use super::intent::{ActionIntent, MovementIntent};
use hades_core::types::Direction;

/// Estado das teclas direcionais (WASD ou Setas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirectionalKeys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl DirectionalKeys {
    /// Converte o estado das 4 teclas em uma das 8 direções (ou None se parado ou opostos se cancelando).
    pub fn to_direction(&self) -> Option<Direction> {
        let vert = match (self.up, self.down) {
            (true, false) => 1i32,
            (false, true) => -1i32,
            _ => 0i32,
        };
        let horiz = match (self.right, self.left) {
            (true, false) => 1i32,
            (false, true) => -1i32,
            _ => 0i32,
        };

        match (horiz, vert) {
            (0, 0) => None,
            (1, 0) => Some(Direction::East),
            (1, 1) => Some(Direction::NorthEast),
            (0, 1) => Some(Direction::North),
            (-1, 1) => Some(Direction::NorthWest),
            (-1, 0) => Some(Direction::West),
            (-1, -1) => Some(Direction::SouthWest),
            (0, -1) => Some(Direction::South),
            (1, -1) => Some(Direction::SouthEast),
            _ => None,
        }
    }
}

/// Mapeamento de ações do teclado estilo ToS:
/// Z: Ataque básico
/// X: Salto / Esquiva / Cancelar
/// C: Interagir
/// 1..9: Habilidades diretas
pub fn resolve_keyboard_action(key: char, facing: Direction) -> ActionIntent {
    match key {
        'z' | 'Z' => ActionIntent::BasicAttack,
        'c' | 'C' => ActionIntent::Interact,
        'x' | 'X' => ActionIntent::Cancel,
        '1'..='9' => {
            let slot = (key as u8) - b'0';
            ActionIntent::UseSkill {
                slot,
                direction: facing,
            }
        }
        _ => ActionIntent::None,
    }
}

/// Converte teclas direcionais em MovementIntent.
pub fn resolve_keyboard_movement(keys: DirectionalKeys, shift_pressed: bool) -> MovementIntent {
    if let Some(dir) = keys.to_direction() {
        MovementIntent {
            is_moving: true,
            direction: dir,
            running: shift_pressed,
        }
    } else {
        MovementIntent::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directional_keys_to_8_directions() {
        // Apenas Cima
        let k = DirectionalKeys {
            up: true,
            down: false,
            left: false,
            right: false,
        };
        assert_eq!(k.to_direction(), Some(Direction::North));

        // Cima + Direita
        let k = DirectionalKeys {
            up: true,
            down: false,
            left: false,
            right: true,
        };
        assert_eq!(k.to_direction(), Some(Direction::NorthEast));

        // Teclas opostas se anulam (Cima + Baixo)
        let k = DirectionalKeys {
            up: true,
            down: true,
            left: false,
            right: false,
        };
        assert_eq!(k.to_direction(), None);

        // Baixo + Esquerda
        let k = DirectionalKeys {
            up: false,
            down: true,
            left: true,
            right: false,
        };
        assert_eq!(k.to_direction(), Some(Direction::SouthWest));
    }

    #[test]
    fn test_keyboard_actions_tos_mode() {
        let facing = Direction::East;
        assert_eq!(
            resolve_keyboard_action('z', facing),
            ActionIntent::BasicAttack
        );
        assert_eq!(resolve_keyboard_action('c', facing), ActionIntent::Interact);
        assert_eq!(
            resolve_keyboard_action('1', facing),
            ActionIntent::UseSkill {
                slot: 1,
                direction: facing
            }
        );
    }
}
