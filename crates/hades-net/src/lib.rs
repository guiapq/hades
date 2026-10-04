//! # Hades Network Layer (WebTransport & QUIC)
//!
//! Camada de rede moderna do Hades sobre HTTP/3 / QUIC (UDP), provendo
//! multiplexação de canais confiáveis e datagramas ultrarrápidos para o MovementDelta (6B).

pub mod client;
pub mod protocol;
pub mod server;
pub mod tls;
pub mod world;

pub use client::{connect_to_server, ClientError};
pub use protocol::{
    decode_movement_datagram, encode_movement_datagram, ProtocolError, ReliablePacket,
};
pub use server::{HadesServer, ServerError};
pub use tls::{create_client_config, create_server_config, generate_self_signed_cert, TlsError};
pub use world::{PlayerSession, WorldSessionManager};

#[cfg(test)]
mod tests {
    use super::*;
    use hades_core::{Direction, EntityId, MovementDelta, Position};
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_quic_server_bind_and_local_addr() {
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = HadesServer::bind(bind_addr).expect("Servidor deve vincular porta efêmera");
        let local_addr = server.local_addr().expect("Deve obter endereço local");
        assert_ne!(local_addr.port(), 0);
    }

    #[tokio::test]
    async fn test_quic_loopback_connection_and_datagram_exchange() {
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = HadesServer::bind(bind_addr).expect("Falha ao subir servidor");
        let server_addr = server.local_addr().unwrap();
        let cert_der = server.cert_der().clone();

        // 1. Spawna task do servidor aguardando conexão e recebendo datagrama
        let server_task = tokio::spawn(async move {
            let conn = server
                .accept()
                .await
                .expect("Servidor deve aceitar conexão");
            // Lê datagrama de 6 bytes recebido
            let dgram = conn
                .read_datagram()
                .await
                .expect("Servidor deve ler datagrama");
            decode_movement_datagram(&dgram).expect("Deve decodificar MovementDelta")
        });

        // 2. Cliente conecta via loopback
        let client_conn = connect_to_server(server_addr, "localhost", cert_der)
            .await
            .expect("Cliente deve conectar ao servidor QUIC");

        // 3. Cliente envia MovementDelta de 6 bytes via Datagrama não-bloqueante
        let test_delta = MovementDelta::new(
            EntityId::new(777),
            Position::new_unchecked(1000, 2000),
            Direction::NorthWest,
            15,
        );
        let bytes = encode_movement_datagram(&test_delta);
        client_conn
            .send_datagram(bytes.to_vec().into())
            .expect("Cliente deve enviar datagrama com sucesso");

        // 4. Valida se o servidor recebeu o delta idêntico
        let received_delta = server_task.await.expect("Server task completou");
        assert_eq!(received_delta, test_delta);
    }

    #[tokio::test]
    async fn test_quic_reliable_stream_exchange() {
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = HadesServer::bind(bind_addr).unwrap();
        let server_addr = server.local_addr().unwrap();
        let cert_der = server.cert_der().clone();

        let (done_tx, done_rx) = tokio::sync::oneshot::channel();

        // Servidor aceita stream confiável, lê mensagem de chat e responde
        let server_task = tokio::spawn(async move {
            let conn = server.accept().await.unwrap();
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();

            // Lê pacote confiável do cliente
            let mut buf = vec![0u8; 1024];
            let n = recv.read(&mut buf).await.unwrap().unwrap();
            let packet = ReliablePacket::decode(&buf[..n]).unwrap();

            // Responde com confirmação
            let response = ReliablePacket::AuthResponse {
                success: true,
                char_id: 150001,
            };
            send.write_all(&response.encode()).await.unwrap();
            send.finish().unwrap();

            // Mantém a conexão ativa até o cliente concluir a leitura
            let _ = done_rx.await;
            packet
        });

        // Cliente conecta e abre stream bidirecional
        let client_conn = connect_to_server(server_addr, "localhost", cert_der)
            .await
            .unwrap();
        let (mut send, mut recv) = client_conn.open_bi().await.unwrap();

        let auth_req = ReliablePacket::AuthRequest {
            account_id: 1000,
            token: "valid_session".to_string(),
        };
        send.write_all(&auth_req.encode()).await.unwrap();
        send.finish().unwrap();

        let mut resp_buf = vec![0u8; 1024];
        let n = recv.read(&mut resp_buf).await.unwrap().unwrap();
        let resp_packet = ReliablePacket::decode(&resp_buf[..n]).unwrap();

        // Notifica o servidor que a leitura foi concluída
        let _ = done_tx.send(());

        assert_eq!(
            resp_packet,
            ReliablePacket::AuthResponse {
                success: true,
                char_id: 150001
            }
        );

        let server_received = server_task.await.unwrap();
        assert_eq!(server_received, auth_req);
    }
}
