//! SPEC-0006: Particionamento Espacial em Buckets O(1) e Gestão de Área de Interesse (AoI).
//!
//! Organiza o mapa em setores de 32x32 tiles, permitindo consultas espaciais 3x3
//! sem varredura global (zero O(N^2)) e suporte a hibernação (Spatial Economics).

use crate::types::{EntityId, Position};

pub const BUCKET_SIZE: u16 = 32;

/// Coordenada discreta de um bucket no grid espacial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BucketCoord {
    pub bx: u16,
    pub by: u16,
}

impl BucketCoord {
    #[inline(always)]
    pub const fn new(bx: u16, by: u16) -> Self {
        Self { bx, by }
    }
}

/// Um setor espacial contendo entidades ativas e contagem de jogadores observadores.
#[derive(Debug, Clone, Default)]
pub struct SpatialBucket {
    pub entities: Vec<EntityId>,
    pub player_count: u16,
}

/// Representa a transição de um bucket para outro quando uma entidade se move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BucketMigration {
    pub old_bucket: BucketCoord,
    pub new_bucket: BucketCoord,
}

/// Grid de particionamento espacial de buckets O(1).
#[derive(Debug, Clone)]
pub struct SpatialGrid {
    pub width_tiles: u16,
    pub height_tiles: u16,
    pub buckets_x: u16,
    pub buckets_y: u16,
    buckets: Box<[SpatialBucket]>,
    /// Rastreia o bucket atual de cada entidade ativa para updates O(1)
    entity_positions: Vec<Option<BucketCoord>>,
}

impl SpatialGrid {
    /// Cria um novo grid espacial para um mapa de dimensões dadas em tiles.
    pub fn new(width_tiles: u16, height_tiles: u16) -> Self {
        let buckets_x = width_tiles.div_ceil(BUCKET_SIZE).max(1);
        let buckets_y = height_tiles.div_ceil(BUCKET_SIZE).max(1);
        let total_buckets = (buckets_x as usize) * (buckets_y as usize);

        let mut buckets = Vec::with_capacity(total_buckets);
        for _ in 0..total_buckets {
            buckets.push(SpatialBucket::default());
        }

        Self {
            width_tiles,
            height_tiles,
            buckets_x,
            buckets_y,
            buckets: buckets.into_boxed_slice(),
            entity_positions: Vec::new(),
        }
    }

    /// Converte coordenada contínua de tile para a coordenada discreta do bucket (32x32).
    #[inline(always)]
    pub const fn tile_to_bucket(pos: Position) -> BucketCoord {
        BucketCoord {
            bx: pos.x / BUCKET_SIZE,
            by: pos.y / BUCKET_SIZE,
        }
    }

    #[inline(always)]
    fn bucket_index(&self, coord: BucketCoord) -> usize {
        (coord.by as usize) * (self.buckets_x as usize) + (coord.bx as usize)
    }

    /// Insere uma entidade no grid espacial. Operação O(1).
    pub fn insert_entity(&mut self, entity_id: EntityId, pos: Position, is_player: bool) {
        let bcoord = Self::tile_to_bucket(pos);
        if bcoord.bx >= self.buckets_x || bcoord.by >= self.buckets_y {
            return;
        }

        let idx = self.bucket_index(bcoord);
        let bucket = &mut self.buckets[idx];
        bucket.entities.push(entity_id);
        if is_player {
            bucket.player_count = bucket.player_count.saturating_add(1);
        }

        let id_idx = entity_id.0 as usize;
        if id_idx >= self.entity_positions.len() {
            self.entity_positions.resize(id_idx + 1, None);
        }
        self.entity_positions[id_idx] = Some(bcoord);
    }

    /// Remove uma entidade do grid espacial. Operação O(1).
    pub fn remove_entity(&mut self, entity_id: EntityId, is_player: bool) {
        let id_idx = entity_id.0 as usize;
        let Some(bcoord) = self.entity_positions.get(id_idx).copied().flatten() else {
            return;
        };

        let idx = self.bucket_index(bcoord);
        let bucket = &mut self.buckets[idx];
        if let Some(pos_in_bucket) = bucket.entities.iter().position(|&id| id == entity_id) {
            bucket.entities.swap_remove(pos_in_bucket);
        }
        if is_player {
            bucket.player_count = bucket.player_count.saturating_sub(1);
        }

        self.entity_positions[id_idx] = None;
    }

    /// Atualiza a posição de uma entidade. Se cruzou a fronteira do bucket de 32x32 tiles,
    /// move a entidade entre buckets e retorna a migração realizada.
    pub fn update_entity_position(
        &mut self,
        entity_id: EntityId,
        new_pos: Position,
        is_player: bool,
    ) -> Option<BucketMigration> {
        let id_idx = entity_id.0 as usize;
        let current_bcoord = self.entity_positions.get(id_idx).copied().flatten()?;
        let target_bcoord = Self::tile_to_bucket(new_pos);

        if current_bcoord == target_bcoord {
            return None;
        }

        // Removendo do bucket antigo
        let old_idx = self.bucket_index(current_bcoord);
        let old_bucket = &mut self.buckets[old_idx];
        if let Some(pos) = old_bucket.entities.iter().position(|&id| id == entity_id) {
            old_bucket.entities.swap_remove(pos);
        }
        if is_player {
            old_bucket.player_count = old_bucket.player_count.saturating_sub(1);
        }

        // Inserindo no novo bucket
        let new_idx = self.bucket_index(target_bcoord);
        let new_bucket = &mut self.buckets[new_idx];
        new_bucket.entities.push(entity_id);
        if is_player {
            new_bucket.player_count = new_bucket.player_count.saturating_add(1);
        }

        self.entity_positions[id_idx] = Some(target_bcoord);

        Some(BucketMigration {
            old_bucket: current_bcoord,
            new_bucket: target_bcoord,
        })
    }

    /// Retorna todas as entidades presentes na Área de Interesse (raio 3x3 de buckets ao redor da posição).
    /// Operação O(1) sobre exatamente no máximo 9 buckets.
    pub fn query_aoi(&self, pos: Position, out: &mut Vec<EntityId>) {
        out.clear();
        let center = Self::tile_to_bucket(pos);

        let min_x = center.bx.saturating_sub(1);
        let max_x = (center.bx + 1).min(self.buckets_x.saturating_sub(1));

        let min_y = center.by.saturating_sub(1);
        let max_y = (center.by + 1).min(self.buckets_y.saturating_sub(1));

        for by in min_y..=max_y {
            for bx in min_x..=max_x {
                let idx = self.bucket_index(BucketCoord::new(bx, by));
                let bucket = &self.buckets[idx];
                out.extend_from_slice(&bucket.entities);
            }
        }
    }

    /// Spatial Economics: Verifica se um bucket está Ativo (possui pelo menos 1 jogador no raio 3x3).
    /// Se retornar `false`, monstros neste bucket podem ser hibernados.
    pub fn is_bucket_active(&self, bcoord: BucketCoord) -> bool {
        if bcoord.bx >= self.buckets_x || bcoord.by >= self.buckets_y {
            return false;
        }

        let min_x = bcoord.bx.saturating_sub(1);
        let max_x = (bcoord.bx + 1).min(self.buckets_x.saturating_sub(1));

        let min_y = bcoord.by.saturating_sub(1);
        let max_y = (bcoord.by + 1).min(self.buckets_y.saturating_sub(1));

        for by in min_y..=max_y {
            for bx in min_x..=max_x {
                let idx = self.bucket_index(BucketCoord::new(bx, by));
                if self.buckets[idx].player_count > 0 {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_grid_bucket_coordinates() {
        assert_eq!(
            SpatialGrid::tile_to_bucket(Position::new_unchecked(0, 0)),
            BucketCoord::new(0, 0)
        );
        assert_eq!(
            SpatialGrid::tile_to_bucket(Position::new_unchecked(31, 31)),
            BucketCoord::new(0, 0)
        );
        assert_eq!(
            SpatialGrid::tile_to_bucket(Position::new_unchecked(32, 32)),
            BucketCoord::new(1, 1)
        );
        assert_eq!(
            SpatialGrid::tile_to_bucket(Position::new_unchecked(64, 96)),
            BucketCoord::new(2, 3)
        );
    }

    #[test]
    fn test_entity_bucket_insertion_and_removal() {
        let mut grid = SpatialGrid::new(256, 256);
        let id1 = EntityId::new(10);
        let pos1 = Position::new_unchecked(10, 10);

        grid.insert_entity(id1, pos1, true);

        let mut aoi = Vec::new();
        grid.query_aoi(pos1, &mut aoi);
        assert_eq!(aoi, vec![id1]);

        grid.remove_entity(id1, true);
        grid.query_aoi(pos1, &mut aoi);
        assert!(aoi.is_empty());
    }

    #[test]
    fn test_aoi_3x3_query() {
        let mut grid = SpatialGrid::new(512, 512);

        // Bucket (1, 1) é de 32 a 63 em X e Y
        let center_pos = Position::new_unchecked(40, 40);

        let id_center = EntityId::new(1);
        let id_neighbor = EntityId::new(2);
        let id_distant = EntityId::new(3);

        grid.insert_entity(id_center, center_pos, false);
        // Vizinho no bucket (2, 1) -> X=70, Y=40
        grid.insert_entity(id_neighbor, Position::new_unchecked(70, 40), false);
        // Distante no bucket (5, 5) -> X=160, Y=160 (fora do raio 3x3)
        grid.insert_entity(id_distant, Position::new_unchecked(160, 160), false);

        let mut visible = Vec::new();
        grid.query_aoi(center_pos, &mut visible);

        assert!(visible.contains(&id_center));
        assert!(visible.contains(&id_neighbor));
        assert!(
            !visible.contains(&id_distant),
            "Entidade distante não deve estar no AoI 3x3"
        );
    }

    #[test]
    fn test_spatial_economics_hibernation() {
        let mut grid = SpatialGrid::new(256, 256);
        let monster_coord = BucketCoord::new(3, 3);

        // Sem jogadores por perto: deve hibernar!
        assert!(!grid.is_bucket_active(monster_coord));

        // Jogador entra no bucket adjacente (2, 3)
        let player_id = EntityId::new(99);
        grid.insert_entity(
            player_id,
            Position::new_unchecked(2 * 32 + 5, 3 * 32 + 5),
            true,
        );

        // Agora o setor do monstro acorda!
        assert!(grid.is_bucket_active(monster_coord));

        // Jogador se desconecta / sai
        grid.remove_entity(player_id, true);
        assert!(
            !grid.is_bucket_active(monster_coord),
            "Deve voltar a hibernar após saída do jogador"
        );
    }

    #[test]
    fn test_bucket_migration() {
        let mut grid = SpatialGrid::new(256, 256);
        let id = EntityId::new(42);

        // Inicia no bucket (0, 0)
        grid.insert_entity(id, Position::new_unchecked(10, 10), false);

        // Move dentro do mesmo bucket: sem migração
        let no_migration = grid.update_entity_position(id, Position::new_unchecked(20, 20), false);
        assert_eq!(no_migration, None);

        // Move para o bucket (1, 0): cruza a fronteira X >= 32
        let migration = grid.update_entity_position(id, Position::new_unchecked(35, 20), false);
        assert_eq!(
            migration,
            Some(BucketMigration {
                old_bucket: BucketCoord::new(0, 0),
                new_bucket: BucketCoord::new(1, 0),
            })
        );
    }
}
