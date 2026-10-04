//! Geração de certificados TLS 1.3 em memória para QUIC/WebTransport.

use quinn::crypto::rustls::QuicServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("Erro gerando certificado autoassinado: {0}")]
    CertificateGeneration(#[from] rcgen::Error),

    #[error("Erro configurando TLS do servidor: {0}")]
    ServerConfig(#[from] rustls::Error),
}

/// Gera um par de chaves e certificado X.509 autoassinado para desenvolvimento local em memória.
pub fn generate_self_signed_cert(
) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), TlsError> {
    let mut subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];

    if let Ok(extra_sans) = std::env::var("HADES_TLS_SAN") {
        for s in extra_sans.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if !subject_alt_names.contains(&s.to_string()) {
                subject_alt_names.push(s.to_string());
            }
        }
    }
    if let Ok(server_host) = std::env::var("HADES_SERVER_HOST") {
        let clean = server_host.trim();
        if !clean.is_empty() && !subject_alt_names.contains(&clean.to_string()) {
            subject_alt_names.push(clean.to_string());
        }
    }

    let certified_key = rcgen::generate_simple_self_signed(subject_alt_names)?;

    let cert_der = certified_key.cert.der().to_owned();
    let key_der = PrivateKeyDer::Pkcs8(certified_key.signing_key.serialize_der().into());

    Ok((cert_der, key_der))
}

/// Constrói a configuração padrão do servidor QUIC com TLS 1.3 e datagramas ativados.
pub fn create_server_config(
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
) -> Result<quinn::ServerConfig, TlsError> {
    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)?;

    server_crypto.alpn_protocols = vec![b"hades-ro".to_vec(), b"h3".to_vec()];

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
        QuicServerConfig::try_from(server_crypto)
            .map_err(|e| rustls::Error::General(format!("QuicServerConfig error: {e:?}")))?,
    ));

    // Ativa datagramas não-confiáveis no nível do transporte QUIC
    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(std::time::Duration::from_secs(30).try_into().unwrap()));
    transport.datagram_receive_buffer_size(Some(1024 * 1024)); // 1MB buffer de datagramas

    server_config.transport_config(Arc::new(transport));

    Ok(server_config)
}

/// Constrói a configuração de cliente para testes em loopback que aceita o certificado autoassinado.
pub fn create_client_config(
    cert: CertificateDer<'static>,
) -> Result<quinn::ClientConfig, TlsError> {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(cert)
        .map_err(|e| rustls::Error::General(format!("Failed to add cert: {e}")))?;

    let mut client_crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    client_crypto.alpn_protocols = vec![b"hades-ro".to_vec(), b"h3".to_vec()];

    let mut client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
            .map_err(|e| rustls::Error::General(format!("QuicClientConfig error: {e:?}")))?,
    ));

    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(std::time::Duration::from_secs(30).try_into().unwrap()));
    client_config.transport_config(Arc::new(transport));

    Ok(client_config)
}

#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Constrói configuração de cliente QUIC para desenvolvimento local que ignora verificação de CA.
pub fn create_insecure_client_config() -> Result<quinn::ClientConfig, TlsError> {
    let mut client_crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    client_crypto.alpn_protocols = vec![b"hades-ro".to_vec(), b"h3".to_vec()];

    let mut client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)
            .map_err(|e| rustls::Error::General(format!("QuicClientConfig error: {e:?}")))?,
    ));

    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(std::time::Duration::from_secs(30).try_into().unwrap()));
    client_config.transport_config(Arc::new(transport));

    Ok(client_config)
}

