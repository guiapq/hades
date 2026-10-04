//! SPEC-0017: Módulo de Navegação Contínua e Pathfinding por Mouse (Seamless Mouse Navigation).

use hades_core::collision::CollisionGrid;
use hades_core::pathfinding::{find_path, DEFAULT_MAX_EXPANSIONS};
use hades_core::types::Position;

/// Gerenciador de navegação e condução contínua por mouse.
#[derive(Debug, Clone, Default)]
pub struct MouseNavigation {
    /// Se a navegação por mouse está ativa guiando o personagem.
    pub is_active: bool,
    /// Se o botão do mouse está sendo mantido pressionado (arraste contínuo).
    pub is_dragging: bool,
    /// Coordenada final almejada no mapa.
    pub target_pos: Option<Position>,
    /// Fila ordenada de passos intermediários do caminho calculado.
    pub waypoints: Vec<Position>,
    /// Coordenada onde o marcador visual de clique deve ser desenhado no chão.
    pub click_marker_pos: Option<Position>,
    /// Temporizador decrescente de exibição do marcador de clique (em segundos).
    pub click_marker_timer: f32,
}

impl MouseNavigation {
    pub const MARKER_DURATION: f32 = 0.6; // 600ms de animação do anel visual

    pub fn new() -> Self {
        Self::default()
    }

    /// Disparado quando o botão esquerdo do mouse é pressionado pela primeira vez.
    pub fn on_mouse_down(
        &mut self,
        grid: &CollisionGrid,
        current_pos: Position,
        target_pos: Position,
    ) -> bool {
        self.is_dragging = true;
        self.set_target(grid, current_pos, target_pos)
    }

    /// Disparado a cada frame enquanto o botão esquerdo do mouse permanece pressionado.
    /// Atualiza o caminho de forma contínua e fluida conforme o cursor percorre o cenário.
    pub fn on_mouse_drag(
        &mut self,
        grid: &CollisionGrid,
        current_pos: Position,
        new_target: Position,
    ) -> bool {
        if !self.is_dragging {
            return false;
        }

        // Se o cursor ainda estiver na mesma célula, não recalcula
        if self.target_pos == Some(new_target) {
            return false;
        }

        self.set_target(grid, current_pos, new_target)
    }

    /// Disparado quando o botão esquerdo do mouse é solto.
    pub fn on_mouse_up(&mut self) {
        self.is_dragging = false;
        // O personagem continua andando até o último destino clicado
    }

    /// Interrompe a navegação por mouse imediatamente (ex: ao usar WASD ou Gamepad).
    pub fn interrupt(&mut self) {
        self.is_active = false;
        self.is_dragging = false;
        self.target_pos = None;
        self.waypoints.clear();
        self.click_marker_pos = None;
        self.click_marker_timer = 0.0;
    }

    /// Atualiza o destino calculando a rota pelo A*.
    pub fn set_target(
        &mut self,
        grid: &CollisionGrid,
        current_pos: Position,
        target_pos: Position,
    ) -> bool {
        if let Some(result) = find_path(grid, current_pos, target_pos, DEFAULT_MAX_EXPANSIONS) {
            if !result.waypoints.is_empty() {
                self.target_pos = result.waypoints.last().copied();
                self.waypoints = result.waypoints;
                self.is_active = true;
                self.click_marker_pos = self.target_pos;
                self.click_marker_timer = Self::MARKER_DURATION;
                return true;
            }
        }
        false
    }

    /// Avança o próximo waypoint quando o personagem atinge o tile atual.
    pub fn advance_step(&mut self, arrived_pos: Position) -> Option<Position> {
        if let Some(&first) = self.waypoints.first() {
            if first == arrived_pos {
                self.waypoints.remove(0);
            }
        }

        if self.waypoints.is_empty() {
            self.is_active = false;
            self.target_pos = None;
            None
        } else {
            self.waypoints.first().copied()
        }
    }

    /// Atualiza o temporizador da animação visual a cada frame.
    pub fn tick(&mut self, dt_seconds: f32) {
        if self.click_marker_timer > 0.0 {
            self.click_marker_timer = (self.click_marker_timer - dt_seconds).max(0.0);
            if self.click_marker_timer <= 0.0 {
                self.click_marker_pos = None;
            }
        }
    }
}

/// Encontra a célula caminhável adjacente ao alvo (dentro do alcance permitido para a arma)
/// que minimiza a distância de aproximação do atacante.
pub fn find_best_attack_cell(
    grid: &CollisionGrid,
    attacker_pos: Position,
    target_pos: Position,
    weapon_range: u16,
) -> Option<Position> {
    let mut best_cell = None;
    let mut best_dist_sq = u32::MAX;
    let range_i = weapon_range as i32;

    for dx in -range_i..=range_i {
        for dy in -range_i..=range_i {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = target_pos.x as i32 + dx;
            let ny = target_pos.y as i32 + dy;
            if nx >= 0 && ny >= 0 && nx < grid.width as i32 && ny < grid.height as i32 {
                let p = Position::new_unchecked(nx as u16, ny as u16);
                if grid.is_walkable(p) {
                    let d_x = attacker_pos.x as i32 - p.x as i32;
                    let d_y = attacker_pos.y as i32 - p.y as i32;
                    let dist_sq = (d_x * d_x + d_y * d_y) as u32;
                    if dist_sq < best_dist_sq {
                        best_dist_sq = dist_sq;
                        best_cell = Some(p);
                    }
                }
            }
        }
    }
    best_cell
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_navigation_click_creates_path() {
        let grid = CollisionGrid::new(20, 20, true);
        let mut nav = MouseNavigation::new();

        let start = Position::new_unchecked(5, 5);
        let goal = Position::new_unchecked(5, 8);

        let success = nav.on_mouse_down(&grid, start, goal);
        assert!(success);
        assert!(nav.is_active);
        assert!(nav.is_dragging);
        assert_eq!(nav.target_pos, Some(goal));
        assert_eq!(nav.waypoints.len(), 3);
        assert_eq!(nav.waypoints[0], Position::new_unchecked(5, 6));

        // Avança passos
        let next = nav.advance_step(Position::new_unchecked(5, 6));
        assert_eq!(next, Some(Position::new_unchecked(5, 7)));

        let next2 = nav.advance_step(Position::new_unchecked(5, 7));
        assert_eq!(next2, Some(Position::new_unchecked(5, 8)));

        let end = nav.advance_step(Position::new_unchecked(5, 8));
        assert_eq!(end, None);
        assert!(!nav.is_active);
    }

    #[test]
    fn test_mouse_navigation_interrupted_by_manual_control() {
        let grid = CollisionGrid::new(20, 20, true);
        let mut nav = MouseNavigation::new();

        let start = Position::new_unchecked(2, 2);
        let goal = Position::new_unchecked(8, 8);
        nav.on_mouse_down(&grid, start, goal);
        assert!(nav.is_active);

        nav.interrupt();
        assert!(!nav.is_active);
        assert!(!nav.is_dragging);
        assert!(nav.waypoints.is_empty());
    }

    #[test]
    fn test_mouse_navigation_dragging_updates_target() {
        let grid = CollisionGrid::new(30, 30, true);
        let mut nav = MouseNavigation::new();

        let start = Position::new_unchecked(10, 10);
        let goal1 = Position::new_unchecked(15, 10);
        nav.on_mouse_down(&grid, start, goal1);
        assert_eq!(nav.target_pos, Some(goal1));

        // Arrasta o mouse para outro destino
        let goal2 = Position::new_unchecked(10, 18);
        let updated = nav.on_mouse_drag(&grid, start, goal2);
        assert!(updated);
        assert_eq!(nav.target_pos, Some(goal2));
        assert_eq!(nav.waypoints.last().copied(), Some(goal2));
    }

    #[test]
    fn test_find_best_attack_cell_melee_range_1() {
        let grid = CollisionGrid::new(30, 30, true);
        let attacker = Position::new_unchecked(10, 10);
        let target = Position::new_unchecked(15, 10);

        // Para arma melee de 1 célula (espada/adaga), o atacante deve se aproximar
        // até a célula adjacente mais próxima: (14, 10)
        let best = find_best_attack_cell(&grid, attacker, target, 1);
        assert_eq!(best, Some(Position::new_unchecked(14, 10)));
    }

    #[test]
    fn test_find_best_attack_cell_spear_range_2() {
        let grid = CollisionGrid::new(30, 30, true);
        let attacker = Position::new_unchecked(10, 10);
        let target = Position::new_unchecked(15, 10);

        // Para lança com alcance 2 células, a célula ideal mais próxima é (13, 10)
        let best = find_best_attack_cell(&grid, attacker, target, 2);
        assert_eq!(best, Some(Position::new_unchecked(13, 10)));
    }

    #[test]
    fn test_find_best_attack_cell_with_obstacle() {
        let mut grid = CollisionGrid::new(30, 30, true);
        let attacker = Position::new_unchecked(10, 10);
        let target = Position::new_unchecked(15, 10);

        // Se a célula direta (14, 10) estiver bloqueada por colisão:
        grid.set_walkable(Position::new_unchecked(14, 10), false);

        // Deve escolher uma das células adjacentes livres (ex: 14, 9 ou 14, 11)
        let best = find_best_attack_cell(&grid, attacker, target, 1).unwrap();
        assert_ne!(best, Position::new_unchecked(14, 10));
        assert!(grid.is_walkable(best));
        assert_eq!(best.chebyshev_distance(target), 1);
    }
}
