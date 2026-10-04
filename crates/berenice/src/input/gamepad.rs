//! SPEC-0009: Abstração de controle universal (Gamepad / XInput / DirectInput / Switch / DualSense).

use super::intent::{
    analog_stick_to_direction, resolve_action_input, ActionIntent, FaceButton, HotbarModifier,
    MovementIntent,
};
use hades_core::types::Direction;

/// Estado instantâneo do controle físico do jogador.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GamepadState {
    pub left_stick_x: f32,
    pub left_stick_y: f32,
    pub right_stick_x: f32,
    pub right_stick_y: f32,

    pub trigger_l: bool,
    pub trigger_r: bool,

    pub btn_south: bool,
    pub btn_east: bool,
    pub btn_west: bool,
    pub btn_north: bool,
    pub btn_right_thumb: bool,

    pub stick_deadzone: f32,
}

impl GamepadState {
    pub fn new() -> Self {
        Self {
            stick_deadzone: 0.2,
            ..Default::default()
        }
    }

    /// Determina o modificador de hotbar ativo com base nos gatilhos.
    pub fn hotbar_modifier(&self) -> HotbarModifier {
        match (self.trigger_l, self.trigger_r) {
            (true, true) => HotbarModifier::TriggersBoth,
            (true, false) => HotbarModifier::TriggerL,
            (false, true) => HotbarModifier::TriggerR,
            (false, false) => HotbarModifier::Normal,
        }
    }

    /// Extrai a intenção de movimento do analógico esquerdo.
    pub fn movement_intent(&self) -> MovementIntent {
        if let Some(dir) =
            analog_stick_to_direction(self.left_stick_x, self.left_stick_y, self.stick_deadzone)
        {
            MovementIntent {
                is_moving: true,
                direction: dir,
                running: true,
            }
        } else {
            MovementIntent::default()
        }
    }

    /// Extrai a intenção de ação baseada no botão frontal pressionado e na direção que o jogador está virado.
    pub fn action_intent(&self, facing: Direction) -> ActionIntent {
        let modifier = self.hotbar_modifier();

        if self.btn_south {
            resolve_action_input(FaceButton::South, modifier, facing)
        } else if self.btn_west {
            resolve_action_input(FaceButton::West, modifier, facing)
        } else if self.btn_north {
            resolve_action_input(FaceButton::North, modifier, facing)
        } else if self.btn_east {
            resolve_action_input(FaceButton::East, modifier, facing)
        } else {
            ActionIntent::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_state_movement_and_action() {
        let mut state = GamepadState::new();
        state.left_stick_x = 0.9;
        state.left_stick_y = 0.0;
        state.btn_south = true;

        let move_intent = state.movement_intent();
        assert!(move_intent.is_moving);
        assert_eq!(move_intent.direction, Direction::East);

        let action = state.action_intent(move_intent.direction);
        assert_eq!(action, ActionIntent::BasicAttack);

        // Ativa gatilho L
        state.trigger_l = true;
        let action_skill = state.action_intent(move_intent.direction);
        assert_eq!(
            action_skill,
            ActionIntent::UseSkill {
                slot: 3,
                direction: Direction::East
            }
        );
    }
}
