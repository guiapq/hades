//! # Hades Core Engine
//!
//! Núcleo fundamental de tipos, serialização compacta e simulação determinística
//! para motores de MMORPGs 2D/2.5D.

pub mod aoi;
pub mod bitpacking;
pub mod collision;
pub mod items;
pub mod pathfinding;
pub mod tick;
pub mod types;

pub use aoi::{BucketCoord, BucketMigration, SpatialBucket, SpatialGrid, BUCKET_SIZE};
pub use bitpacking::{BitpackError, MovementDelta};
pub use collision::CollisionGrid;
pub use items::{
    drop_item_to_floor, pickup_floor_item, FloorItem, FloorItemManager, Inventory, ItemError,
    ItemInstance, ItemInstanceId,
};
pub use pathfinding::{
    find_nearest_walkable, find_path, octile_distance, PathResult, COST_CARDINAL, COST_DIAGONAL,
    DEFAULT_MAX_EXPANSIONS,
};
pub use tick::{Tick, TickClock, TICK_DURATION_MICROS, TICK_DURATION_MILLIS, TICK_RATE_HZ};
pub use types::{Direction, EntityId, Position, TypeError};


/// Operação Unitária Pura: Tenta mover uma entidade em uma direção respeitando o grid de colisão.
///
/// Não possui efeitos colaterais: recebe o estado anterior e retorna o novo estado e o delta gerado.
#[inline]
pub fn step_entity_movement(
    entity_id: EntityId,
    current_pos: Position,
    direction: Direction,
    grid: &CollisionGrid,
) -> (Position, Option<MovementDelta>) {
    let (dx, dy) = direction.delta();

    let target_x = current_pos.x as i32 + dx as i32;
    let target_y = current_pos.y as i32 + dy as i32;

    // Se estiver fora dos limites positivos do mundo
    if target_x < 0 || target_y < 0 {
        return (current_pos, None);
    }

    let target_pos = Position::new_unchecked(target_x as u16, target_y as u16);

    // Valida se o destino é andável
    if !grid.is_walkable(target_pos) {
        return (current_pos, None);
    }

    let delta = MovementDelta::new(entity_id, target_pos, direction, 0);
    (target_pos, Some(delta))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_operation_movement_success() {
        let grid = CollisionGrid::new(100, 100, true);
        let id = EntityId::new(42);
        let start = Position::new(10, 10).unwrap();

        let (new_pos, delta) = step_entity_movement(id, start, Direction::East, &grid);

        assert_eq!(new_pos, Position::new(11, 10).unwrap());
        assert!(delta.is_some());
        let d = delta.unwrap();
        assert_eq!(d.entity_id, id);
        assert_eq!(d.position, new_pos);
        assert_eq!(d.direction, Direction::East);
    }

    #[test]
    fn test_unit_operation_movement_blocked_by_wall() {
        let mut grid = CollisionGrid::new(100, 100, true);
        let id = EntityId::new(42);
        let start = Position::new(10, 10).unwrap();

        // Bloqueia a célula a Leste (11, 10)
        grid.set_walkable(Position::new(11, 10).unwrap(), false);

        let (new_pos, delta) = step_entity_movement(id, start, Direction::East, &grid);

        // Movimento deve ser rejeitado: a posição permanece a original e nenhum delta é emitido
        assert_eq!(new_pos, start);
        assert!(delta.is_none());
    }
}
