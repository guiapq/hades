//! SPEC-0009: Hub unificado de entrada do cliente Berenice.

pub mod gamepad;
pub mod intent;
pub mod keyboard;
pub mod mouse_nav;

pub use gamepad::GamepadState;
pub use intent::{
    analog_stick_to_direction, resolve_action_input, ActionIntent, FaceButton, HotbarModifier,
    MovementIntent,
};
pub use keyboard::{resolve_keyboard_action, resolve_keyboard_movement, DirectionalKeys};
pub use mouse_nav::{find_best_attack_cell, MouseNavigation};

use hades_core::types::Direction;

/// Hub central que agrega periféricos e produz os inputs finais do jogador.
#[derive(Debug, Default)]
pub struct InputHub {
    pub gamepad: GamepadState,
    pub keyboard_keys: DirectionalKeys,
    pub mouse_nav: MouseNavigation,
    pub shift_pressed: bool,
    pub facing_direction: Direction,
}


impl InputHub {
    pub fn new() -> Self {
        Self {
            gamepad: GamepadState::new(),
            facing_direction: Direction::South,
            ..Default::default()
        }
    }

    /// Obtém a intenção de movimento canônica (câmera com yaw 0).
    pub fn poll_movement(&mut self) -> MovementIntent {
        self.poll_movement_relative(0.0)
    }

    /// Obtém a intenção de movimento relativa à orientação atual da câmera (yaw em radianos).
    /// Converte inputs direcionais de tela (WASD/Analógico) para o espaço de mundo rotacionado.
    pub fn poll_movement_relative(&mut self, camera_yaw: f32) -> MovementIntent {
        let deadzone = self.gamepad.stick_deadzone;
        let gx = self.gamepad.left_stick_x;
        let gy = self.gamepad.left_stick_y;
        let mag_sq = gx * gx + gy * gy;

        if mag_sq >= deadzone * deadzone {
            let cos_y = camera_yaw.cos();
            let sin_y = camera_yaw.sin();
            let wx = gx * cos_y + gy * sin_y;
            let wy = -gx * sin_y + gy * cos_y;
            if let Some(dir) = analog_stick_to_direction(wx, wy, 0.0) {
                self.facing_direction = dir;
                return MovementIntent {
                    is_moving: true,
                    direction: dir,
                    running: true,
                };
            }
        }

        let sx = (self.keyboard_keys.right as i32 - self.keyboard_keys.left as i32) as f32;
        let sy = (self.keyboard_keys.up as i32 - self.keyboard_keys.down as i32) as f32;

        if sx != 0.0 || sy != 0.0 {
            let cos_y = camera_yaw.cos();
            let sin_y = camera_yaw.sin();
            let wx = sx * cos_y + sy * sin_y;
            let wy = -sx * sin_y + sy * cos_y;
            if let Some(dir) = analog_stick_to_direction(wx, wy, 0.0) {
                self.facing_direction = dir;
                return MovementIntent {
                    is_moving: true,
                    direction: dir,
                    running: self.shift_pressed,
                };
            }
        }

        MovementIntent::default()
    }

    /// Obtém a intenção de ação do controle físico.
    pub fn poll_action(&self) -> ActionIntent {
        self.gamepad.action_intent(self.facing_direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_movement_relative_keyboard() {
        let mut hub = InputHub::new();

        // Tecla W (cima na tela) com yaw = 0 (Norte canônico)
        hub.keyboard_keys.up = true;
        let mov = hub.poll_movement_relative(0.0);
        assert!(mov.is_moving);
        assert_eq!(mov.direction, Direction::North);

        // Tecla W (cima na tela) com yaw = 90 graus (PI / 2 radianos) -> Leste no mundo
        let mov_rot = hub.poll_movement_relative(std::f32::consts::FRAC_PI_2);
        assert!(mov_rot.is_moving);
        assert_eq!(mov_rot.direction, Direction::East);

        // Tecla W com yaw = 180 graus (PI radianos) -> Sul no mundo
        let mov_180 = hub.poll_movement_relative(std::f32::consts::PI);
        assert!(mov_180.is_moving);
        assert_eq!(mov_180.direction, Direction::South);
    }

    #[test]
    fn test_poll_movement_relative_gamepad() {
        let mut hub = InputHub::new();

        // Analógico apontando para a direita na tela (X = 1.0, Y = 0.0)
        hub.gamepad.left_stick_x = 1.0;
        hub.gamepad.left_stick_y = 0.0;

        // Com yaw = 0, direita na tela é Leste no mundo
        let mov_0 = hub.poll_movement_relative(0.0);
        assert!(mov_0.is_moving);
        assert_eq!(mov_0.direction, Direction::East);

        // Com yaw = 90 graus (PI / 2), direita na tela vira Sul no mundo
        let mov_90 = hub.poll_movement_relative(std::f32::consts::FRAC_PI_2);
        assert!(mov_90.is_moving);
        assert_eq!(mov_90.direction, Direction::South);
    }
}
