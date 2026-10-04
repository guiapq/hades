//! SPEC-0016: Núcleo de simulação do World Server, gestão espacial AoI e movimento.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use hades_core::aoi::SpatialGrid;
use hades_core::collision::CollisionGrid;
use hades_core::types::{Direction, EntityId, Position};

use crate::messages::{ClientWorldMsg, ServerWorldMsg};

/// Representa a sessão ativa de um jogador no mundo.
pub struct WorldSession {
    pub entity_id: EntityId,
    pub aid: u32,
    pub gid: u32,
    pub name: String,
    pub position: Position,
    pub facing: Direction,
    pub speed: u16,
    pub tx: mpsc::UnboundedSender<ServerWorldMsg>,
}

/// Estado interno do mundo.
struct WorldInner {
    spatial_grid: SpatialGrid,
    collision_grid: CollisionGrid,
    sessions: HashMap<EntityId, WorldSession>,
}

/// Gerenciador do Mundo (World Manager) thread-safe e clonável.
#[derive(Clone)]
pub struct WorldManager {
    next_entity_id: Arc<AtomicU16>,
    inner: Arc<RwLock<WorldInner>>,
}

impl WorldManager {
    /// Cria uma nova instância do gerenciador de mundo.
    pub fn new(collision_grid: CollisionGrid) -> Self {
        let width = collision_grid.width;
        let height = collision_grid.height;
        let spatial_grid = SpatialGrid::new(width, height);

        Self {
            next_entity_id: Arc::new(AtomicU16::new(100)),
            inner: Arc::new(RwLock::new(WorldInner {
                spatial_grid,
                collision_grid,
                sessions: HashMap::new(),
            })),
        }
    }

    /// Retorna o timestamp atual em milissegundos.
    pub fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Registra um novo jogador no mundo e conecta sua fila de saída.
    pub async fn register_player(
        &self,
        aid: u32,
        gid: u32,
        name: String,
        initial_pos: Position,
        tx: mpsc::UnboundedSender<ServerWorldMsg>,
    ) -> (EntityId, ServerWorldMsg) {
        let raw_id = self.next_entity_id.fetch_add(1, Ordering::Relaxed);
        let entity_id = EntityId::new(raw_id);
        let speed = 150;

        let mut inner = self.inner.write().await;

        // 1. Insere o jogador no grid espacial AoI
        inner
            .spatial_grid
            .insert_entity(entity_id, initial_pos, true);

        // 2. Consulta entidades já presentes no raio AoI 3x3
        let mut aoi_entities = Vec::with_capacity(16);
        inner.spatial_grid.query_aoi(initial_pos, &mut aoi_entities);

        // 3. Notifica entidades existentes sobre o novo jogador (entity_spawn)
        let spawn_for_others = ServerWorldMsg::EntitySpawn {
            id: entity_id.as_u16(),
            name: name.clone(),
            job: 0,
            pos_x: initial_pos.x,
            pos_y: initial_pos.y,
            dir: 0,
            speed,
        };

        for &peer_id in &aoi_entities {
            if peer_id != entity_id {
                if let Some(peer) = inner.sessions.get(&peer_id) {
                    let _ = peer.tx.send(spawn_for_others.clone());
                }
            }
        }

        let welcome_msg = ServerWorldMsg::EnterWorldOk {
            aid,
            gid,
            name: name.clone(),
            map_name: "prontera.gat".to_string(),
            pos_x: initial_pos.x,
            pos_y: initial_pos.y,
            dir: 0,
            sex: 0,
            speed,
        };

        // Envia confirmação de entrada como primeira mensagem
        let _ = tx.send(welcome_msg.clone());

        // 4. Envia para o novo jogador a lista de entidades já presentes no seu AoI
        for &peer_id in &aoi_entities {
            if peer_id != entity_id {
                if let Some(peer) = inner.sessions.get(&peer_id) {
                    let peer_spawn = ServerWorldMsg::EntitySpawn {
                        id: peer.entity_id.as_u16(),
                        name: peer.name.clone(),
                        job: 0,
                        pos_x: peer.position.x,
                        pos_y: peer.position.y,
                        dir: peer.facing as u8,
                        speed: peer.speed,
                    };
                    let _ = tx.send(peer_spawn);
                }
            }
        }

        // 5. Salva a nova sessão
        inner.sessions.insert(
            entity_id,
            WorldSession {
                entity_id,
                aid,
                gid,
                name: name.clone(),
                position: initial_pos,
                facing: Direction::South,
                speed,
                tx: tx.clone(),
            },
        );

        info!(
            "Jogador registrado no mundo: AID={aid}, GID={gid}, EntityId={}, Pos=({}, {})",
            entity_id.as_u16(),
            initial_pos.x,
            initial_pos.y
        );

        (entity_id, welcome_msg)
    }

    /// Desregistra o jogador do mundo e notifica o AoI.
    pub async fn unregister_player(&self, entity_id: EntityId) {
        let mut inner = self.inner.write().await;
        if let Some(session) = inner.sessions.remove(&entity_id) {
            inner.spatial_grid.remove_entity(entity_id, true);

            // Multicast de despawn para o AoI 3x3
            let mut aoi_entities = Vec::with_capacity(16);
            inner
                .spatial_grid
                .query_aoi(session.position, &mut aoi_entities);

            let despawn_msg = ServerWorldMsg::EntityDespawn {
                id: entity_id.as_u16(),
            };
            for peer_id in aoi_entities {
                if let Some(peer) = inner.sessions.get(&peer_id) {
                    let _ = peer.tx.send(despawn_msg.clone());
                }
            }

            info!(
                "Jogador desconectado do mundo: EntityId={}, Nome={}",
                entity_id.as_u16(),
                session.name
            );
        }
    }

    /// Processa uma mensagem vinda do cliente.
    pub async fn handle_client_message(&self, entity_id: EntityId, msg: ClientWorldMsg) {
        match msg {
            ClientWorldMsg::MoveRequest { to_x, to_y } => {
                self.process_move_request(entity_id, to_x, to_y).await;
            }
            ClientWorldMsg::Ping { timestamp } => {
                debug!("Ping recebido de EntityId={}, ts={timestamp}", entity_id.as_u16());
                let inner = self.inner.read().await;
                if let Some(session) = inner.sessions.get(&entity_id) {
                    let _ = session.tx.send(ServerWorldMsg::Pong { timestamp });
                }
            }
            ClientWorldMsg::EnterWorld { .. } => {
                warn!("EnterWorld repetido recebido de EntityId={}", entity_id.as_u16());
            }
        }
    }

    /// Valida e despacha requisição de movimento.
    async fn process_move_request(&self, entity_id: EntityId, to_x: u16, to_y: u16) {
        let mut inner = self.inner.write().await;

        let target_pos = Position::new_unchecked(to_x, to_y);

        // Validação de limites e colisão
        if !inner.collision_grid.is_walkable(target_pos) {
            debug!(
                "Movimento bloqueado para ({to_x}, {to_y}) por colisão para EntityId={}",
                entity_id.as_u16()
            );
            return;
        }

        let (from_pos, speed, tx) = match inner.sessions.get(&entity_id) {
            Some(s) => (s.position, s.speed, s.tx.clone()),
            None => return,
        };

        let dx = (to_x as i32 - from_pos.x as i32).abs();
        let dy = (to_y as i32 - from_pos.y as i32).abs();
        let steps = dx.max(dy) as u64;

        if steps == 0 {
            return;
        }

        // Calcula tempos de animação clássica (~150ms por célula)
        let start_time = Self::now_ms();
        let duration_ms = steps * (speed as u64);
        let end_time = start_time + duration_ms;

        // Atualiza posição da sessão
        if let Some(session) = inner.sessions.get_mut(&entity_id) {
            session.position = target_pos;
        }

        // Atualiza particionamento espacial (migração de bucket no AoI se mudou)
        inner
            .spatial_grid
            .update_entity_position(entity_id, target_pos, true);

        // Mensagem de confirmação para o próprio jogador (NOTIFY_PLAYERMOVE)
        let player_move_msg = ServerWorldMsg::PlayerMove {
            from_x: from_pos.x,
            from_y: from_pos.y,
            to_x,
            to_y,
            start_time,
            end_time,
        };
        let _ = tx.send(player_move_msg);

        // Multicast de movimento para vizinhos no AoI 3x3 (NOTIFY_MOVE)
        let mut aoi_entities = Vec::with_capacity(16);
        inner.spatial_grid.query_aoi(target_pos, &mut aoi_entities);

        let entity_move_msg = ServerWorldMsg::EntityMove {
            id: entity_id.as_u16(),
            from_x: from_pos.x,
            from_y: from_pos.y,
            to_x,
            to_y,
            start_time,
            end_time,
        };

        for peer_id in aoi_entities {
            if peer_id != entity_id {
                if let Some(peer) = inner.sessions.get(&peer_id) {
                    let _ = peer.tx.send(entity_move_msg.clone());
                }
            }
        }

        debug!(
            "Movimento processado: EntityId={} ({}, {}) -> ({}, {}), duracao={}ms",
            entity_id.as_u16(),
            from_pos.x,
            from_pos.y,
            to_x,
            to_y,
            duration_ms
        );
    }

    /// Retorna contagem de sessões ativas.
    pub async fn session_count(&self) -> usize {
        let inner = self.inner.read().await;
        inner.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_player_registration_and_movement_aoi() {
        let col = CollisionGrid::new(256, 256, true);
        let mgr = WorldManager::new(col);

        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();

        // Registra Jogador A em (50, 50)
        let (id_a, welcome_a) = mgr
            .register_player(
                1,
                1,
                "Berenice".to_string(),
                Position::new_unchecked(50, 50),
                tx_a,
            )
            .await;
        assert_eq!(id_a.as_u16(), 100);
        assert!(matches!(welcome_a, ServerWorldMsg::EnterWorldOk { .. }));

        // Registra Jogador B em (52, 50) (mesmo AoI)
        let (id_b, _welcome_b) = mgr
            .register_player(
                2,
                2,
                "OutroPlayer".to_string(),
                Position::new_unchecked(52, 50),
                tx_b,
            )
            .await;
        assert_eq!(id_b.as_u16(), 101);

        // Jogador A recebe EnterWorldOk
        let a_first = rx_a.recv().await.expect("welcome para A");
        assert!(matches!(a_first, ServerWorldMsg::EnterWorldOk { .. }));

        // Jogador B recebe EnterWorldOk
        let b_first = rx_b.recv().await.expect("welcome para B");
        assert!(matches!(b_first, ServerWorldMsg::EnterWorldOk { .. }));

        // Jogador A deve ter recebido EntitySpawn do Jogador B!
        let msg_for_a = rx_a.recv().await.expect("mensagem para A");
        assert!(
            matches!(msg_for_a, ServerWorldMsg::EntitySpawn { id, .. } if id == id_b.as_u16())
        );

        // Jogador B deve ter recebido EntitySpawn do Jogador A!
        let msg_for_b = rx_b.recv().await.expect("mensagem para B");
        assert!(
            matches!(msg_for_b, ServerWorldMsg::EntitySpawn { id, .. } if id == id_a.as_u16())
        );

        // Jogador A se move para (55, 50)
        mgr.handle_client_message(id_a, ClientWorldMsg::MoveRequest { to_x: 55, to_y: 50 })
            .await;

        // Jogador A recebe PlayerMove
        let a_move = rx_a.recv().await.expect("PlayerMove para A");
        assert!(matches!(a_move, ServerWorldMsg::PlayerMove { to_x: 55, to_y: 50, .. }));

        // Jogador B recebe EntityMove do Jogador A!
        let b_move = rx_b.recv().await.expect("EntityMove para B");
        assert!(matches!(
            b_move,
            ServerWorldMsg::EntityMove { id, to_x: 55, to_y: 50, .. } if id == id_a.as_u16()
        ));
    }
}
