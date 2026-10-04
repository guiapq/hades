//! SPEC-0009: Adaptador de rede QUIC/WebTransport para o cliente Berenice.

use bytes::Bytes;
use hades_core::bitpacking::{BitpackError, MovementDelta};
use hades_net::client::{connect_to_server, ClientError};
use hades_net::protocol::ReliablePacket;
use quinn::Connection;
use rustls::pki_types::CertificateDer;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkClientError {
    #[error("Erro de conexão com o servidor Hades: {0}")]
    Connect(#[from] ClientError),

    #[error("Falha ao despachar datagrama QUIC: {0}")]
    SendDatagram(#[from] quinn::SendDatagramError),

    #[error("Erro de I/O na stream QUIC: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro na conexão QUIC: {0}")]
    Connection(#[from] quinn::ConnectionError),

    #[error("Erro ao decodificar pacote bitpacked: {0}")]
    Bitpack(#[from] BitpackError),

    #[error("Erro de escrita na stream QUIC: {0}")]
    Write(#[from] quinn::WriteError),

    #[error("Erro de leitura na stream QUIC: {0}")]
    Read(#[from] quinn::ReadError),

    #[error("Stream QUIC fechada inesperadamente: {0}")]
    ClosedStream(#[from] quinn::ClosedStream),
}

/// Sessão de rede ativa do cliente Berenice com o servidor Hades.
#[derive(Clone)]
pub struct BereniceNetwork {
    pub connection: Connection,
}

impl BereniceNetwork {
    /// Conecta ao servidor Hades no endereço especificado.
    pub async fn connect(
        server_addr: SocketAddr,
        server_name: &str,
        cert: CertificateDer<'static>,
    ) -> Result<Self, NetworkClientError> {
        let connection = connect_to_server(server_addr, server_name, cert).await?;
        Ok(Self { connection })
    }

    /// Conecta ao servidor Hades no endereço especificado aceitando certificados locais em desenvolvimento.
    pub async fn connect_insecure(
        server_addr: SocketAddr,
        server_name: &str,
    ) -> Result<Self, NetworkClientError> {
        let connection = hades_net::client::connect_to_server_insecure(server_addr, server_name).await?;
        Ok(Self { connection })
    }

    /// Abre uma stream bidirecional dedicada para o protocolo do World Server.
    pub async fn open_world_client(&self) -> Result<WorldClient, NetworkClientError> {
        WorldClient::new(&self.connection).await
    }

    /// Envia o pacote compacto de 6 bytes de movimento via Datagrama QUIC (não-bloqueante e ultra-rápido).
    pub fn send_movement(&self, delta: &MovementDelta) -> Result<(), NetworkClientError> {
        let bytes = delta.encode();
        self.connection
            .send_datagram(Bytes::copy_from_slice(&bytes))?;
        Ok(())
    }

    /// Tenta ler um datagrama de movimento de 6 bytes recebido de outra entidade no mundo.
    pub async fn receive_movement_delta(
        &self,
    ) -> Result<Option<MovementDelta>, NetworkClientError> {
        match self.connection.read_datagram().await {
            Ok(bytes) => {
                if bytes.len() >= MovementDelta::PACKET_SIZE {
                    let mut slice = [0u8; MovementDelta::PACKET_SIZE];
                    slice.copy_from_slice(&bytes[..MovementDelta::PACKET_SIZE]);
                    Ok(Some(MovementDelta::decode(&slice)?))
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(NetworkClientError::Connection(e)),
        }
    }

    /// Envia uma ação confiável (habilidade, chat, ping) abrindo uma stream multiplexada.
    pub async fn send_action_packet(
        &self,
        packet: &ReliablePacket,
    ) -> Result<(), NetworkClientError> {
        let mut send_stream = self.connection.open_uni().await?;
        let payload = packet.encode();
        send_stream.write_all(&payload).await?;
        send_stream.finish()?;
        Ok(())
    }

    /// Retorna o tempo de ida e volta (RTT / Ping) estimado da conexão QUIC em milissegundos.
    pub fn rtt_ms(&self) -> u64 {
        self.connection.rtt().as_millis() as u64
    }
}

/// Sessão ativa de stream bidirecional com o Hades World Server (SPEC-0016).
pub struct WorldClient {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    read_buf: Vec<u8>,
}

impl WorldClient {
    pub async fn new(connection: &Connection) -> Result<Self, NetworkClientError> {
        let (send, recv) = connection.open_bi().await?;
        Ok(Self {
            send,
            recv,
            read_buf: Vec::with_capacity(4096),
        })
    }

    /// Envia uma mensagem do cliente para o World Server.
    pub async fn send_msg(&mut self, msg: &hades_world::ClientWorldMsg) -> Result<(), NetworkClientError> {
        let mut json = serde_json::to_vec(msg)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        json.push(b'\n');
        self.send.write_all(&json).await?;
        Ok(())
    }

    /// Tenta ler a próxima mensagem despachada pelo World Server.
    pub async fn read_msg(&mut self) -> Result<Option<hades_world::ServerWorldMsg>, NetworkClientError> {
        loop {
            if let Some(pos) = self.read_buf.iter().position(|&b| b == b'\n') {
                let line_bytes = self.read_buf.drain(..=pos).collect::<Vec<_>>();
                let line = std::str::from_utf8(&line_bytes[..line_bytes.len() - 1])
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let msg: hades_world::ServerWorldMsg = serde_json::from_str(trimmed)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                return Ok(Some(msg));
            }

            let mut tmp = [0u8; 2048];
            match self.recv.read(&mut tmp).await? {
                Some(n) => {
                    self.read_buf.extend_from_slice(&tmp[..n]);
                }
                None => return Ok(None),
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use hades_core::types::{Direction, EntityId, Position};
    use hades_net::server::HadesServer;

    #[tokio::test]
    async fn test_berenice_network_datagram_exchange() {
        let server = HadesServer::bind("127.0.0.1:0".parse().unwrap()).expect("Server bind failed");
        let server_addr = server.local_addr().unwrap();
        let client_cert = server.cert_der().clone();

        // 1. Spawna task do servidor aguardando conexão concorrentemente
        let server_task = tokio::spawn(async move {
            let server_conn = server.accept().await.expect("Server accept failed");
            server_conn
                .read_datagram()
                .await
                .expect("Server read datagram failed")
        });

        // 2. Conecta o cliente Berenice
        let client = BereniceNetwork::connect(server_addr, "localhost", client_cert)
            .await
            .expect("Berenice connect failed");

        // 3. Envia um MovementDelta
        let delta = MovementDelta::new(
            EntityId::new(100),
            Position::new_unchecked(50, 75),
            Direction::NorthEast,
            0x01,
        );

        client
            .send_movement(&delta)
            .expect("Failed to send movement datagram");

        // 4. Recebe o datagrama no servidor
        let received_bytes = server_task.await.unwrap();
        assert_eq!(received_bytes.len(), 6);

        let mut slice = [0u8; 6];
        slice.copy_from_slice(&received_bytes);
        let decoded = MovementDelta::decode(&slice).expect("Failed to decode MovementDelta");

        assert_eq!(decoded.entity_id, EntityId::new(100));
        assert_eq!(decoded.position, Position::new_unchecked(50, 75));
        assert_eq!(decoded.direction, Direction::NorthEast);
        assert_eq!(decoded.action_flags, 0x01);
    }

    #[tokio::test]
    async fn test_berenice_world_session_handshake() {
        use hades_core::collision::CollisionGrid;
        use hades_world::{ClientWorldMsg, ServerWorldMsg, WorldManager, WorldSessionHandler};
        use std::sync::Arc;

        let server = HadesServer::bind("127.0.0.1:0".parse().unwrap()).expect("Server bind failed");
        let server_addr = server.local_addr().unwrap();

        let grid = CollisionGrid::new(100, 100, true);
        let world = Arc::new(WorldManager::new(grid));
        let spawn_pos = Position::new_unchecked(50, 50);
        let session_handler = Arc::new(WorldSessionHandler::new(world, spawn_pos));

        let s_handler = session_handler.clone();
        tokio::spawn(async move {
            if let Some(conn) = server.accept().await {
                s_handler.run_quic(conn).await;
            }
        });

        // Conecta cliente Berenice via TLS 1.3
        let client = BereniceNetwork::connect_insecure(server_addr, "localhost")
            .await
            .expect("Connect insecure failed");

        let mut world_client = client
            .open_world_client()
            .await
            .expect("Open world client failed");

        // Envia handshake EnterWorld
        world_client
            .send_msg(&ClientWorldMsg::EnterWorld {
                aid: 1,
                gid: 1,
                auth_code: 999,
                name: Some("Berenice".into()),
            })
            .await
            .expect("Send EnterWorld failed");

        // Recebe EnterWorldOk
        let response = world_client
            .read_msg()
            .await
            .expect("Read response failed")
            .expect("Expected ServerWorldMsg");

        match response {
            ServerWorldMsg::EnterWorldOk {
                name,
                pos_x,
                pos_y,
                map_name,
                ..
            } => {
                assert_eq!(name, "Berenice");
                assert_eq!(pos_x, 50);
                assert_eq!(pos_y, 50);
                assert_eq!(map_name, "prontera.gat");
            }
            other => panic!("Mensagem inesperada do servidor: {other:?}"),
        }
    }
}

