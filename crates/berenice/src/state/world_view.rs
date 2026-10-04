//! SPEC-0009: Estado local da simulação no cliente Berenice (WorldView).

use hades_core::bitpacking::MovementDelta;
use hades_core::types::{Direction, EntityId, Position};
use std::collections::HashMap;

/// Informações sobre uma entidade observada na tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedEntity {
    pub entity_id: EntityId,
    pub position: Position,
    pub facing: Direction,
    pub is_moving: bool,
}

/// Estado do mundo do ponto de vista do cliente local.
#[derive(Debug, Default)]
pub struct WorldView {
    pub local_entity_id: Option<EntityId>,
    pub local_position: Position,
    pub local_facing: Direction,
    pub nearby_entities: HashMap<EntityId, ObservedEntity>,
}

impl WorldView {
    pub fn new(local_id: EntityId, start_pos: Position) -> Self {
        Self {
            local_entity_id: Some(local_id),
            local_position: start_pos,
            local_facing: Direction::South,
            nearby_entities: HashMap::new(),
        }
    }

    /// Gera o pacote MovementDelta de 6 bytes a partir da posição local e do alvo desejado.
    pub fn create_movement_delta(
        &mut self,
        target_pos: Position,
        facing: Direction,
        running: bool,
    ) -> Option<MovementDelta> {
        let local_id = self.local_entity_id?;
        let action_flags = if running { 0x01 } else { 0x00 };
        let delta = MovementDelta::new(local_id, target_pos, facing, action_flags);
        self.local_position = target_pos;
        self.local_facing = facing;
        Some(delta)
    }

    /// Atualiza uma entidade recebida da réplica do servidor Hades.
    pub fn update_remote_entity(&mut self, entity: ObservedEntity) {
        self.nearby_entities.insert(entity.entity_id, entity);
    }

    /// Remove uma entidade que saiu do campo de visão (AoI).
    pub fn remove_remote_entity(&mut self, entity_id: EntityId) {
        self.nearby_entities.remove(&entity_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_view_generates_6_byte_movement_delta() {
        let mut world = WorldView::new(EntityId::new(42), Position::new_unchecked(100, 200));

        let next_pos = Position::new_unchecked(101, 200);
        let delta = world
            .create_movement_delta(next_pos, Direction::East, true)
            .expect("should generate delta");

        assert_eq!(delta.entity_id, EntityId::new(42));
        assert_eq!(delta.position, Position::new_unchecked(101, 200));
        assert_eq!(delta.direction, Direction::East);
        assert_eq!(delta.action_flags, 0x01);

        let bytes = delta.encode();
        assert_eq!(bytes.len(), 6);
    }
}
