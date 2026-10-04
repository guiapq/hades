//! Hades Login Server: servidor de autenticação e seleção de personagens via WebTransport/QUIC + JSON.

use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use hades_net::HadesServer;
use hades_login::db::Db;
use hades_login::session::{LoginSession, WorldServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    // 1. Inicializa logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,hades_login=debug,hades_net=info")),
        )
        .init();

    info!("Iniciando Hades Login Server...");

    // 2. Configurações via variáveis de ambiente
    let bind_str = std::env::var("HADES_BIND").unwrap_or_else(|_| "0.0.0.0:4433".to_string());
    let bind_addr: SocketAddr = bind_str.parse().expect("HADES_BIND inválido");

    let db_url = std::env::var("HADES_DB_URL")
        .unwrap_or_else(|_| "sqlite://hades_login.db?mode=rwc".to_string());

    let world_ip = std::env::var("HADES_WORLD_IP")
        .or_else(|_| std::env::var("HADES_SERVER_HOST"))
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let world_port: u16 = std::env::var("HADES_WORLD_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4434);
    let world_name = std::env::var("HADES_WORLD_NAME").unwrap_or_else(|_| "Hades Midgard".to_string());

    // 3. Conecta e migra banco de dados
    info!("Conectando ao banco SQLite: {db_url}");
    let db = Db::open(&db_url).await.expect("Falha ao abrir banco de dados SQLite");
    db.migrate().await.expect("Falha ao executar migrações do banco");

    // 4. Cria conta padrão de desenvolvimento se configurada/não existir
    let dev_user = std::env::var("HADES_DEV_ACCOUNT_USER").unwrap_or_else(|_| "admin".to_string());
    let dev_pass = std::env::var("HADES_DEV_ACCOUNT_PASSWORD").unwrap_or_else(|_| "admin".to_string());
    let dev_pass_hash = std::env::var("HADES_DEV_ACCOUNT_PASS_HASH").unwrap_or_else(|_| {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(dev_pass.as_bytes());
        format!("{:x}", hasher.finalize())
    });
    let dev_char_name = std::env::var("HADES_DEV_CHAR_NAME").unwrap_or_else(|_| "Berenice".to_string());

    let admin_aid = db.create_account_dev(&dev_user, &dev_pass_hash, 0).await?;
    info!("Conta dev pronta: {dev_user} (AID: {admin_aid})");

    // Se a conta de desenvolvimento não tem personagem, cria um inicial de teste
    let chars = db.list_chars(admin_aid).await?;
    if chars.is_empty() {
        let created = db.create_char(
            admin_aid,
            &dev_char_name,
            1, 1, 1, 1, 1, 1, // soma = 6
            1, 0, 0,
        ).await;
        match created {
            Ok(c) => info!("Personagem dev inicial criado: {} (GID: {})", c.name, c.gid),
            Err(e) => info!("Nota sobre char dev: {e}"),
        }
    }

    let db = Arc::new(db);
    let session = Arc::new(LoginSession::new(
        db,
        WorldServerConfig {
            ip: world_ip,
            port: world_port,
            name: world_name,
        },
    ));

    // 5. Inicializa listener WebSocket transparente (para navegador/desenvolvimento)
    let ws_bind_str = std::env::var("HADES_WS_BIND").unwrap_or_else(|_| "0.0.0.0:4432".to_string());
    let ws_bind_addr: SocketAddr = ws_bind_str.parse().expect("HADES_WS_BIND inválido");
    let ws_listener = tokio::net::TcpListener::bind(ws_bind_addr).await.expect("Falha ao abrir socket TCP WebSocket");
    info!("Hades Login WebSocket Server rodando em {ws_bind_addr} (ws://)");

    let ws_session = session.clone();
    tokio::spawn(async move {
        while let Ok((stream, peer)) = ws_listener.accept().await {
            let s = ws_session.clone();
            tokio::spawn(async move {
                match tokio_tungstenite::accept_async(stream).await {
                    Ok(ws) => s.run_ws(ws, peer).await,
                    Err(e) => tracing::warn!("Falha no handshake WebSocket de {peer}: {e}"),
                }
            });
        }
    });

    // 6. Inicializa servidor QUIC/WebTransport
    info!("Vinculando socket QUIC em {bind_addr}...");
    let server = HadesServer::bind(bind_addr).expect("Falha ao subir HadesServer");
    info!("Hades Login Server rodando em {bind_addr} (QUIC/WebTransport)");

    // 7. Loop de aceitação de conexões QUIC
    while let Some(conn) = server.accept().await {
        let session = session.clone();
        tokio::spawn(async move {
            session.run(conn).await;
        });
    }

    Ok(())
}
