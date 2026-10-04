//! Cliente de rede QUIC para testes e integração com clientes (Godot/Unity/Bevy).

use crate::tls::{create_client_config, create_insecure_client_config, TlsError};
use rustls::pki_types::CertificateDer;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Erro de I/O na rede: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de configuração TLS: {0}")]
    Tls(#[from] TlsError),

    #[error("Erro de conexão Quinn: {0}")]
    Connect(#[from] quinn::ConnectError),

    #[error("Erro na conexão QUIC estabelecida: {0}")]
    Connection(#[from] quinn::ConnectionError),
}

/// Conecta a um servidor Hades QUIC aceitando o certificado de desenvolvimento informado.
pub async fn connect_to_server(
    server_addr: SocketAddr,
    server_name: &str,
    server_cert: CertificateDer<'static>,
) -> Result<quinn::Connection, ClientError> {
    let client_config = create_client_config(server_cert)?;

    let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let mut endpoint = quinn::Endpoint::client(bind_addr)?;
    endpoint.set_default_client_config(client_config);

    let connection = endpoint.connect(server_addr, server_name)?.await?;
    Ok(connection)
}

/// Conecta a um servidor Hades QUIC aceitando certificados autoassinados em desenvolvimento local.
pub async fn connect_to_server_insecure(
    server_addr: SocketAddr,
    server_name: &str,
) -> Result<quinn::Connection, ClientError> {
    let client_config = create_insecure_client_config()?;

    let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let mut endpoint = quinn::Endpoint::client(bind_addr)?;
    endpoint.set_default_client_config(client_config);

    let connection = endpoint.connect(server_addr, server_name)?.await?;
    Ok(connection)
}

