//! Servidor de transporte QUIC / WebTransport do Hades.

use crate::tls::{create_server_config, generate_self_signed_cert, TlsError};
use rustls::pki_types::CertificateDer;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Erro de I/O na rede: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de configuração TLS: {0}")]
    Tls(#[from] TlsError),

    #[error("Erro no endpoint QUIC: {0}")]
    Quic(#[from] quinn::ConnectionError),
}

/// Instância do servidor de rede gerenciando o endpoint UDP e conexões QUIC.
pub struct HadesServer {
    endpoint: quinn::Endpoint,
    cert_der: CertificateDer<'static>,
}

impl HadesServer {
    /// Inicializa o servidor gerando um certificado autoassinado para testes locais.
    pub fn bind(addr: SocketAddr) -> Result<Self, ServerError> {
        let (cert, key) = generate_self_signed_cert()?;
        let server_config = create_server_config(cert.clone(), key)?;

        let endpoint = quinn::Endpoint::server(server_config, addr)?;

        Ok(Self {
            endpoint,
            cert_der: cert,
        })
    }

    /// Retorna o endereço local (IP:Porta) vinculado ao socket UDP.
    pub fn local_addr(&self) -> Result<SocketAddr, ServerError> {
        Ok(self.endpoint.local_addr()?)
    }

    /// Retorna uma referência ao certificado TLS utilizado pelo servidor.
    pub fn cert_der(&self) -> &CertificateDer<'static> {
        &self.cert_der
    }

    /// Retorna uma referência ao endpoint Quinn subjacente.
    pub fn endpoint(&self) -> &quinn::Endpoint {
        &self.endpoint
    }

    /// Aguarda a próxima conexão de entrada.
    pub async fn accept(&self) -> Option<quinn::Connection> {
        loop {
            let incoming = self.endpoint.accept().await?;
            match incoming.await {
                Ok(conn) => return Some(conn),
                Err(e) => {
                    eprintln!("[HadesServer] Handshake falhou ou cancelado pelo cliente: {e}");
                    continue;
                }
            }
        }
    }
}
