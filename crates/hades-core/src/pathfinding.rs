//! SPEC-0017: Algoritmo de Pathfinding A* e resolução de células caminháveis.
//!
//! Implementação eficiente em 8 direções com heurística Octile, prevenção de corte
//! de quinas e teto estrito de expansões para conformidade com o Potato Budget.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::collision::CollisionGrid;
use crate::types::Position;

/// Custo base para movimentos ortogonais (1.0 multiplicativo normalizado para inteiro).
pub const COST_CARDINAL: u32 = 10;
/// Custo base para movimentos diagonais (~1.414 multiplicativo normalizado para inteiro).
pub const COST_DIAGONAL: u32 = 14;
/// Teto máximo de expansões de nós no A* para garantir tempo de execução bounded.
pub const DEFAULT_MAX_EXPANSIONS: usize = 512;

/// Nó interno da fila de prioridade do A*.
#[derive(Copy, Clone, Eq, PartialEq)]
struct AStarNode {
    pos: Position,
    f_score: u32,
    g_score: u32,
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Inverte para min-heap (menor f_score tem prioridade)
        other
            .f_score
            .cmp(&self.f_score)
            .then_with(|| other.g_score.cmp(&self.g_score))
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Resultado retornado pelo pathfinder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathResult {
    /// Lista ordenada de waypoints a percorrer (excluindo a posição inicial, incluindo o destino).
    pub waypoints: Vec<Position>,
    /// `true` se o destino exato foi alcançado; `false` se foi uma aproximação truncada.
    pub reachable: bool,
}

/// Calcula a distância Octile entre duas posições em 8 direções.
#[inline]
pub fn octile_distance(from: Position, to: Position) -> u32 {
    let dx = (from.x as i32 - to.x as i32).abs() as u32;
    let dy = (from.y as i32 - to.y as i32).abs() as u32;
    let min = dx.min(dy);
    let max = dx.max(dy);
    COST_CARDINAL * max + (COST_DIAGONAL - COST_CARDINAL) * min
}

/// Encontra a célula caminhável mais próxima de uma posição alvo dentro de um raio de busca.
pub fn find_nearest_walkable(
    grid: &CollisionGrid,
    target: Position,
    max_radius: u16,
) -> Option<Position> {
    if grid.is_walkable(target) {
        return Some(target);
    }

    for r in 1..=max_radius {
        let r_i32 = r as i32;
        let tx = target.x as i32;
        let ty = target.y as i32;

        // Itera o perímetro do quadrado de raio r
        for dx in -r_i32..=r_i32 {
            for dy in -r_i32..=r_i32 {
                if dx.abs() != r_i32 && dy.abs() != r_i32 {
                    continue; // Apenas o anel externo
                }
                let nx = tx + dx;
                let ny = ty + dy;
                if nx >= 0 && ny >= 0 && nx < grid.width as i32 && ny < grid.height as i32 {
                    let p = Position::new_unchecked(nx as u16, ny as u16);
                    if grid.is_walkable(p) {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}

/// Executa busca de caminho A* 8-way sobre o CollisionGrid.
pub fn find_path(
    grid: &CollisionGrid,
    start: Position,
    mut goal: Position,
    max_expansions: usize,
) -> Option<PathResult> {
    if start == goal {
        return Some(PathResult {
            waypoints: Vec::new(),
            reachable: true,
        });
    }

    // Se o destino for colidente, ajusta para o vizinho caminhável mais próximo
    if !grid.is_walkable(goal) {
        if let Some(near) = find_nearest_walkable(grid, goal, 5) {
            goal = near;
            if start == goal {
                return Some(PathResult {
                    waypoints: Vec::new(),
                    reachable: true,
                });
            }
        } else {
            return None;
        }
    }

    let mut open_set = BinaryHeap::with_capacity(64);
    // Armazena (g_score, veio_de)
    let mut node_data: HashMap<(u16, u16), (u32, Option<Position>)> = HashMap::with_capacity(128);

    let h_start = octile_distance(start, goal);
    open_set.push(AStarNode {
        pos: start,
        f_score: h_start,
        g_score: 0,
    });
    node_data.insert((start.x, start.y), (0, None));

    let mut closest_pos = start;
    let mut closest_h = h_start;
    let mut expansions = 0;
    let mut reached = false;

    // Deslocamentos das 8 direções: (dx, dy, custo, é_diagonal)
    const NEIGHBORS: [(i32, i32, u32, bool); 8] = [
        (0, 1, COST_CARDINAL, false),   // Norte
        (0, -1, COST_CARDINAL, false),  // Sul
        (1, 0, COST_CARDINAL, false),   // Leste
        (-1, 0, COST_CARDINAL, false),  // Oeste
        (1, 1, COST_DIAGONAL, true),    // Nordeste
        (1, -1, COST_DIAGONAL, true),   // Sudeste
        (-1, -1, COST_DIAGONAL, true),  // Sudoeste
        (-1, 1, COST_DIAGONAL, true),   // Noroeste
    ];

    while let Some(current) = open_set.pop() {
        if current.pos == goal {
            reached = true;
            closest_pos = current.pos;
            break;
        }

        expansions += 1;
        if expansions > max_expansions {
            break;
        }

        let curr_g = match node_data.get(&(current.pos.x, current.pos.y)) {
            Some(&(g, _)) if current.g_score <= g => g,
            _ => continue,
        };

        let curr_x = current.pos.x as i32;
        let curr_y = current.pos.y as i32;

        for &(dx, dy, cost, is_diag) in &NEIGHBORS {
            let next_x = curr_x + dx;
            let next_y = curr_y + dy;

            if next_x < 0 || next_y < 0 || next_x >= grid.width as i32 || next_y >= grid.height as i32 {
                continue;
            }

            let next_pos = Position::new_unchecked(next_x as u16, next_y as u16);
            if !grid.is_walkable(next_pos) {
                continue;
            }

            // Prevenção de corte de quina em diagonais
            if is_diag {
                let side1 = Position::new_unchecked((curr_x + dx) as u16, curr_y as u16);
                let side2 = Position::new_unchecked(curr_x as u16, (curr_y + dy) as u16);
                if !grid.is_walkable(side1) || !grid.is_walkable(side2) {
                    continue;
                }
            }

            let tentative_g = curr_g + cost;
            let recorded_g = node_data.get(&(next_pos.x, next_pos.y)).map(|(g, _)| *g);

            if recorded_g.map_or(true, |old_g| tentative_g < old_g) {
                let h = octile_distance(next_pos, goal);
                let f = tentative_g + h;

                node_data.insert((next_pos.x, next_pos.y), (tentative_g, Some(current.pos)));
                open_set.push(AStarNode {
                    pos: next_pos,
                    f_score: f,
                    g_score: tentative_g,
                });

                if h < closest_h {
                    closest_h = h;
                    closest_pos = next_pos;
                }
            }
        }
    }

    if closest_pos == start {
        return None;
    }

    // Reconstrói caminho a partir de closest_pos
    let mut waypoints = Vec::new();
    let mut curr = closest_pos;

    while let Some(&(_, Some(prev))) = node_data.get(&(curr.x, curr.y)) {
        waypoints.push(curr);
        curr = prev;
        if curr == start {
            break;
        }
    }

    waypoints.reverse();

    Some(PathResult {
        waypoints,
        reachable: reached,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_straight_line_path() {
        let grid = CollisionGrid::new(20, 20, true);
        let start = Position::new_unchecked(5, 5);
        let goal = Position::new_unchecked(5, 9);

        let res = find_path(&grid, start, goal, DEFAULT_MAX_EXPANSIONS).expect("Path found");
        assert!(res.reachable);
        assert_eq!(res.waypoints.len(), 4);
        assert_eq!(res.waypoints.last().copied(), Some(goal));
    }

    #[test]
    fn test_diagonal_path() {
        let grid = CollisionGrid::new(20, 20, true);
        let start = Position::new_unchecked(2, 2);
        let goal = Position::new_unchecked(5, 5);

        let res = find_path(&grid, start, goal, DEFAULT_MAX_EXPANSIONS).expect("Path found");
        assert!(res.reachable);
        assert_eq!(res.waypoints.len(), 3);
        assert_eq!(res.waypoints[0], Position::new_unchecked(3, 3));
        assert_eq!(res.waypoints[1], Position::new_unchecked(4, 4));
        assert_eq!(res.waypoints[2], Position::new_unchecked(5, 5));
    }

    #[test]
    fn test_obstacle_avoidance_around_wall() {
        let mut grid = CollisionGrid::new(20, 20, true);
        // Parede vertical em X=5 de Y=2 até Y=7
        for y in 2..=7 {
            grid.set_walkable(Position::new_unchecked(5, y), false);
        }

        let start = Position::new_unchecked(3, 5);
        let goal = Position::new_unchecked(7, 5);

        let res = find_path(&grid, start, goal, DEFAULT_MAX_EXPANSIONS).expect("Path found");
        assert!(res.reachable);
        // Nenhum waypoint deve colidir com a parede
        for wp in &res.waypoints {
            assert!(grid.is_walkable(*wp), "Waypoint {wp:?} atingiu parede!");
        }
        assert_eq!(res.waypoints.last().copied(), Some(goal));
    }

    #[test]
    fn test_corner_cutting_is_blocked() {
        let mut grid = CollisionGrid::new(10, 10, true);
        // Bloqueia (5, 4) e (4, 5)
        grid.set_walkable(Position::new_unchecked(5, 4), false);

        let start = Position::new_unchecked(4, 4);
        let goal = Position::new_unchecked(5, 5);

        let res = find_path(&grid, start, goal, DEFAULT_MAX_EXPANSIONS).expect("Path found");
        assert!(res.reachable);
        // Não deve ir direto (4,4) -> (5,5) em 1 passo diagonal porque a quina (5,4) está bloqueada
        assert!(res.waypoints.len() > 1);
    }

    #[test]
    fn test_unwalkable_goal_resolves_to_nearest() {
        let mut grid = CollisionGrid::new(20, 20, true);
        let wall = Position::new_unchecked(10, 10);
        grid.set_walkable(wall, false);

        let start = Position::new_unchecked(10, 5);
        let res = find_path(&grid, start, wall, DEFAULT_MAX_EXPANSIONS).expect("Path to near wall");
        // O destino alcançado deve ser vizinho adjacente da parede
        let end = res.waypoints.last().copied().unwrap();
        assert!(grid.is_walkable(end));
        let dx = (end.x as i32 - wall.x as i32).abs();
        let dy = (end.y as i32 - wall.y as i32).abs();
        assert!(dx <= 1 && dy <= 1);
    }
}
