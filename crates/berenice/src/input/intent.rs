//! SPEC-0009: Definições de intenção de controle, cálculo analógico e mapeamento de hotbar.

use hades_core::types::Direction;

/// Estado do modificador de paleta de habilidades (estilo Cross-Hotbar ToS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HotbarModifier {
    #[default]
    Normal,
    TriggerL,
    TriggerR,
    TriggersBoth,
}

/// Botões de ação frontais do controle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceButton {
    South, // A / Cruz
    East,  // B / Círculo
    West,  // X / Quadrado
    North, // Y / Triângulo
}

/// Intenção de movimento gerada pelo jogador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MovementIntent {
    pub is_moving: bool,
    pub direction: Direction,
    pub running: bool,
}

/// Ação de combate ou interação desejada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionIntent {
    #[default]
    None,
    BasicAttack,
    Interact,
    Cancel,
    UseSkill {
        slot: u8,
        direction: Direction,
    },
    UseItem {
        slot: u8,
    },
}

/// Converte um vetor analógico (X, Y) em uma das 8 direções do Hades com suporte a deadzone.
/// Coordenadas: X+ = Leste, X- = Oeste, Y+ = Norte, Y- = Sul.
pub fn analog_stick_to_direction(x: f32, y: f32, deadzone: f32) -> Option<Direction> {
    let magnitude_sq = x * x + y * y;
    let deadzone_sq = deadzone * deadzone;

    if magnitude_sq < deadzone_sq {
        return None;
    }

    // Ângulo em radianos de -PI a +PI
    let angle = y.atan2(x);
    // Converter radianos para graus [0..360)
    let degrees = angle.to_degrees();
    let normalized_deg = if degrees < 0.0 {
        degrees + 360.0
    } else {
        degrees
    };

    // Mapeamento em fatias de 45 graus (com offset de 22.5 graus para centralizar os eixos)
    // 0 deg = Leste (East)
    // 45 deg = Nordeste (NorthEast)
    // 90 deg = Norte (North)
    // 135 deg = Noroeste (NorthWest)
    // 180 deg = Oeste (West)
    // 225 deg = Sudoeste (SouthWest)
    // 270 deg = Sul (South)
    // 315 deg = Sudeste (SouthEast)
    let sector = ((normalized_deg + 22.5) / 45.0).floor() as u32 % 8;

    let dir = match sector {
        0 => Direction::East,
        1 => Direction::NorthEast,
        2 => Direction::North,
        3 => Direction::NorthWest,
        4 => Direction::West,
        5 => Direction::SouthWest,
        6 => Direction::South,
        7 => Direction::SouthEast,
        _ => Direction::South,
    };

    Some(dir)
}

/// Resolve o botão frontal pressionado e o modificador de gatilho para uma intenção de ação.
pub fn resolve_action_input(
    button: FaceButton,
    modifier: HotbarModifier,
    facing: Direction,
) -> ActionIntent {
    match modifier {
        HotbarModifier::Normal => match button {
            FaceButton::South => ActionIntent::BasicAttack,
            FaceButton::East => ActionIntent::Cancel,
            FaceButton::West => ActionIntent::UseSkill {
                slot: 1,
                direction: facing,
            },
            FaceButton::North => ActionIntent::UseSkill {
                slot: 2,
                direction: facing,
            },
        },
        HotbarModifier::TriggerL => match button {
            FaceButton::South => ActionIntent::UseSkill {
                slot: 3,
                direction: facing,
            },
            FaceButton::East => ActionIntent::UseSkill {
                slot: 4,
                direction: facing,
            },
            FaceButton::West => ActionIntent::UseSkill {
                slot: 5,
                direction: facing,
            },
            FaceButton::North => ActionIntent::UseSkill {
                slot: 6,
                direction: facing,
            },
        },
        HotbarModifier::TriggerR => match button {
            FaceButton::South => ActionIntent::UseSkill {
                slot: 7,
                direction: facing,
            },
            FaceButton::East => ActionIntent::UseSkill {
                slot: 8,
                direction: facing,
            },
            FaceButton::West => ActionIntent::UseSkill {
                slot: 9,
                direction: facing,
            },
            FaceButton::North => ActionIntent::UseSkill {
                slot: 10,
                direction: facing,
            },
        },
        HotbarModifier::TriggersBoth => match button {
            FaceButton::South => ActionIntent::UseSkill {
                slot: 11,
                direction: facing,
            },
            FaceButton::East => ActionIntent::UseSkill {
                slot: 12,
                direction: facing,
            },
            FaceButton::West => ActionIntent::UseSkill {
                slot: 13,
                direction: facing,
            },
            FaceButton::North => ActionIntent::UseSkill {
                slot: 14,
                direction: facing,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stick_deadzone() {
        assert_eq!(analog_stick_to_direction(0.05, 0.05, 0.2), None);
        assert_eq!(analog_stick_to_direction(0.0, 0.0, 0.15), None);
    }

    #[test]
    fn test_stick_cardinal_and_diagonal_directions() {
        let dz = 0.15;
        // Leste (+X, 0)
        assert_eq!(
            analog_stick_to_direction(1.0, 0.0, dz),
            Some(Direction::East)
        );
        // Nordeste (+X, +Y)
        assert_eq!(
            analog_stick_to_direction(0.7, 0.7, dz),
            Some(Direction::NorthEast)
        );
        // Norte (0, +Y)
        assert_eq!(
            analog_stick_to_direction(0.0, 1.0, dz),
            Some(Direction::North)
        );
        // Noroeste (-X, +Y)
        assert_eq!(
            analog_stick_to_direction(-0.7, 0.7, dz),
            Some(Direction::NorthWest)
        );
        // Oeste (-X, 0)
        assert_eq!(
            analog_stick_to_direction(-1.0, 0.0, dz),
            Some(Direction::West)
        );
        // Sudoeste (-X, -Y)
        assert_eq!(
            analog_stick_to_direction(-0.7, -0.7, dz),
            Some(Direction::SouthWest)
        );
        // Sul (0, -Y)
        assert_eq!(
            analog_stick_to_direction(0.0, -1.0, dz),
            Some(Direction::South)
        );
        // Sudeste (+X, -Y)
        assert_eq!(
            analog_stick_to_direction(0.7, -0.7, dz),
            Some(Direction::SouthEast)
        );
    }

    #[test]
    fn test_cross_hotbar_action_mapping() {
        let facing = Direction::North;

        // Sem modificadores
        let act = resolve_action_input(FaceButton::South, HotbarModifier::Normal, facing);
        assert_eq!(act, ActionIntent::BasicAttack);

        let act = resolve_action_input(FaceButton::West, HotbarModifier::Normal, facing);
        assert_eq!(
            act,
            ActionIntent::UseSkill {
                slot: 1,
                direction: facing
            }
        );

        // Com Gatilho Esquerdo (L1/L2)
        let act = resolve_action_input(FaceButton::South, HotbarModifier::TriggerL, facing);
        assert_eq!(
            act,
            ActionIntent::UseSkill {
                slot: 3,
                direction: facing
            }
        );

        // Com Gatilho Direito (R1/R2)
        let act = resolve_action_input(FaceButton::North, HotbarModifier::TriggerR, facing);
        assert_eq!(
            act,
            ActionIntent::UseSkill {
                slot: 10,
                direction: facing
            }
        );

        // Com Ambos Gatilhos
        let act = resolve_action_input(FaceButton::East, HotbarModifier::TriggersBoth, facing);
        assert_eq!(
            act,
            ActionIntent::UseSkill {
                slot: 12,
                direction: facing
            }
        );
    }
}
