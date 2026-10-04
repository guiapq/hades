//! SPEC-0011: Gestão de sessões mundiais e sincronização AoI em tempo real sobre QUIC TLS 1.3.

use bytes::Bytes;
use hades_core::aoi::SpatialGrid;
use hades_core::bitpacking::MovementDelta;
use hades_core::collision::CollisionGrid;
use hades_core::types::{Direction, EntityId, Position};
use quinn::Connection;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Sessão ativa de um jogador conectado ao mundo.
pub struct PlayerSession {
    pub entity_id: EntityId,
    pub username: String,
    pub position: Position,
    pub facing: Direction,
    pub connection: Connection,
}

/// Estado interno do mundo com grid espacial AoI e sessões ativas.
struct WorldState {
    spatial_grid: SpatialGrid,
    collision_grid: Option<CollisionGrid>,
    sessions: HashMap<EntityId, PlayerSession>,
}

/// Gerenciador de mundo multi-sessão e multicast de Área de Interesse (AoI).
#[derive(Clone)]
pub struct WorldSessionManager {
    next_entity_id: Arc<AtomicU16>,
    state: Arc<RwLock<WorldState>>,
}

impl WorldSessionManager {
    /// Cria uma nova instância do gerenciador de mundo para as dimensões de mapa fornecidas.
    pub fn new(width_tiles: u16, height_tiles: u16, collision_grid: Option<CollisionGrid>) -> Self {
        let spatial_grid = SpatialGrid::new(width_tiles, height_tiles);
        Self {
            next_entity_id: Arc::new(AtomicU16::new(100)),
            state: Arc::new(RwLock::new(WorldState {
                spatial_grid,
                collision_grid,
                sessions: HashMap::new(),
            })),
        }
    }

    /// Registra uma nova sessão de jogador, atribuindo um EntityId atômico e inserindo no AoI.
    pub async fn register_player(
        &self,
        connection: Connection,
        username: String,
        initial_pos: Position,
    ) -> EntityId {
        let raw_id = self.next_entity_id.fetch_add(1, Ordering::Relaxed);
        let entity_id = EntityId::new(raw_id);

        let mut state = self.state.write().await;
        state
            .spatial_grid
            .insert_entity(entity_id, initial_pos, true);

        // Despacha notificação inicial de spawn para outros jogadores no raio AoI 3x3
        let mut aoi_entities = Vec::with_capacity(16);
        state.spatial_grid.query_aoi(initial_pos, &mut aoi_entities);

        let spawn_delta =
            MovementDelta::new(entity_id, initial_pos, Direction::South, 0x00).encode();
        let payload = Bytes::copy_from_slice(&spawn_delta);

        for &peer_id in &aoi_entities {
            if peer_id != entity_id {
                if let Some(peer_session) = state.sessions.get(&peer_id) {
                    let _ = peer_session.connection.send_datagram(payload.clone());
                }
            }
        }

        // Insere a nova sessão
        state.sessions.insert(
            entity_id,
            PlayerSession {
                entity_id,
                username,
                position: initial_pos,
                facing: Direction::South,
                connection,
            },
        );

        entity_id
    }

    /// Remove a sessão do jogador desconectado e limpa do particionamento espacial.
    pub async fn unregister_player(&self, entity_id: EntityId) {
        let mut state = self.state.write().await;
        state.spatial_grid.remove_entity(entity_id, true);
        state.sessions.remove(&entity_id);
    }

    /// Processa um delta de movimento recebido de um cliente:
    /// 1. Valida colisão e limites do mapa.
    /// 2. Atualiza posição no SpatialGrid.
    /// 3. Faz multicast do pacote de 6 bytes estritamente para conexões dentro do AoI 3x3.
    pub async fn process_movement_delta(&self, source_id: EntityId, delta: &MovementDelta) -> bool {
        let mut state = self.state.write().await;

        // Validação de colisão se houver CollisionGrid
        if let Some(ref col) = state.collision_grid {
            if !col.is_walkable(delta.position) {
                return false;
            }
        }

        // Atualiza a posição da sessão
        if let Some(session) = state.sessions.get_mut(&source_id) {
            session.position = delta.position;
            session.facing = delta.direction;
        } else {
            return false;
        }

        // Atualiza SpatialGrid (migração entre buckets se necessário)
        state
            .spatial_grid
            .update_entity_position(source_id, delta.position, true);

        // Consulta AoI 3x3
        let mut aoi_entities = Vec::with_capacity(16);
        state
            .spatial_grid
            .query_aoi(delta.position, &mut aoi_entities);

        // Despacho multicast O(1) de datagrama de 6 bytes
        let encoded = delta.encode();
        let payload = Bytes::copy_from_slice(&encoded);

        for &peer_id in &aoi_entities {
            if peer_id != source_id {
                if let Some(peer_session) = state.sessions.get(&peer_id) {
                    let _ = peer_session.connection.send_datagram(payload.clone());
                }
            }
        }

        true
    }

    /// Retorna a lista de entidades atualmente presentes no raio AoI 3x3 de uma dada posição.
    pub async fn get_aoi_entities(&self, pos: Position) -> Vec<(EntityId, Position, Direction)> {
        let state = self.state.read().await;
        let mut aoi_entities = Vec::with_capacity(16);
        state.spatial_grid.query_aoi(pos, &mut aoi_entities);

        let mut out = Vec::with_capacity(aoi_entities.len());
        for id in aoi_entities {
            if let Some(session) = state.sessions.get(&id) {
                out.push((session.entity_id, session.position, session.facing));
            }
        }
        out
    }

    /// Retorna a contagem atual de jogadores conectados.
    pub async fn session_count(&self) -> usize {
        let state = self.state.read().await;
        state.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::connect_to_server;
    use crate::server::HadesServer;

    #[tokio::test]
    async fn test_multi_session_aoi_movement_sync() {
        let server = HadesServer::bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let server_addr = server.local_addr().unwrap();
        let cert = server.cert_der().clone();

        let world_mgr = WorldSessionManager::new(256, 256, None);
        let world_mgr_clone = world_mgr.clone();

        // Loop receptor de conexões no servidor
        tokio::spawn(async move {
            while let Some(conn) = server.accept().await {
                let mgr = world_mgr_clone.clone();
                tokio::spawn(async move {
                    // Posição inicial próxima para teste
                    let pid = mgr
                        .register_player(
                            conn.clone(),
                            "test_user".to_string(),
                            Position::new_unchecked(50, 50),
                        )
                        .await;

                    while let Ok(dgram) = conn.read_datagram().await {
                        if dgram.len() >= MovementDelta::PACKET_SIZE {
                            let mut buf = [0u8; MovementDelta::PACKET_SIZE];
                            buf.copy_from_slice(&dgram[..MovementDelta::PACKET_SIZE]);
                            if let Ok(delta) = MovementDelta::decode(&buf) {
                                mgr.process_movement_delta(pid, &delta).await;
                            }
                        }
                    }
                    mgr.unregister_player(pid).await;
                });
            }
        });

        // Conecta Cliente A e Cliente B
        let client_a = connect_to_server(server_addr, "localhost", cert.clone())
            .await
            .expect("Client A connect");
        let client_b = connect_to_server(server_addr, "localhost", cert.clone())
            .await
            .expect("Client B connect");

        // Dá tempo para o handshake e registro
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        assert_eq!(world_mgr.session_count().await, 2);

        // Cliente A envia movimento (Posição 51, 50)
        let delta_a = MovementDelta::new(
            EntityId::new(100),
            Position::new_unchecked(51, 50),
            Direction::East,
            0x01,
        );
        client_a
            .send_datagram(Bytes::copy_from_slice(&delta_a.encode()))
            .expect("send datagram");

        // Cliente B deve receber o datagrama do Cliente A porque estão no mesmo AoI 3x3!
        let recv_b = tokio::time::timeout(
            tokio::time::Duration::from_millis(200),
            client_b.read_datagram(),
        )
        .await
        .expect("timeout waiting for datagram")
        .expect("read datagram");

        let mut buf = [0u8; 6];
        buf.copy_from_slice(&recv_b[..6]);
        let decoded = MovementDelta::decode(&buf).expect("decode");

        assert_eq!(decoded.position, Position::new_unchecked(51, 50));
        assert_eq!(decoded.direction, Direction::East);
    }
}
