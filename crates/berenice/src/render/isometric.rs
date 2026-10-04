//! SPEC-0009: Projeção Isométrica 2.5D e conversão de coordenadas (Mundo <-> Tela).

use hades_core::types::Position;

use hades_core::types::Direction;

/// Dimensões canônicas padrão da célula 2.5D em pixels.
pub const DEFAULT_TILE_WIDTH: f32 = 24.0;
pub const DEFAULT_TILE_HEIGHT: f32 = 18.0;

/// Limites e valor padrão do ângulo de ataque (pitch) da câmera em graus.
pub const MIN_PITCH_DEG: f32 = 10.0;
pub const MAX_PITCH_DEG: f32 = 85.0;
pub const DEFAULT_PITCH_DEG: f32 = 48.590378;

/// Estrutura utilitária com parâmetros da projeção 2.5D regulável (yaw / pitch / zoom).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IsometricProjection {
    pub tile_width: f32,
    pub tile_height: f32,
    pub yaw: f32,   // Ângulo de rotação horizontal em radianos (0.0 = Norte padrão)
    pub pitch: f32, // Ângulo de ataque / inclinação vertical em radianos (10° a 85°)
    pub zoom: f32,  // Nível de ampliação (1.0 = padrão)
}

impl Default for IsometricProjection {
    fn default() -> Self {
        let pitch = (DEFAULT_TILE_HEIGHT / DEFAULT_TILE_WIDTH).asin();
        Self {
            tile_width: DEFAULT_TILE_WIDTH,
            tile_height: DEFAULT_TILE_HEIGHT,
            yaw: 0.0,
            pitch,
            zoom: 1.0,
        }
    }
}

impl IsometricProjection {
    pub fn new(tile_width: f32, tile_height: f32) -> Self {
        let pitch = (tile_height / tile_width).clamp(0.1, 1.0).asin();
        Self {
            tile_width,
            tile_height,
            yaw: 0.0,
            pitch,
            zoom: 1.0,
        }
    }

    /// Retorna o ângulo de ataque atual em graus (entre 30° e 150°).
    #[inline]
    pub fn pitch_deg(&self) -> f32 {
        self.pitch.to_degrees()
    }

    /// Define o ângulo de ataque da câmera em graus, limitado estritamente entre 30° e 150°.
    /// Atualiza proporcionalmente a altura das células (tile_height = tile_width * sin(pitch)).
    pub fn set_pitch_deg(&mut self, deg: f32) {
        let clamped = deg.clamp(MIN_PITCH_DEG, MAX_PITCH_DEG);
        self.pitch = clamped.to_radians();
        self.tile_height = (self.tile_width * self.pitch.sin()).max(4.0);
    }

    /// Converte coordenadas de célula do mundo (Grid X, Y) para coordenadas de tela (Screen X, Y)
    /// com suporte a rotação em torno da câmera (yaw) e zoom regulável estilo MMO clássico.
    #[inline]
    pub fn world_to_screen(
        &self,
        world_x: f32,
        world_y: f32,
        camera_origin_x: f32,
        camera_origin_y: f32,
    ) -> (f32, f32) {
        if self.yaw.abs() < 0.0001 {
            let screen_x = world_x * self.tile_width * self.zoom + camera_origin_x;
            let screen_y = -world_y * self.tile_height * self.zoom + camera_origin_y;
            (screen_x, screen_y)
        } else {
            let cos_yaw = self.yaw.cos();
            let sin_yaw = self.yaw.sin();

            // Rotaciona coordenadas de mundo pelo ângulo yaw da câmera
            let rx = world_x * cos_yaw - world_y * sin_yaw;
            let ry = world_x * sin_yaw + world_y * cos_yaw;

            let screen_x = rx * self.tile_width * self.zoom + camera_origin_x;
            let screen_y = -ry * self.tile_height * self.zoom + camera_origin_y;

            (screen_x, screen_y)
        }
    }

    /// Converte coordenadas de tela (Screen X, Y) de volta para célula do mundo (Grid X, Y).
    #[inline]
    pub fn screen_to_world(
        &self,
        screen_x: f32,
        screen_y: f32,
        camera_origin_x: f32,
        camera_origin_y: f32,
    ) -> Option<Position> {
        let rel_x = (screen_x - camera_origin_x) / (self.tile_width * self.zoom);
        let rel_y = -(screen_y - camera_origin_y) / (self.tile_height * self.zoom);

        let (world_x, world_y) = if self.yaw.abs() < 0.0001 {
            (rel_x, rel_y)
        } else {
            let cos_yaw = self.yaw.cos();
            let sin_yaw = self.yaw.sin();
            // Rotação inversa (-yaw)
            let wx = rel_x * cos_yaw + rel_y * sin_yaw;
            let wy = -rel_x * sin_yaw + rel_y * cos_yaw;
            (wx, wy)
        };

        let gx = world_x.round();
        let gy = world_y.round();

        if gx < 0.0 || gy < 0.0 {
            return None;
        }

        Position::new(gx as u16, gy as u16).ok()
    }

    /// Converte um vetor de deslocamento relativo à tela (WASD/Analógico) para direção canônica no mundo.
    #[inline]
    pub fn screen_input_to_world_delta(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        if self.yaw.abs() < 0.0001 {
            (screen_x, screen_y)
        } else {
            let cos_yaw = self.yaw.cos();
            let sin_yaw = self.yaw.sin();
            let wx = screen_x * cos_yaw + screen_y * sin_yaw;
            let wy = -screen_x * sin_yaw + screen_y * cos_yaw;
            (wx, wy)
        }
    }

    /// Converte a direção no plano de coordenadas do mundo para a perspectiva visual da câmera (yaw).
    /// Essencial para que sprites 2D e modelos exibam a face e a caminhada corretas
    /// em relação ao ponto de vista do observador na tela (evita o efeito "moonwalk" ao girar a câmera).
    #[inline]
    pub fn world_facing_to_camera(&self, facing: Direction) -> Direction {
        if self.yaw.abs() < 0.0001 {
            return facing;
        }
        let (dx, dy) = facing.delta();
        let cos_y = self.yaw.cos();
        let sin_y = self.yaw.sin();
        let rx = dx as f32 * cos_y - dy as f32 * sin_y;
        let ry = dx as f32 * sin_y + dy as f32 * cos_y;
        vector_to_direction(rx, ry)
    }
}

/// Converte um vetor bidimensional (dx, dy) na direção cardeal/colateral mais próxima (8 direções).
#[inline]
pub fn vector_to_direction(dx: f32, dy: f32) -> Direction {
    let angle = dy.atan2(dx);
    let octant = ((angle / (std::f32::consts::PI / 4.0)).round() as i32 + 8) % 8;
    match octant {
        0 => Direction::East,
        1 => Direction::NorthEast,
        2 => Direction::North,
        3 => Direction::NorthWest,
        4 => Direction::West,
        5 => Direction::SouthWest,
        6 => Direction::South,
        7 => Direction::SouthEast,
        _ => Direction::North,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_to_screen_and_screen_to_world_roundtrip() {
        let proj = IsometricProjection::default();
        let camera_x = 400.0;
        let camera_y = 300.0;

        let origin_cell = Position::new_unchecked(10, 10);
        let (sx, sy) = proj.world_to_screen(
            origin_cell.x as f32,
            origin_cell.y as f32,
            camera_x,
            camera_y,
        );

        let picked = proj
            .screen_to_world(sx, sy, camera_x, camera_y)
            .expect("should map back to world");
        assert_eq!(picked, origin_cell);

        // Teste com deslocamento assimétrico
        let cell = Position::new_unchecked(15, 8);
        let (sx, sy) = proj.world_to_screen(cell.x as f32, cell.y as f32, camera_x, camera_y);
        let picked = proj
            .screen_to_world(sx, sy, camera_x, camera_y)
            .expect("should map back to world");
        assert_eq!(picked, cell);
    }

    #[test]
    fn test_camera_rotation_and_zoom() {
        let proj = IsometricProjection {
            yaw: std::f32::consts::PI * 0.5, // 90 graus
            zoom: 1.5,
            ..IsometricProjection::default()
        };

        let origin = Position::new_unchecked(20, 20);
        let (sx, sy) = proj.world_to_screen(20.0, 20.0, 400.0, 300.0);
        let picked = proj
            .screen_to_world(sx, sy, 400.0, 300.0)
            .expect("un-rotate");
        assert_eq!(picked, origin);

        // Entrada na tela apontando para CIMA (sy = 1.0) com câmera virada 90 graus
        let (wx, wy) = proj.screen_input_to_world_delta(0.0, 1.0);
        assert!(wx > 0.9); // Aponta para Leste no mundo
        assert!(wy.abs() < 0.1);
    }

    #[test]
    fn test_vector_to_direction_octants() {
        assert_eq!(vector_to_direction(0.0, 1.0), Direction::North);
        assert_eq!(vector_to_direction(0.0, -1.0), Direction::South);
        assert_eq!(vector_to_direction(1.0, 0.0), Direction::East);
        assert_eq!(vector_to_direction(-1.0, 0.0), Direction::West);
        assert_eq!(vector_to_direction(1.0, 1.0), Direction::NorthEast);
        assert_eq!(vector_to_direction(-1.0, 1.0), Direction::NorthWest);
        assert_eq!(vector_to_direction(1.0, -1.0), Direction::SouthEast);
        assert_eq!(vector_to_direction(-1.0, -1.0), Direction::SouthWest);
    }

    #[test]
    fn test_camera_pitch_angle_of_attack_bounds_and_height() {
        let mut proj = IsometricProjection::default();
        assert!((proj.pitch_deg() - 48.59).abs() < 0.1);
        assert!((proj.tile_height - 18.0).abs() < 0.01);

        // Clamping inferior em 10 graus
        proj.set_pitch_deg(2.0);
        assert!((proj.pitch_deg() - 10.0).abs() < 0.01);
        // 24 * sin(10°) ≈ 4.17
        assert!((proj.tile_height - 4.17).abs() < 0.1);

        // Ângulo isométrico clássico (48.59°)
        proj.set_pitch_deg(48.590378);
        assert!((proj.pitch_deg() - 48.59).abs() < 0.1);
        assert!((proj.tile_height - 18.0).abs() < 0.1); // ≈ 24 * sin(48.59°)

        // Clamping superior em 85 graus
        proj.set_pitch_deg(120.0);
        assert!((proj.pitch_deg() - 85.0).abs() < 0.01);
        // 24 * sin(85°) ≈ 23.91
        assert!((proj.tile_height - 23.91).abs() < 0.1);
    }

    #[test]
    fn test_world_facing_to_camera_relative_to_yaw() {
        let mut proj = IsometricProjection::default();
        
        // Yaw = 0 (câmera padrão)
        assert_eq!(proj.world_facing_to_camera(Direction::North), Direction::North);
        assert_eq!(proj.world_facing_to_camera(Direction::East), Direction::East);
        assert_eq!(proj.world_facing_to_camera(Direction::South), Direction::South);
        assert_eq!(proj.world_facing_to_camera(Direction::West), Direction::West);

        // Yaw = 90° (câmera virada 90 graus à esquerda)
        proj.yaw = std::f32::consts::PI * 0.5;
        assert_eq!(proj.world_facing_to_camera(Direction::North), Direction::West);
        assert_eq!(proj.world_facing_to_camera(Direction::East), Direction::North);
        assert_eq!(proj.world_facing_to_camera(Direction::South), Direction::East);
        assert_eq!(proj.world_facing_to_camera(Direction::West), Direction::South);

        // Yaw = 180° (câmera invertida - previne moonwalk)
        proj.yaw = std::f32::consts::PI;
        assert_eq!(proj.world_facing_to_camera(Direction::North), Direction::South);
        assert_eq!(proj.world_facing_to_camera(Direction::East), Direction::West);
        assert_eq!(proj.world_facing_to_camera(Direction::South), Direction::North);
        assert_eq!(proj.world_facing_to_camera(Direction::West), Direction::East);

        // Yaw = 270° (-90°)
        proj.yaw = std::f32::consts::PI * 1.5;
        assert_eq!(proj.world_facing_to_camera(Direction::North), Direction::East);
        assert_eq!(proj.world_facing_to_camera(Direction::East), Direction::South);
        assert_eq!(proj.world_facing_to_camera(Direction::South), Direction::West);
        assert_eq!(proj.world_facing_to_camera(Direction::West), Direction::North);
    }
}

